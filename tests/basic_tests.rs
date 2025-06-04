//! Basic connectivity and discovery tests for bevy_brp_tool

mod support;
use anyhow::Result;
use support::*;

#[tokio::test]
async fn test_server_ready() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // Execute - using the BRP-based ready check
    let output = runner.run_command_with_app(&["ready"], &app).await?;

    // Verify
    assert!(output.success(), "Ready command should succeed");
    let json = output.parse_json()?;
    assert_eq!(json["ready"], true);

    Ok(())
}

#[tokio::test]
async fn test_methods_discovery() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // Execute
    let output = runner.run_command_with_app(&["methods"], &app).await?;

    // Verify
    assert!(output.success(), "Methods command should succeed");
    let response = output.parse_json()?;

    // Verify we have methods
    let methods = response
        .get("methods")
        .and_then(|m| m.as_array())
        .expect("Expected methods array");

    assert!(!methods.is_empty(), "Should have discovered methods");

    // Check for expected standard Bevy methods
    let method_names: Vec<&str> = methods
        .iter()
        .filter_map(|m| m.get("name").and_then(|n| n.as_str()))
        .collect();

    // Standard Bevy methods
    assert!(method_names.contains(&"bevy/query"));
    assert!(method_names.contains(&"bevy/get"));
    assert!(method_names.contains(&"bevy/list"));
    assert!(method_names.contains(&"bevy/spawn"));
    assert!(method_names.contains(&"bevy/destroy"));
    assert!(method_names.contains(&"bevy/insert"));
    assert!(method_names.contains(&"bevy/remove"));

    // BRP Tool methods
    assert!(method_names.contains(&"brp_tool/screenshot"));
    assert!(method_names.contains(&"brp_tool/shutdown"));

    Ok(())
}

#[tokio::test]
async fn test_cli_help() -> Result<()> {
    // Setup
    let runner = CliTestRunner::new()?;

    // Execute
    let output = runner.run_command(&["--help"]).await?;

    // Verify
    assert!(output.success(), "Help command should succeed");
    assert!(output.stdout_contains("controls running Bevy apps remotely"));
    assert!(output.stdout_contains("--port"));
    assert!(output.stdout_contains("--managed"));

    Ok(())
}

#[tokio::test]
async fn test_cli_version_help() -> Result<()> {
    // Setup
    let runner = CliTestRunner::new()?;

    // Execute - since --version doesn't exist, test the header in --help instead
    let output = runner.run_command(&["--help"]).await?;

    // Verify
    assert!(output.success(), "Help command should succeed");
    // The help output should contain the binary name
    assert!(output.stdout_contains("brp"));

    Ok(())
}

#[tokio::test]
async fn test_cli_list_commands() -> Result<()> {
    // Setup
    let runner = CliTestRunner::new()?;

    // Execute
    let output = runner.run_command(&["--list-commands"]).await?;

    // Verify
    assert!(output.success(), "List commands should succeed");
    assert!(output.stdout_contains("destroy"));
    assert!(output.stdout_contains("get"));
    assert!(output.stdout_contains("ready"));
    assert!(output.stdout_contains("spawn"));

    Ok(())
}

#[tokio::test]
async fn test_cli_help_for_specific_command() -> Result<()> {
    // Setup
    let runner = CliTestRunner::new()?;

    // Execute
    let output = runner.run_command(&["--help-for", "ready"]).await?;

    // Verify
    assert!(output.success(), "Help for specific command should succeed");
    assert!(output.stdout_contains("ready") || output.stdout_contains("Ready"));

    Ok(())
}

#[tokio::test]
async fn test_cli_ready_command_without_app() -> Result<()> {
    // Setup
    let runner = CliTestRunner::new()?;

    // Execute - use a valid port that likely has no app running
    let output = runner.run_command(&["--port", "65432", "ready"]).await?;

    // Verify - this should fail since no app is running
    assert!(
        !output.success(),
        "Ready command should fail when no app is running"
    );
    assert!(
        output.stderr_contains("No app is running") || output.stdout_contains("No app is running")
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_ready_command_with_app() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // Execute
    let output = runner.run_command_with_app(&["ready"], &app).await?;

    // Verify
    assert!(
        output.success(),
        "Ready command should succeed with running app"
    );

    // Parse the JSON response
    let json = output.parse_json()?;
    assert_eq!(json["ready"], true);
    assert!(json["message"].as_str().unwrap().contains("ready"));

    Ok(())
}

#[tokio::test]
async fn test_cli_invalid_command() -> Result<()> {
    // Setup
    let runner = CliTestRunner::new()?;

    // Execute
    let output = runner.run_command(&["invalid-command"]).await?;

    // Verify
    assert!(!output.success(), "Invalid command should fail");

    Ok(())
}

#[tokio::test]
async fn test_cli_no_command_standalone() -> Result<()> {
    // Setup
    let runner = CliTestRunner::new()?;

    // Execute
    let output = runner.run_command(&[]).await?;

    // Verify
    assert!(
        !output.success(),
        "No command should fail in standalone mode"
    );
    assert!(
        output.stderr_contains("No command specified")
            || output.stdout_contains("No command specified")
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_brp_info() -> Result<()> {
    // Setup
    let runner = CliTestRunner::new()?;

    // Execute
    let output = runner.run_command(&["--brp"]).await?;

    // Verify
    assert!(output.success(), "BRP info command should succeed");
    assert!(output.stdout_contains("BRP") || output.stdout_contains("protocol"));

    Ok(())
}

#[tokio::test]
async fn test_cli_workflows() -> Result<()> {
    // Setup
    let runner = CliTestRunner::new()?;

    // Execute
    let output = runner.run_command(&["--workflows"]).await?;

    // Verify
    assert!(output.success(), "Workflows command should succeed");
    // Should contain workflow examples or help text
    assert!(
        !output.stdout.is_empty(),
        "Workflows output should not be empty"
    );

    Ok(())
}
