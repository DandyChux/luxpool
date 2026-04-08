use std::sync::Mutex;

/// A thread-safe dynamic queue for managing work chunks.
///
/// The `DynamicQueue` is designed to hold chunks of data (micro-batches).
/// Multiple threads can safely call [`pop`](DynamicQueue::pop) concurrently to grab
/// the next available chunk of work. This enables dynamic load balancing, ensuring
/// fast threads don't wait for slow threads to finish.
pub struct DynamicQueue<T> {
    items: Mutex<Vec<T>>,
}

impl<T> DynamicQueue<T> {
    /// Creates a new `DynamicQueue` from an existing `Vec` of data.
    ///
    /// The data is moved into a `Mutex` to allow safe, concurrent access across threads.
    pub fn new(data: Vec<T>) -> Self {
        Self {
            items: Mutex::new(data),
        }
    }

    /// Pops the next item off the queue.
    ///
    /// Safely locks the queue, removes the last item, and returns it.
    /// Returns `None` if the queue is entirely empty, signaling to the worker
    /// thread that all processing is complete.
    pub fn pop(&self) -> Option<T> {
        match self.items.lock() {
            Ok(mut queue) => queue.pop(),
            Err(poisoned) => poisoned.into_inner().pop(),
        }
    }
}
