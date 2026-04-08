use std::marker::PhantomData;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;

// A "Job" is a trait object holding a closure.
// - `FnOnce()` because the closure is executed exactly once.
// - `Send` because we are sending it from the main thread to a worker thread.
// - `'static` because the thread might live forever, so the closure can't borrow local variables.
pub(crate) type Job = Box<dyn FnOnce() + Send + 'static>;

// ==========================================
// Thread Pool
// ==========================================

/// A thread pool for executing tasks in parallel.
///
/// The `ThreadPool` spawns a specified number of worker threads upon creation.
/// These threads remain alive and wait for closures to be sent to them
///
/// You can execute independent tasks using [`ThreadPool::execute`], or use
/// [`ThreadPool::scope`] to safely borrow local variables from the surrounding environment.
pub struct ThreadPool {
    workers: Vec<Worker>,
    pub(crate) sender: mpsc::Sender<Job>,
}

impl ThreadPool {
    /// Creates a new `ThreadPool`.
    ///
    /// The `size` parameter dictates the number of worker threads that will be
    /// spawned. Usually, this should match the number of logical CPU cores on your machine.
    ///
    /// # Panics
    ///
    /// Panics if the `size` is zero.
    pub fn new(size: usize) -> Self {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();

        // Need to wrap the receiver in an Arc and Mutex so it can be shared between threads
        let shared_receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&shared_receiver)));
        }

        ThreadPool { workers, sender }
    }

    /// Submits a closure to the thread pool for execution.
    ///
    /// The closure `f` will be sent to the first available worker thread.
    /// If all threads are currently busy, the closure will wait in a queue.
    /// Because the pool lives forever, the closure cannot borrow any local variables
    /// unless they are wrapped in an `Arc`.
    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        self.sender
            .send(job)
            .expect("ThreadPool has been shut down");
    }

    /// Creates a synchronous scope allowing safe borrowing of local variables.
    ///
    /// This method allows you to spawn jobs that borrow local variables (`&T` or `&mut T`).
    /// It guarantees memory safety by **blocking the current thread** until all jobs
    /// spawned within the scope have completed.
    ///
    /// # Example
    /// ```rust
    /// let pool = ThreadPool::new(4);
    /// let mut numbers = vec![1, 2, 3];
    ///
    /// pool.scope(|s| {
    ///     s.spawn(|| {
    ///         println!("I can safely read numbers: {:?}", numbers);
    ///     });
    /// });
    /// ```
    pub fn scope<'pool, 'env, F, R>(&'pool self, f: F) -> R
    where
        F: FnOnce(&Scope<'pool, 'env>) -> R,
    {
        // Create a channel to block the main thread
        let (tx, rx) = mpsc::channel();

        // Create the scope object that holds the sender
        let scope = Scope {
            pool: self,
            tx,
            _marker: PhantomData,
        };

        // Run the user's closure, giving them the ability to call `scope.spawn(...)`
        let result = f(&scope);

        // Drop the scope. This destroys the main copy of the Sender `tx`.
        // Now, the ONLY Senders that exist are the ones cloned into the worker threads.
        drop(scope);

        // ENFORCE THE CONTRACT!
        // This blocks the main thread. It will only unblock when every single worker
        // thread has dropped its cloned Sender, severing the channel.
        let _ = rx.recv();

        // It is now 100% safe to return, because all worker threads are guaranteed
        // to be finished with the borrowed variables!
        result
    }
}

// ==========================================
// Scope
// ==========================================

/// A scope for spawning safely-borrowed tasks on a [`ThreadPool`].
///
/// This struct is created by [`ThreadPool::scope`]. It allows you to spawn tasks
/// that borrow local variables from the surrounding environment.
pub struct Scope<'pool, 'env> {
    pool: &'pool ThreadPool,
    tx: mpsc::Sender<()>,
    // PhantomData acts as a hint to the compiler that this struct is logically
    // tied to the lifetime of the borrowed local variables ('env).
    _marker: PhantomData<&'env mut &'env ()>,
}

impl<'pool, 'env> Scope<'pool, 'env> {
    /// Spawns a job into the thread pool that can borrow local variables.
    ///
    /// The closure `f` is bound to the `'env` lifetime, meaning it can safely
    /// capture references to local variables defined outside the scope.
    pub fn spawn<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'env,
    {
        let tx_clone = self.tx.clone();

        // Wrap the user's closure in a new closure that drops the Sender when finished
        let wrapper = move || {
            f();
            drop(tx_clone); // Signal that this specific job is complete
        };

        // Box the closure (it currently has the temporary 'env lifetime)
        let job: Box<dyn FnOnce() + Send + 'env> = Box::new(wrapper);

        // I know that `ThreadPool::scope` will block the main thread from exiting
        // until this closure finishes. Therefore, it's safe to lie to
        // the compiler and pretend this closure lives forever ('static).
        let static_job: Job = unsafe {
            // Extract the raw memory pointer from the Box
            let raw_ptr = Box::into_raw(job);

            // Cast the pointer to pretend the trait object has a 'static lifetime
            let static_ptr = raw_ptr as *mut (dyn FnOnce() + Send + 'static);

            // Re-box it with the fake 'static lifetime
            Box::from_raw(static_ptr)
        };

        // Send the "static" job to the thread pool
        self.pool
            .sender
            .send(static_job)
            .expect("ThreadPool panicked");
    }
}

// ==========================================
// Worker
// ==========================================

struct Worker {
    #[allow(dead_code)]
    id: usize,
    #[allow(dead_code)]
    thread: thread::JoinHandle<()>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        let thread = thread::spawn(move || {
            // Inside the loop, the worker needs to:
            // 1. Lock the mutex to get access to the receiver
            // 2. Call `.recv()` to wait for a Job
            // 3. Unlock the mutex so other threads can grab the next job
            // 4. Execute the job
            loop {
                let message = {
                    let lock = receiver.lock().unwrap();
                    lock.recv()
                };

                match message {
                    Ok(job) => {
                        job();
                    }
                    Err(_) => {
                        // The sender disconnected, meaning the ThreadPool was destroyed
                        // Time for the worker thread to exit the loop and die gracefully
                        break;
                    }
                }
            }
        });

        Worker { id, thread }
    }
}

// ==========================================
// Tests
// ==========================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thread_pool_executes() {
        let pool = ThreadPool::new(4);
        // Add test logic here to make sure it works!
    }
}
