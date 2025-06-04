use std::time::Duration;

use anyhow::Result;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::time::sleep;

use super::cli_client::{execute_command, wait_for_app_ready};
use super::client::RemoteClient;
use super::support::{
    detect_bevy_app, find_workspace_binary_with_target_dir, is_port_available,
    wait_for_port_connectable,
};
use crate::DEFAULT_REMOTE_PORT;

/// Run in managed mode (start app and manage lifecycle)
pub async fn run_managed(
    app: Option<String>,
    commands: Option<String>,
    requested_port: u16,
    profile: Option<String>,
) -> Result<()> {
    // Determine which app to run and get its manifest directory and target directory
    let (app_to_run, manifest_dir, target_dir) = detect_bevy_app(app)?;

    // Find the app binary in the workspace using the target directory
    let app_path =
        find_workspace_binary_with_target_dir(&app_to_run, &target_dir, profile.as_deref())?;
    // Make the path absolute since we'll be changing directories
    let app_path = std::fs::canonicalize(&app_path)?;
    println!("Starting app: {}", app_path.display());

    // Pick an appropriate port: use random if default was requested, otherwise use what user
    // specified
    let port = if requested_port == DEFAULT_REMOTE_PORT {
        pick_random_available_port().await?
    } else {
        requested_port
    };

    // Use the manifest directory for the working directory and CARGO_MANIFEST_DIR
    // This ensures assets are found relative to the crate's location
    println!("Using manifest directory: {:?}", manifest_dir);

    // Spawn the subprocess with custom port
    let mut child = Command::new(&app_path)
        .current_dir(&manifest_dir)
        .env("CARGO_MANIFEST_DIR", &manifest_dir)
        .arg("--port")
        .arg(port.to_string())
        .kill_on_drop(true)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    // Set up output streaming
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow::anyhow!("Failed to get stdout"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| anyhow::anyhow!("Failed to get stderr"))?;

    // Spawn tasks to print stdout/stderr
    let app_name = app_to_run.clone();
    let stdout_task = tokio::spawn(async move {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();
        while let Some(line) = lines.next_line().await.unwrap_or(None) {
            println!("[{}] {}", app_name, line);
        }
    });

    let app_name = app_to_run.clone();
    let stderr_task = tokio::spawn(async move {
        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();
        while let Some(line) = lines.next_line().await.unwrap_or(None) {
            eprintln!("[{}] {}", app_name, line);
        }
    });

    // Wait for app to start by checking if port is available
    wait_for_port(port, Duration::from_secs(10)).await?;
    println!(
        "\nApp started on port {}. Ready for remote commands.\n",
        port
    );

    // Execute the command list
    if let Some(commands) = commands {
        run_command_list(commands, port).await?;
    } else {
        anyhow::bail!("No commands provided for managed mode");
    }

    // Clean up
    child.kill().await?;
    stdout_task.abort();
    stderr_task.abort();

    Ok(())
}

/// Pick a random available port in a safe range for managed instances
async fn pick_random_available_port() -> Result<u16> {
    use rand::Rng;

    // Port range for managed instances:
    // - Start at 15703 (one above the default BRP port 15702)
    // - End at 16702 to provide 1000 port options
    // This avoids conflicts with the default port while giving plenty of options
    // for multiple managed instances to run concurrently
    const MIN_PORT: u16 = 15703;
    const MAX_PORT: u16 = 16702;
    const MAX_ATTEMPTS: u32 = 50;

    let mut rng = rand::rng();

    for _ in 0..MAX_ATTEMPTS {
        let port = rng.random_range(MIN_PORT..=MAX_PORT);

        // Check if port is available by trying to bind to it
        if is_port_available(port).await {
            println!("Selected random port: {}", port);
            return Ok(port);
        }
        // Port is in use, try another
    }

    anyhow::bail!(
        "Could not find an available port after {} attempts",
        MAX_ATTEMPTS
    )
}

/// Wait for a port to become available
async fn wait_for_port(port: u16, timeout_duration: Duration) -> Result<()> {
    wait_for_port_connectable(port, timeout_duration).await
}

/// Run a comma-separated list of commands with proper JSON handling
async fn run_command_list(commands: String, port: u16) -> Result<()> {
    let client = RemoteClient::new(port);

    // Ensure app is ready before executing commands
    wait_for_app_ready(&client).await?;

    let commands = parse_command_list(&commands)?;

    for command in commands {
        let command = command.trim();
        println!("\n=== Executing: {} ===", command);

        if let Some(wait_time) = command.strip_prefix("wait:") {
            let seconds: u64 = wait_time.parse()?;
            println!("Waiting {} seconds...", seconds);
            sleep(Duration::from_secs(seconds)).await;
        } else {
            execute_command(&client, command).await?;
        }
    }

    Ok(())
}

/// Parse a command list respecting JSON structure
fn parse_command_list(input: &str) -> Result<Vec<String>> {
    let mut commands = Vec::new();
    let mut current_command = String::new();
    let mut in_json = false;
    let mut brace_count = 0;
    let mut in_string = false;
    let mut escape_next = false;

    for ch in input.chars() {
        if escape_next {
            current_command.push(ch);
            escape_next = false;
            continue;
        }

        match ch {
            '\\' if in_json => {
                current_command.push(ch);
                escape_next = true;
            }
            '"' if in_json => {
                current_command.push(ch);
                if !escape_next {
                    in_string = !in_string;
                }
            }
            '{' if !in_string => {
                in_json = true;
                brace_count += 1;
                current_command.push(ch);
            }
            '}' if !in_string && in_json => {
                brace_count -= 1;
                current_command.push(ch);
                if brace_count == 0 {
                    in_json = false;
                }
            }
            ',' if !in_json => {
                if !current_command.trim().is_empty() {
                    commands.push(current_command.trim().to_string());
                }
                current_command.clear();
            }
            _ => {
                current_command.push(ch);
            }
        }
    }

    // Don't forget the last command
    if !current_command.trim().is_empty() {
        commands.push(current_command.trim().to_string());
    }

    Ok(commands)
}
