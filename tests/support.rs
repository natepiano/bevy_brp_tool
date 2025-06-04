//! Test support utilities for bevy_brp_tool integration tests

#![allow(dead_code)]

use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use anyhow::Result;
use bevy::app::App;
use bevy::prelude::*;
use bevy_brp_tool::BrpToolPlugin;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::process::Command;
use tokio::time::timeout;

/// Base port for tests (avoiding conflict with default 15702)
const TEST_PORT_BASE: u16 = 16000;

/// Test run mode for the Bevy app
#[derive(Clone, Copy)]
pub enum TestRunMode {
    /// Run once and exit
    Once,
    /// Run in a loop (for tests requiring multiple frames)
    Loop,
}

/// Test component with various field types
#[derive(Component, Debug, Default, Reflect, Serialize, Deserialize)]
#[reflect(Component, Deserialize, Serialize)]
pub struct TestComponent {
    /// Integer value for testing
    pub value: i32,
    /// String name for testing
    pub name: String,
    /// Boolean flag for testing
    pub enabled: bool,
}

/// Another test component for multi-component queries
#[derive(Component, Debug, Default, Reflect, Serialize, Deserialize)]
#[reflect(Component, Deserialize, Serialize)]
pub struct SecondaryComponent {
    /// Vector data for testing
    pub data: Vec<f32>,
}

/// Test resource
#[derive(Resource, Debug, Default, Reflect, Serialize, Deserialize)]
#[reflect(Resource, Deserialize, Serialize)]
pub struct TestResource {
    /// Counter value for testing
    pub counter: u32,
    /// Message string for testing
    pub message: String,
}

/// Marker component for hierarchy testing
#[derive(Component)]
pub struct Parent;

/// Child marker component for hierarchy testing
#[derive(Component)]
pub struct Child;

/// Get the fully qualified type name for TestComponent using Bevy's reflection system
pub fn test_component_type() -> String {
    format!("{}::support::TestComponent", env!("CARGO_CRATE_NAME"))
}

/// Get the fully qualified type name for SecondaryComponent using Bevy's reflection system
pub fn secondary_component_type() -> String {
    format!("{}::support::SecondaryComponent", env!("CARGO_CRATE_NAME"))
}

/// Get the fully qualified type name for TestResource using Bevy's reflection system
pub fn test_resource_type() -> String {
    format!("{}::support::TestResource", env!("CARGO_CRATE_NAME"))
}

/// Create a temporary directory for test outputs
pub fn create_test_output_dir() -> tempfile::TempDir {
    tempfile::tempdir().expect("Failed to create temp dir")
}

/// Helper to parse entity ID from spawn response
pub fn extract_entity_id(response: &Value) -> Result<u64> {
    response
        .get("entity")
        .and_then(|e| e.as_u64())
        .ok_or_else(|| anyhow::anyhow!("No entity ID in response"))
}

/// Helper to unpack a Bevy Entity ID into index and generation components
pub fn unpack_entity_id(entity_id: u64) -> (u64, u64) {
    let entity_index = entity_id & 0xFFFFFFFF;
    let entity_generation = (entity_id >> 32) & 0xFFFFFFFF;
    (entity_index, entity_generation)
}

/// Helper to check if an entity exists in the detailed entity list
pub fn entity_exists_in_list(entities: &Value, entity_id: u64) -> bool {
    let (entity_index, entity_generation) = unpack_entity_id(entity_id);

    entities.as_array().unwrap_or(&vec![]).iter().any(|e| {
        let list_index = e.get("entity").and_then(|id| id.as_u64()).unwrap_or(0);
        let list_generation = e.get("generation").and_then(|g| g.as_u64()).unwrap_or(0);
        list_index == entity_index && list_generation == entity_generation
    })
}

/// Allocate a unique port for a test using process-based allocation
pub fn allocate_test_port() -> u16 {
    let process_id = std::process::id();
    TEST_PORT_BASE + (process_id % 1000) as u16
}

/// Test app handle that ensures cleanup
pub struct TestApp {
    port: u16,
    handle: Option<tokio::task::JoinHandle<()>>,
}

impl TestApp {
    /// Start a new test app with collision recovery
    pub async fn new(run_mode: TestRunMode) -> Result<Self> {
        let base_port = allocate_test_port();
        let mut port = base_port;
        let mut attempts = 0;

        // Try up to 10 different ports if we get collisions
        loop {
            match Self::try_start_on_port(port, run_mode).await {
                Ok(app) => return Ok(app),
                Err(_) if attempts < 10 => {
                    attempts += 1;
                    port = base_port + attempts;
                    // Add small jitter to avoid thundering herd
                    tokio::time::sleep(Duration::from_millis(attempts as u64 * 10)).await;
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Try to start app on specific port
    async fn try_start_on_port(port: u16, run_mode: TestRunMode) -> Result<Self> {
        // Create app in a separate thread to avoid blocking the async runtime
        let _handle = std::thread::spawn(move || {
            let mut app = create_test_app(port, run_mode);
            app.run();
        });

        // Start polling immediately instead of waiting
        // The wait_for_ready function will handle retries

        // Wait for app to be ready using CLI
        wait_for_ready_cli(port, Duration::from_secs(10)).await?;

        // Convert thread handle to tokio task handle for compatibility
        let task_handle = tokio::spawn(async move {
            // This will wait forever or until aborted
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        });

        Ok(TestApp {
            port,
            handle: Some(task_handle),
        })
    }

    /// Get the port this app is running on
    pub fn port(&self) -> u16 {
        self.port
    }

    // Note: client() method removed - use CliTestRunner instead
}

impl Drop for TestApp {
    fn drop(&mut self) {
        // Ensure app is stopped when test completes
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
    }
}

/// Create a test Bevy app with remote control plugin
fn create_test_app(port: u16, _run_mode: TestRunMode) -> App {
    let mut app = App::new();

    // MinimalPlugins includes: TaskPoolPlugin, TypeRegistrationPlugin, FrameCountPlugin,
    // TimePlugin, ScheduleRunnerPlugin For tests, we always need the app to keep running (not
    // exit after one frame) so we use run_loop even for "Once" mode tests
    // Use 8ms (125 FPS) for tests to reduce frame-based delays
    let runner = bevy::app::ScheduleRunnerPlugin::run_loop(Duration::from_millis(8));

    app.add_plugins(MinimalPlugins.set(runner))
        .add_plugins(BrpToolPlugin::with_port(port))
        .register_type::<TestComponent>()
        .register_type::<SecondaryComponent>()
        .register_type::<TestResource>()
        .register_type::<Name>()
        .register_type::<Transform>()
        .register_type::<bevy::ecs::hierarchy::ChildOf>()
        .register_type::<bevy::ecs::hierarchy::Children>()
        .init_resource::<TestResource>()
        .add_systems(Startup, setup_test_world);

    app
}

/// Setup a test world with various entities and components
fn setup_test_world(mut commands: Commands) {
    // Create test entities without hierarchy (to avoid needing Transform/Hierarchy plugins)
    commands.spawn((
        Name::new("Entity1"),
        TestComponent {
            value: 100,
            name: "entity1".into(),
            enabled: true,
        },
    ));

    commands.spawn((
        Name::new("Entity2"),
        TestComponent {
            value: 200,
            name: "entity2".into(),
            enabled: false,
        },
    ));

    commands.spawn((
        Name::new("Entity3"),
        SecondaryComponent {
            data: vec![1.0, 2.0, 3.0],
        },
    ));

    // Entity with both components
    commands.spawn((
        Name::new("Entity4"),
        TestComponent {
            value: 300,
            name: "entity4".into(),
            enabled: true,
        },
        SecondaryComponent {
            data: vec![4.0, 5.0],
        },
    ));
}

/// Wait for app to report ready using CLI
pub async fn wait_for_ready_cli(port: u16, timeout_duration: Duration) -> Result<()> {
    let runner = CliTestRunner::new()?;
    timeout(timeout_duration, async {
        let mut retry_count = 0;
        loop {
            let port_str = port.to_string();
            match runner.run_command(&["--port", &port_str, "ready"]).await {
                Ok(output) if output.success() => {
                    if let Ok(json) = output.parse_json() {
                        if json.get("ready").and_then(|v| v.as_bool()) == Some(true) {
                            return Ok(());
                        }
                    }
                }
                _ => {
                    // Use exponential backoff with a cap
                    // Start with 10ms, double each time up to 100ms
                    let delay = std::cmp::min(10 * (1 << retry_count), 100);
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                    retry_count += 1;
                }
            }
        }
    })
    .await?
}

/// CLI test infrastructure for testing the CLI binary
pub struct CliTestRunner {
    binary_path: PathBuf,
}

impl CliTestRunner {
    /// Create a new CLI test runner
    pub fn new() -> Result<Self> {
        let binary_path = Self::find_binary_path()?;
        Ok(Self { binary_path })
    }

    /// Find the binary path
    fn find_binary_path() -> Result<PathBuf> {
        // For standalone crate, look in the local target directory
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let binary_path = manifest_dir.join("target").join("debug").join("brp");

        if binary_path.exists() {
            return Ok(binary_path);
        }

        // Fall back to trying cargo metadata to find target directory
        anyhow::bail!(
            "Could not find brp binary at {}. Make sure the project is built.",
            binary_path.display()
        )
    }

    /// Execute a CLI command and capture output
    pub async fn run_command(&self, args: &[&str]) -> Result<CliOutput> {
        let mut cmd = Command::new(&self.binary_path);
        cmd.args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null());

        let output = cmd.output().await?;

        Ok(CliOutput {
            status: output.status,
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }

    /// Execute a CLI command with app connection (requires a running TestApp)
    pub async fn run_command_with_app(&self, args: &[&str], app: &TestApp) -> Result<CliOutput> {
        let port_string = app.port().to_string();
        let mut full_args = vec!["--port", &port_string];
        full_args.extend_from_slice(args);

        self.run_command(&full_args).await
    }

    /// Execute a CLI command in managed mode
    pub async fn run_managed_command(&self, app_binary: &str, commands: &str) -> Result<CliOutput> {
        let args = vec!["--managed-commands", commands, "--app", app_binary];

        // For managed mode, we need to handle longer running processes
        // Use a timeout to prevent hanging
        let mut cmd = Command::new(&self.binary_path);
        cmd.args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null());

        let output = timeout(Duration::from_secs(30), cmd.output()).await??;

        Ok(CliOutput {
            status: output.status,
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }
}

/// Output from a CLI command execution
#[derive(Debug)]
pub struct CliOutput {
    /// Exit status of the command
    pub status: std::process::ExitStatus,
    /// Standard output from the command
    pub stdout: String,
    /// Standard error from the command
    pub stderr: String,
}

impl CliOutput {
    /// Check if the command succeeded
    pub fn success(&self) -> bool {
        self.status.success()
    }

    /// Get the exit code
    pub fn exit_code(&self) -> Option<i32> {
        self.status.code()
    }

    /// Check if stdout contains the given text
    pub fn stdout_contains(&self, text: &str) -> bool {
        self.stdout.contains(text)
    }

    /// Check if stderr contains the given text
    pub fn stderr_contains(&self, text: &str) -> bool {
        self.stderr.contains(text)
    }

    /// Parse stdout as JSON
    pub fn parse_json(&self) -> Result<Value> {
        Ok(serde_json::from_str(&self.stdout)?)
    }

    /// Get stdout lines as a vector
    pub fn stdout_lines(&self) -> Vec<&str> {
        self.stdout.lines().collect()
    }

    /// Get stderr lines as a vector
    pub fn stderr_lines(&self) -> Vec<&str> {
        self.stderr.lines().collect()
    }
}
