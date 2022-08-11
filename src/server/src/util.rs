use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use futures::Future;
use log::error;
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
#[allow(clippy::manual_async_fn)]
pub(crate) fn convert_to_future(handle: JoinHandle<()>) -> impl Future<Output = ()> {
    async move {
        if let Err(e) = handle.await {
            error!("Error occurred when trying to join a handle: {}", e);
        }
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use std::time::Duration;

    use futures::{future, FutureExt};
    use tokio::time::timeout;

    use super::*;

    #[test]
    fn calculate_hash_u64_returnsCorrectHash() {
        // Arrange
        let data = 1337u64;
        let data_hash = 17549176527789548176;

        // Act
        let result = calculate_hash(&data);

        // Assert
        assert_eq!(data_hash, result);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn convert_to_future_passedAlreadyCompletedHandle_completesImmediately() {
        // Arrange
        let handle = tokio::spawn(future::ready(()));

        // Act
        let future = convert_to_future(handle);

        // Assert
        // `handle` isn't ready "immediately". The given closure still needs to be evaluated, hence the sleep.
        tokio::time::sleep(Duration::from_millis(1)).await;

        assert!(future.now_or_never().is_some());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn convert_to_future_passedANotYetReadyHandle_completesAfterHandleCompletes(
    ) -> Result<(), anyhow::Error> {
        // Arrange
        let handle = tokio::spawn(async { tokio::time::sleep(Duration::from_millis(50)).await });

        // Act
        let future = convert_to_future(handle);

        // Assert
        timeout(Duration::from_millis(75), future).await?;

        Ok(())
    }
}
