use std::time::Duration;

use anyhow::Result;
use tokio::time::sleep;

use super::client::RemoteClient;
use super::commands::{Commands, execute_standalone_command, parse_command_string};
use super::support::{is_connection_error, poll_until_ready};
use crate::DEFAULT_REMOTE_PORT;

/// Detect running instances on common ports
pub async fn detect_running_instances(requested_port: u16) -> Result<Vec<u16>> {
    let mut running_ports = Vec::new();

    // Check the requested port first
    if is_port_responsive(requested_port).await {
        running_ports.push(requested_port);
    }

    // If requested port is the default, check a few nearby ports
    if requested_port == DEFAULT_REMOTE_PORT {
        for offset in 1..=5 {
            let port = DEFAULT_REMOTE_PORT + offset;
            if is_port_responsive(port).await {
                running_ports.push(port);
            }
        }
    }

    Ok(running_ports)
}

/// Check if a port has a responsive BRP-enabled instance
async fn is_port_responsive(port: u16) -> bool {
    let client = RemoteClient::new(port);
    // Try to connect with a BRP command - this is a quick check
    client.is_ready().await.unwrap_or(false)
}

/// Wait for the app to be ready by polling with BRP commands
pub async fn wait_for_app_ready(client: &RemoteClient) -> Result<()> {
    let port = client.port();

    poll_until_ready(
        || async {
            match client.is_ready().await {
                Ok(true) => Ok(()),
                Ok(false) => anyhow::bail!("App not ready"),
                Err(e) => {
                    let error_str = e.to_string();
                    if is_connection_error(&error_str) {
                        anyhow::bail!("Connection error: {}", error_str);
                    } else {
                        // Non-connection error might mean the app is starting up
                        anyhow::bail!("App error: {}", e);
                    }
                }
            }
        },
        Duration::from_secs(5),
        Duration::from_millis(50),
        format!(
            "No app is running on port {}. Start the app first or use --managed mode.",
            port
        ),
    )
    .await
}

/// Execute a single command
pub async fn execute_command(client: &RemoteClient, command: &str) -> Result<()> {
    // Handle special wait command
    if let Some(duration_str) = command.strip_prefix("wait:") {
        let seconds: u64 = duration_str.parse()?;
        println!("Waiting {} seconds...", seconds);
        sleep(Duration::from_secs(seconds)).await;
        return Ok(());
    }

    // Parse the command string into a Commands enum
    match parse_command_string(command) {
        Ok(cmd) => {
            // Delegate to the standalone command executor
            execute_standalone_command(client, cmd).await
        }
        Err(parse_error) => {
            // If parsing fails, check if it's a raw command with method syntax
            let parts: Vec<&str> = command.splitn(2, ' ').collect();
            let cmd_name = parts[0];

            // Check if it looks like a method call (contains / or .)
            if cmd_name.contains('/') || cmd_name.contains('.') {
                // Try as a raw command
                let raw_args: Vec<String> =
                    command.split_whitespace().map(|s| s.to_string()).collect();
                execute_standalone_command(client, Commands::Raw { args: raw_args }).await
            } else {
                // Return the parse error
                Err(parse_error)
            }
        }
    }
}
