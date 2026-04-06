# luxpool ⚡

A lightweight, zero-dependency thread pool and dynamic task queue for Rust.

`luxpool` provides a simple and educational approach to parallel data processing. It is designed to be a "Rayon-lite"—offering a global thread pool and dynamic work-stealing queues without the heavy abstractions or complex dependency trees of larger concurrency frameworks.

## Features
- **Zero Dependencies:** Compiles lightning-fast and doesn't bloat your dependency tree.
- **Thread Pool:** Spawn `N` worker threads once and reuse them for multiple tasks.
- **Dynamic Work Queue:** Safely share a queue of micro-batches across all threads so that no CPU core ever sits idle.
- **Educational:** The source code is minimal, heavily documented, and easy to read.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
luxpool = "0.1.0"
```

## Usage

Here is a basic example of combining the `ThreadPool` with a `DynamicQueue` to process data in parallel:

```rust
use std::sync::Arc;
use luxpool::{ThreadPool, DynamicQueue};

fn main() {
    // 1. Create a ThreadPool with 4 worker threads
    let pool = ThreadPool::new(4);

    // 2. Create some data chunks (micro-batches)
    let chunks = vec![
        vec![1, 2, 3],
        vec![4, 5, 6],
        vec![7, 8, 9],
        vec![10, 11, 12],
    ];

    // 3. Wrap the data in a DynamicQueue and an Arc so threads can share it safely
    let queue = Arc::new(DynamicQueue::new(chunks));

    // 4. Dispatch 4 workers to pull from the queue until it's empty
    for _ in 0..4 {
        let queue_clone = Arc::clone(&queue);
        
        pool.execute(move || {
            // Threads dynamically grab the next available chunk
            while let Some(chunk) = queue_clone.pop() {
                let sum: i32 = chunk.iter().sum();
                println!("Processed chunk sum: {}", sum);
            }
        });
    }
}
