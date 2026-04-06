#![warn(missing_docs)]

//! # luxpool
//!
//! `luxpool` is a lightweight, zero-dependency concurrency library for Rust.
//! It provides two main primitives for parallel data processing:
//!
//! 1. [`ThreadPool`]: A pool of worker threads that wait for closures to execute.
//! 2. [`DynamicQueue`]: A thread-safe data queue for micro-batching and dynamic load balancing.
//!
//! By combining these two primitives, you can efficiently process large datasets
//! across multiple CPU cores without spawning new threads for every task.

mod pool;
mod queue;

pub use pool::*;
pub use queue::*;
