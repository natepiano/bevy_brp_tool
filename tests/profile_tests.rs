//! Tests for the --profile flag functionality

mod support;

use anyhow::Result;
use support::*;

/// Test that --profile flag is accepted by the CLI
#[tokio::test]
async fn test_profile_flag_accepted() -> Result<()> {
    let runner = CliTestRunner::new()?;

    // Test with long form
    let output = runner
        .run_command(&["--profile", "release", "--help"])
        .await?;
    assert!(output.success(), "Should accept --profile flag");

    // Test with short form
    let output = runner.run_command(&["-P", "release", "--help"]).await?;
    assert!(output.success(), "Should accept -P short form");

    Ok(())
}

/// Test that help text includes profile flag documentation
#[tokio::test]
async fn test_profile_help_text() -> Result<()> {
    let runner = CliTestRunner::new()?;

    let output = runner.run_command(&["--help"]).await?;
    assert!(output.success());

    assert!(
        output.stdout_contains("--profile"),
        "Help should mention --profile flag"
    );
    assert!(
        output.stdout_contains("-P"),
        "Help should mention -P short form"
    );
    assert!(
        output.stdout_contains("Build profile to use"),
        "Help should include profile description"
    );

    Ok(())
}

// Note: --help-for only works with commands, not flags, so we can't test detailed help for
// --profile

/// Test that profile flag works with managed mode
#[tokio::test]
async fn test_profile_with_managed_mode() -> Result<()> {
    let runner = CliTestRunner::new()?;

    // This will fail because no app exists, but it should parse the arguments correctly
    let output = runner
        .run_command(&[
            "--profile",
            "release",
            "--managed-commands",
            "list",
            "--app",
            "nonexistent",
        ])
        .await?;

    // Should fail because app doesn't exist, but shouldn't fail on argument parsing
    assert!(!output.success());
    // Should get past argument parsing and fail on app detection or binary finding
    assert!(
        output.stderr_contains("not found") || output.stderr_contains("No Bevy app"),
        "Should fail on app/binary detection, not argument parsing"
    );

    Ok(())
}

/// Test that profile flag works with detached mode
#[tokio::test]
async fn test_profile_with_detached_mode() -> Result<()> {
    let runner = CliTestRunner::new()?;

    // This will fail because no app exists, but it should parse the arguments correctly
    let output = runner
        .run_command(&["--profile", "release", "--detached", "--app", "nonexistent"])
        .await?;

    // Should fail because app doesn't exist, but shouldn't fail on argument parsing
    assert!(!output.success());
    assert!(
        output.stderr_contains("not found") || output.stderr_contains("No Bevy app"),
        "Should fail on app/binary detection, not argument parsing"
    );

    Ok(())
}

/// Test that invalid profile names are rejected
#[tokio::test]
async fn test_invalid_profile_names() -> Result<()> {
    let runner = CliTestRunner::new()?;

    // Test profile with path separator
    let output = runner
        .run_command(&["--profile", "../evil", "--managed-commands", "list"])
        .await?;

    assert!(!output.success());
    assert!(
        output.stderr_contains("Invalid profile name")
            || output.stderr_contains("cannot contain path separators"),
        "Should reject profile names with path separators"
    );

    Ok(())
}

/// Test that profile flag is properly passed through to error messages
#[tokio::test]
async fn test_profile_in_error_messages() -> Result<()> {
    let runner = CliTestRunner::new()?;

    // Create a temporary workspace to test in
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

    // Change to the test directory
    let original_cwd = std::env::current_dir()?;
    std::env::set_current_dir(temp_project_dir)?;

    // Try to run with a custom profile that doesn't exist
    let output = runner
        .run_command(&["--profile", "production", "--detect"])
        .await?;

    // The detect command itself should succeed
    assert!(output.success(), "Detect command should work");

    // But if we try to actually use the binary, it should fail with the right error
    let output = runner
        .run_command(&["--profile", "production", "--managed-commands", "list"])
        .await?;

    assert!(!output.success());
    assert!(
        output.stderr_contains("cargo build --profile production"),
        "Error message should suggest building with the specified profile"
    );

    // Restore working directory
    std::env::set_current_dir(&original_cwd)?;

    Ok(())
}

/// Test default profile behavior (should use debug)
#[tokio::test]
async fn test_default_profile_is_debug() -> Result<()> {
    let runner = CliTestRunner::new()?;

    // When no profile is specified, it should look in debug directory
    // This test just verifies the arguments are accepted
    let output = runner.run_command(&["--help"]).await?;
    assert!(output.success());

    // The actual behavior of defaulting to debug is tested in binary_discovery_tests.rs

    Ok(())
}

/// Test that common profile names are accepted
#[tokio::test]
async fn test_common_profile_names() -> Result<()> {
    let runner = CliTestRunner::new()?;

    // Test common profile names with help to verify they're accepted
    let profiles = ["debug", "release", "test", "bench", "custom-profile"];

    for profile in &profiles {
        let output = runner
            .run_command(&["--profile", profile, "--help"])
            .await?;
        assert!(output.success(), "Should accept profile name: {}", profile);
    }

    Ok(())
}
