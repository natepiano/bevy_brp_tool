//! Detached mode for starting and managing persistent app sessions

use std::env;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sysinfo::{Pid, System};

use super::cli_client;
use super::constants::BIN_NAME;
use super::support::{detect_bevy_app, find_workspace_binary_with_target_dir, poll_until_ready};

/// Session information for a detached app
#[derive(Debug)]
pub struct DetachedSession {
    pub pid: u32,
    pub port: u16,
    pub log_file: PathBuf,
}

/// Persistent session information stored in temp directory
#[derive(Debug, Serialize, Deserialize)]
struct SessionInfo {
    pid: u32,
    port: u16,
    log_file: PathBuf,
    start_time: SystemTime,
    app_binary: String,
}

/// Get the session file prefix used for all session-related files
fn get_session_prefix() -> String {
    format!("{}_session", BIN_NAME)
}

/// Get the path to the session info file for a given port
fn get_session_info_path(port: u16) -> PathBuf {
    env::temp_dir().join(format!("{}_port_{}.json", get_session_prefix(), port))
}

/// Get the path for a session log file with a given timestamp
fn get_session_log_path(timestamp: u128) -> PathBuf {
    env::temp_dir().join(format!("{}_{}.log", get_session_prefix(), timestamp))
}

/// Start app in detached mode with auto-generated temp log file
pub async fn start_detached(
    app_binary: Option<String>,
    port: u16,
    profile: Option<String>,
) -> Result<DetachedSession> {
    // Determine which app to run and get its manifest directory and target directory
    let (app_to_run, manifest_dir, target_dir) = detect_bevy_app(app_binary)?;
    // Generate unique log file name in temp directory using process ID and timestamp
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| anyhow::anyhow!("Failed to get timestamp: {}", e))?
        .as_millis();
    let log_file = get_session_log_path(timestamp);

    // Create log file
    let mut file = File::create(&log_file)
        .with_context(|| format!("Failed to create log file: {:?}", log_file))?;
    writeln!(file, "=== BRP Tool Detached Session ===")?;
    writeln!(file, "Started at: {:?}", std::time::SystemTime::now())?;
    writeln!(file, "Port: {}", port)?;
    writeln!(file, "App binary: {}", app_to_run)?;
    writeln!(file, "============================================\n")?;
    file.sync_all()?;

    // Find full path to app binary using consolidated function
    let app_path =
        find_workspace_binary_with_target_dir(&app_to_run, &target_dir, profile.as_deref())?;

    // Make the path absolute since we'll be changing directories
    let app_path = std::fs::canonicalize(&app_path)?;

    // Start the app in background with output redirected to log file
    let log_file_for_redirect = File::options().append(true).open(&log_file)?;

    // Use the manifest directory for working directory
    println!("Using manifest directory: {:?}", manifest_dir);

    // Write debug info to log file
    let mut log_file_for_debug = File::options().append(true).open(&log_file)?;
    writeln!(
        log_file_for_debug,
        "Debug: Manifest directory set to: {:?}",
        manifest_dir
    )?;
    writeln!(log_file_for_debug, "Debug: Binary path: {:?}", app_path)?;
    writeln!(
        log_file_for_debug,
        "Debug: Current directory before spawn: {:?}",
        std::env::current_dir()?
    )?;
    log_file_for_debug.sync_all()?;

    let child = Command::new(&app_path)
        .current_dir(&manifest_dir)
        .env("CARGO_MANIFEST_DIR", &manifest_dir)
        .arg("--port")
        .arg(port.to_string())
        .stdout(Stdio::from(log_file_for_redirect.try_clone()?))
        .stderr(Stdio::from(log_file_for_redirect))
        .spawn()
        .with_context(|| format!("Failed to start app: {:?}", app_path))?;

    let pid = child.id();

    // Wait for app to be ready (with timeout)
    println!("Starting app in detached mode...");
    println!("Log file: {:?}", log_file);

    let app_ready = poll_until_ready(
        || async move {
            match cli_client::detect_running_instances(port).await {
                Ok(instances) if instances.contains(&port) => Ok(()),
                _ => Err(anyhow::anyhow!("App not responding")),
            }
        },
        Duration::from_secs(30),
        Duration::from_millis(100),
        "Timeout waiting for app to start. Check log file for errors.",
    )
    .await;

    if let Err(error) = app_ready {
        // Try to clean up the process
        let _ = kill_process(pid);
        let _ = fs::remove_file(&log_file);
        return Err(error);
    }

    println!("App started successfully on port {}", port);

    // Save session info to temp directory
    let session_info = SessionInfo {
        pid,
        port,
        log_file: log_file.clone(),
        start_time: SystemTime::now(),
        app_binary: app_to_run.clone(),
    };

    let session_info_path = get_session_info_path(port);
    let session_json = serde_json::to_string_pretty(&session_info)?;
    fs::write(&session_info_path, session_json)
        .with_context(|| format!("Failed to save session info to {:?}", session_info_path))?;

    Ok(DetachedSession {
        pid,
        port,
        log_file,
    })
}

/// Get information about a running detached session
pub async fn get_session_info(port: u16) -> Result<Option<serde_json::Value>> {
    // First check if the session info file exists
    let session_info_path = get_session_info_path(port);

    if !session_info_path.exists() {
        // No session info file - check if app is running anyway
        let instances = cli_client::detect_running_instances(port).await?;
        if instances.contains(&port) {
            return Ok(Some(serde_json::json!({
                "app_running": true,
                "port": port,
                "message": "App is running but no session info found (may have been started manually)"
            })));
        }
        return Ok(None);
    }

    // Read session info
    let mut file = File::open(&session_info_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let session_info: SessionInfo = serde_json::from_str(&contents)?;

    // Check if app is still running
    let instances = cli_client::detect_running_instances(port).await?;
    let app_running = instances.contains(&port);

    // Calculate uptime
    let uptime_seconds = SystemTime::now()
        .duration_since(session_info.start_time)
        .unwrap_or_default()
        .as_secs();

    // Check if process is still alive (platform-specific)
    let process_alive = is_process_alive(session_info.pid);

    let info = serde_json::json!({
        "app_running": app_running,
        "process_alive": process_alive,
        "pid": session_info.pid,
        "port": session_info.port,
        "log_file": session_info.log_file.to_string_lossy(),
        "app_binary": session_info.app_binary,
        "start_time": session_info.start_time,
        "uptime_seconds": uptime_seconds,
        "uptime_formatted": format_duration(uptime_seconds),
    });

    // Clean up stale session info if process is dead
    if !app_running && !process_alive {
        let _ = fs::remove_file(&session_info_path);
    }

    Ok(Some(info))
}

/// Check if a process is still alive
fn is_process_alive(pid: u32) -> bool {
    let mut system = System::new();
    system.refresh_processes(
        sysinfo::ProcessesToUpdate::Some(&[Pid::from_u32(pid)]),
        false,
    );
    system.process(Pid::from_u32(pid)).is_some()
}

/// Format duration in human-readable format
fn format_duration(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, secs)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, secs)
    } else {
        format!("{}s", secs)
    }
}

/// Kill a process by PID (cross-platform)
fn kill_process(pid: u32) -> Result<()> {
    let mut system = System::new();
    system.refresh_processes(
        sysinfo::ProcessesToUpdate::Some(&[Pid::from_u32(pid)]),
        false,
    );

    if let Some(process) = system.process(Pid::from_u32(pid)) {
        process.kill();
        Ok(())
    } else {
        // Process not found - this is not an error, it might have already exited
        Ok(())
    }
}

/// Clean up all session log files and info files
pub async fn cleanup_all_logs() -> Result<()> {
    let temp_dir = env::temp_dir();
    let mut cleaned_count = 0;
    let mut preserved_count = 0;
    let mut error_count = 0;
    let session_prefix = get_session_prefix();
    let mut active_session_files = std::collections::HashSet::new();

    // First pass: identify active sessions by reading all JSON files
    let mut entries = tokio::fs::read_dir(&temp_dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if let Some(file_name) = path.file_name() {
            let file_name_str = file_name.to_string_lossy();
            // Check if it's a session info JSON file
            if file_name_str.starts_with(&session_prefix) && file_name_str.ends_with(".json") {
                // Try to read and parse the session info
                match tokio::fs::read_to_string(&path).await {
                    Ok(contents) => {
                        match serde_json::from_str::<SessionInfo>(&contents) {
                            Ok(session_info) => {
                                // Check if the process is still alive
                                if is_process_alive(session_info.pid) {
                                    // This is an active session - preserve its files
                                    active_session_files.insert(path.clone());
                                    if let Some(log_file_name) = session_info.log_file.file_name() {
                                        active_session_files.insert(temp_dir.join(log_file_name));
                                    }
                                    println!(
                                        "Found active session on port {} (PID: {})",
                                        session_info.port, session_info.pid
                                    );
                                }
                            }
                            Err(e) => {
                                eprintln!(
                                    "Failed to parse session info from {}: {}",
                                    file_name_str, e
                                );
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to read {}: {}", file_name_str, e);
                    }
                }
            }
        }
    }

    // Second pass: clean up files that don't belong to active sessions
    let mut entries = tokio::fs::read_dir(&temp_dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if let Some(file_name) = path.file_name() {
            let file_name_str = file_name.to_string_lossy();
            // Check if it's one of our session files (either .log or .json)
            if file_name_str.starts_with(&session_prefix)
                && (file_name_str.ends_with(".log") || file_name_str.ends_with(".json"))
            {
                if active_session_files.contains(&path) {
                    // This file belongs to an active session - preserve it
                    let file_type = if file_name_str.ends_with(".log") {
                        "log file"
                    } else {
                        "session info"
                    };
                    println!("Preserving active {}: {}", file_type, file_name_str);
                    preserved_count += 1;
                } else {
                    // This file doesn't belong to an active session - remove it
                    match tokio::fs::remove_file(&path).await {
                        Ok(_) => {
                            let file_type = if file_name_str.ends_with(".log") {
                                "log file"
                            } else {
                                "session info"
                            };
                            println!("Removed inactive {}: {}", file_type, file_name_str);
                            cleaned_count += 1;
                        }
                        Err(e) => {
                            eprintln!("Failed to remove {}: {}", file_name_str, e);
                            error_count += 1;
                        }
                    }
                }
            }
        }
    }

    if cleaned_count == 0 && error_count == 0 && preserved_count == 0 {
        println!("No {} session files found", BIN_NAME);
    } else {
        println!("\nCleanup complete:");
        if cleaned_count > 0 {
            println!("  - {} inactive files removed", cleaned_count);
        }
        if preserved_count > 0 {
            println!("  - {} active session files preserved", preserved_count);
        }
        if error_count > 0 {
            println!("  - {} files could not be removed (errors)", error_count);
        }
    }

    Ok(())
}
