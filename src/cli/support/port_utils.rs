//! Port utility functions for checking availability and connectivity
//!
//! This module provides shared utilities for:
//! - Checking if a port is available for binding
//! - Testing if a port has a responsive service
//! - Detecting connection errors
//! - Waiting for ports to become connectable

use std::time::Duration;

use anyhow::{Result, bail};
use tokio::net::{TcpListener, TcpStream};

use super::polling::poll_until_ready;
use crate::cli::constants::POLL_INTERVAL_MS;

/// Check if an error string indicates a connection failure
///
/// This detects common connection errors that indicate the server is not running,
/// as opposed to other errors that might indicate the server is running but having issues.
pub fn is_connection_error(error_str: &str) -> bool {
    error_str.contains("Connection refused")
        || error_str.contains("tcp connect error")
        || error_str.contains("error sending request")
}

/// Check if a port is available for binding
///
/// Returns true if the port can be bound to, false if it's already in use.
pub async fn is_port_available(port: u16) -> bool {
    TcpListener::bind(("127.0.0.1", port)).await.is_ok()
}

/// Wait for a port to become connectable with improved error detection
///
/// This function polls the port until it becomes connectable, with better error messages
/// distinguishing between timeout and port already in use scenarios.
///
/// # Parameters
/// - `port`: The port to wait for
/// - `timeout_duration`: Maximum time to wait before timing out
///
/// # Returns
/// - `Ok(())` if the port becomes connectable within the timeout
/// - `Err` with appropriate message if timeout occurs or port is already in use
pub async fn wait_for_port_connectable(port: u16, timeout_duration: Duration) -> Result<()> {
    let result = poll_until_ready(
        || async move {
            match TcpStream::connect(("127.0.0.1", port)).await {
                Ok(_) => Ok(()),
                Err(_) => Err(anyhow::anyhow!("Port not ready")),
            }
        },
        timeout_duration,
        Duration::from_millis(POLL_INTERVAL_MS),
        format!("Timeout waiting for app to start on port {}", port),
    )
    .await;

    // If timeout occurred, check if port is already in use to provide better error message
    if result.is_err() {
        match TcpListener::bind(("127.0.0.1", port)).await {
            Ok(_) => {
                bail!(
                    "Timeout waiting for app to start on port {}. The app may have failed to start.",
                    port
                );
            }
            Err(_) => {
                bail!(
                    "Port {} is already in use by another process. Use -p/--port to specify a different port.",
                    port
                );
            }
        }
    }

    result
}
