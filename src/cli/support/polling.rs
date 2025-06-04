use std::time::Duration;

use anyhow::{Result, bail};
use tokio::time::{interval, timeout};

/// Generic polling helper that waits for a condition to be met
///
/// This function polls a check function at regular intervals until:
/// - The check function returns Ok(()) - indicating success
/// - The timeout is reached - returns the timeout error message
///
/// # Parameters
/// - `check_fn`: An async function that returns Ok(()) when ready, or an error when not ready
/// - `timeout_duration`: Maximum time to wait before timing out
/// - `poll_interval`: How often to check the condition
/// - `timeout_message`: Error message to use when timeout occurs
pub async fn poll_until_ready<F, Fut>(
    mut check_fn: F,
    timeout_duration: Duration,
    poll_interval: Duration,
    timeout_message: impl Into<String>,
) -> Result<()>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<()>>,
{
    let timeout_msg = timeout_message.into();
    let mut interval = interval(poll_interval);

    match timeout(timeout_duration, async {
        loop {
            interval.tick().await;

            if check_fn().await.is_ok() {
                return Ok(());
            }
            // Continue polling on any error
        }
    })
    .await
    {
        Ok(result) => result,
        Err(_) => bail!(timeout_msg),
    }
}
