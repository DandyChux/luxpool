use std::sync::{Arc, Mutex, mpsc};
use std::thread;

// A "Job" is a trait object holding a closure.
// - `FnOnce()` because the closure is executed exactly once.
// - `Send` because we are sending it from the main thread to a worker thread.
// - `'static` because the thread might live forever, so the closure can't borrow local variables.
pub(crate) type Job = Box<dyn FnOnce() + Send + 'static>;

/// A thread pool for executing tasks in parallel.
///
/// The `ThreadPool` spawns a specified number of worker threads upon creation.
/// These threads remain alive and wait for closures to be sent to them via the
/// [`execute`](ThreadPool::execute) method.
pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Job>,
}

impl ThreadPool {
    /// Creates a new `ThreadPool`.
    ///
    /// The `size` parameter dictates the number of worker threads that will be
    /// spawned. Usually, this should match the number of logical CPU cores on your machine.
    ///
    /// # Panics
    ///
    /// The `new` function will panic if the `size` is zero.
    pub fn new(size: usize) -> Self {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();

        // Need to wrap the receiver in an Arc and Mutex so it can be shared between threads
        let shared_receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            // TODO: instantiate your workers here
            workers.push(Worker::new(id, Arc::clone(&shared_receiver)));
        }

        ThreadPool { workers, sender }
    }

    /// Submits a closure to the thread pool for execution.
    ///
    /// The closure `f` will be sent to the first available worker thread.
    /// If all threads are currently busy, the closure will wait in a queue until
    /// a thread becomes available.
    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        // TODO: Box the closure `f` into a `Job` and send it down the channel
    }
}

// ==========================================

struct Worker {
    id: usize,
    thread: thread::JoinHandle<()>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        let thread = thread::spawn(move || {
            // TODO: Write an infinite loop here
            // Inside the loop, the worker needs to:
            // 1. Lock the mutex to get access to the receiver
            // 2. Call `.recv()` to wait for a Job
            // 3. Unlock the mutex so other threads can grab the next job
            // 4. Execute the job
        });

        Worker { id, thread }
    }
}

// ==========================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thread_pool_executes() {
        let pool = ThreadPool::new(4);
        // Add your test logic here to make sure it works!
    }
}
