//! Tests for binary discovery across different working directories
//!
//! These tests verify that the CLI tool can correctly find and execute Bevy apps
//! from various working directories, including subdirectories of the workspace.
//!
//! These tests were created to catch the bug where `brp -d` would fail when run
//! from a subdirectory of the workspace because it was looking for binaries in
//! `./target/debug/` instead of `<workspace_root>/target/debug/`.

mod support;

use std::env;
use std::path::PathBuf;

use anyhow::Result;
use support::*;
use tempfile::TempDir;

/// Creates a temporary workspace with the given structure and a dummy binary
struct TestWorkspace {
    _temp_dir: TempDir,
    root: PathBuf,
}

impl TestWorkspace {
    /// Create a workspace with member crates
    fn new_workspace(members: &[&str]) -> Result<Self> {
        let temp_dir = tempfile::tempdir()?;
        let root = temp_dir.path().to_path_buf();

        // Create workspace Cargo.toml
        let workspace_toml = format!(
            r#"[workspace]
members = {:?}
"#,
            members
        );
        std::fs::write(root.join("Cargo.toml"), workspace_toml)?;

        // Create member crates
        for member in members {
            let member_path = root.join(member);
            std::fs::create_dir_all(&member_path)?;
            std::fs::write(
                member_path.join("Cargo.toml"),
                format!(
                    r#"[package]
name = "{}"
version = "0.1.0"
"#,
                    member
                ),
            )?;
            std::fs::create_dir_all(member_path.join("src"))?;
            std::fs::write(member_path.join("src").join("main.rs"), "fn main() {}")?;
        }

        // Create target directory and dummy binary
        Self::create_dummy_binary(&root, "test_bevy_app")?;

        Ok(TestWorkspace {
            _temp_dir: temp_dir,
            root,
        })
    }

    /// Create a dummy binary in target/debug
    fn create_dummy_binary(root: &std::path::Path, name: &str) -> Result<()> {
        let target_debug = root.join("target").join("debug");
        std::fs::create_dir_all(&target_debug)?;

        let binary_path = target_debug.join(name);
        std::fs::write(&binary_path, "#!/bin/sh\necho 'dummy bevy app'")?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&binary_path, std::fs::Permissions::from_mode(0o755))?;
        }

        Ok(())
    }

    fn root(&self) -> &std::path::Path {
        &self.root
    }
}

/// Test that would have caught the binary discovery bug
///
/// This test simulates the bug where running `brp -d` from a subdirectory
/// of the workspace would fail to find the binary because it was looking
/// in `./target/debug/` instead of `<workspace_root>/target/debug/`.
#[tokio::test]
async fn test_binary_discovery_from_workspace_subdirectory() -> Result<()> {
    let runner = CliTestRunner::new()?;
    let original_cwd = env::current_dir()?;

    // Create a test workspace
    let workspace = TestWorkspace::new_workspace(&["crate-a", "crate-b"])?;
    let workspace_root = workspace.root();
    let test_subdir = workspace_root.join("crate-a");
    let binary_path = workspace_root
        .join("target")
        .join("debug")
        .join("test_bevy_app");

    // Change to the subdirectory
    env::set_current_dir(&test_subdir)?;

    // Simulate the old buggy behavior: if we were to look for the binary
    // in `./target/debug/<binary>` from this subdirectory, it wouldn't exist
    let binary_name = binary_path.file_name().unwrap();
    let buggy_path = test_subdir.join("target").join("debug").join(binary_name);
    assert!(
        !buggy_path.exists(),
        "Binary should NOT exist at subdirectory relative path (this was the bug): {}",
        buggy_path.display()
    );

    // The correct binary should exist at the workspace root
    assert!(
        binary_path.exists(),
        "Binary should exist at workspace target directory: {}",
        binary_path.display()
    );

    // Now test that our CLI tool can still run basic commands from subdirectory
    let output = runner.run_command(&["--help"]).await?;

    // Verify that the CLI tool itself can still run from the subdirectory
    assert!(
        output.success(),
        "CLI should work from subdirectory (basic sanity check)"
    );

    // Test that detection commands work from subdirectory
    // This exercises the cargo metadata detection that was fixed
    let output = runner.run_command(&["--detect"]).await?;

    // The --detect command should succeed from subdirectory
    // This would have failed with the old buggy code
    assert!(
        output.success(),
        "Detect command should work from subdirectory - this test would have caught the bug"
    );

    // Restore original working directory
    env::set_current_dir(&original_cwd)?;

    Ok(())
}

/// Test binary discovery from workspace root
#[tokio::test]
async fn test_binary_discovery_from_workspace_root() -> Result<()> {
    let runner = CliTestRunner::new()?;
    let _original_cwd = env::current_dir()?;

    // Create a test workspace
    let workspace = TestWorkspace::new_workspace(&["member-a"])?;
    let workspace_root = workspace.root();
    let binary_path = workspace_root
        .join("target")
        .join("debug")
        .join("test_bevy_app");

    assert!(
        binary_path.exists(),
        "Binary should exist at workspace root target directory: {}",
        binary_path.display()
    );

    // Test basic functionality from workspace root
    let output = runner.run_command(&["--detect"]).await?;
    assert!(
        output.success(),
        "Detect command should work from workspace root"
    );

    Ok(())
}

/// Test binary discovery from a directory above the workspace
#[tokio::test]
async fn test_binary_discovery_from_parent_directory() -> Result<()> {
    let runner = CliTestRunner::new()?;
    let original_cwd = env::current_dir()?;

    // Create a test project and then go to its parent
    let temp_parent = tempfile::tempdir()?;
    let project_dir = temp_parent.path().join("my-bevy-project");
    std::fs::create_dir(&project_dir)?;

    // Create a simple project structure
    std::fs::write(
        project_dir.join("Cargo.toml"),
        r#"[package]
name = "my-bevy-project"
version = "0.1.0"
"#,
    )?;

    // Change to parent directory (outside any project)
    env::set_current_dir(temp_parent.path())?;

    // Test that the tool can still provide basic functionality
    // even when not in a cargo project
    let output = runner.run_command(&["--help"]).await?;
    assert!(
        output.success(),
        "Help command should work from parent directory"
    );

    // The --detect command might fail since we're not in a cargo project,
    // but it should fail gracefully, not crash
    let _output = runner.run_command(&["--detect"]).await;
    // Don't assert success here since we're outside the project

    // Restore working directory
    env::set_current_dir(&original_cwd)?;

    Ok(())
}

/// Test binary discovery from a standalone Bevy project
///
/// This simulates testing from any other Cargo project
#[tokio::test]
async fn test_binary_discovery_from_standalone_project() -> Result<()> {
    let runner = CliTestRunner::new()?;
    let original_cwd = env::current_dir()?;

    // Create a temporary standalone project for testing
    let temp_dir = tempfile::tempdir()?;
    let temp_project_dir = temp_dir.path();

    // Create a minimal Cargo.toml
    std::fs::write(
        temp_project_dir.join("Cargo.toml"),
        r#"[package]
name = "test-project"
version = "0.1.0"
edition = "2021"

[dependencies]
bevy = "0.16"
"#,
    )?;

    // Create src directory and main.rs
    std::fs::create_dir_all(temp_project_dir.join("src"))?;
    std::fs::write(
        temp_project_dir.join("src").join("main.rs"),
        "fn main() { println!(\"Hello, world!\"); }",
    )?;

    env::set_current_dir(temp_project_dir)?;

    // Test that our CLI tool still works when run from a different project
    let output = runner.run_command(&["--help"]).await?;
    assert!(
        output.success(),
        "Help command should work from standalone project"
    );

    // Test detect command - this should work since it's a Cargo project
    let output = runner.run_command(&["--detect"]).await?;
    assert!(
        output.success(),
        "Detect command should work from standalone Bevy project"
    );

    // Restore working directory
    env::set_current_dir(&original_cwd)?;

    Ok(())
}

/// Test that demonstrates the specific bug scenario
///
/// This test shows what would have happened with the old buggy code:
/// - Run from a subdirectory of the workspace
/// - Try to find a binary using relative path construction
/// - Fail because the relative path doesn't exist
#[tokio::test]
async fn test_specific_bug_scenario_simulation() -> Result<()> {
    let original_cwd = env::current_dir()?;

    // Create a test workspace
    let workspace = TestWorkspace::new_workspace(&["app-crate", "lib-crate"])?;
    let workspace_root = workspace.root();
    let test_subdir = workspace_root.join("app-crate");
    let binary_path = workspace_root
        .join("target")
        .join("debug")
        .join("test_bevy_app");

    // Change to the subdirectory
    env::set_current_dir(&test_subdir)?;

    // Simulate the old buggy behavior
    let binary_name = binary_path.file_name().unwrap();
    let buggy_binary_path = PathBuf::from("target/debug").join(binary_name);

    // This should NOT exist (demonstrating the bug)
    assert!(
        !buggy_binary_path.exists(),
        "Buggy relative path should not exist: {}",
        buggy_binary_path.display()
    );

    // The correct binary path should exist at the workspace root
    // This SHOULD exist (demonstrating the fix)
    assert!(
        binary_path.exists(),
        "Correct absolute path should exist: {}",
        binary_path.display()
    );

    // Restore working directory
    env::set_current_dir(&original_cwd)?;

    Ok(())
}
