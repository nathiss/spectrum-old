use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use futures::Future;
use tokio::task::JoinHandle;

/// This is a helper function used to calculate hash of the given object.
///
/// # Arguments
///
/// * `t` - Any object that implements `Hash` trait.
///
/// # Returns
///
/// A `u64` representation of the calculated hash is returned.
pub(crate) fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut hasher = DefaultHasher::new();

    t.hash(&mut hasher);

    hasher.finish()
}

/// This is a helper function used to convert a `JoinHandle<()>` into a `impl Future<Output = ()>`.
///
/// # Arguments
///
/// * `handle` - A handle to another fiber of execution.
///
/// # Returns
///
/// A `impl Future<Output = ()>` that wraps around `handle` is returned.
pub(crate) fn convert_to_future(handle: JoinHandle<()>) -> impl Future<Output = ()> {
    async move { handle.await.unwrap() }
}
