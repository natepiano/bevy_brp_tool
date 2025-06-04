//! Error handling tests for bevy_brp_tool

mod support;
use anyhow::Result;
use support::*;

// Note: This test requires raw HTTP access which isn't available via CLI
// The CLI validates JSON before sending it, so we can't test
// malformed JSON through the CLI interface.
/*
#[tokio::test]
async fn test_invalid_json_format() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let port = app.port();

    // Create a raw HTTP client to send malformed JSON
    let client = reqwest::Client::new();

    // Execute - send invalid JSON
    let response = client
        .post(format!("http://localhost:{}", port))
        .body("{ invalid json }")
        .header("Content-Type", "application/json")
        .send()
        .await?;

    // Verify we get a parse error response
    let status = response.status();
    let body: serde_json::Value = response.json().await?;

    assert_eq!(status, 200); // JSON-RPC typically returns 200 even for errors
    assert!(body.get("error").is_some(), "Should have error field");

    let error = body.get("error").unwrap();
    assert!(error.get("code").is_some());
    assert!(error.get("message").is_some());

    Ok(())
}
*/

// Note: This test requires raw RPC access which isn't available via CLI
// The CLI validates commands before sending them, so we can't test
// arbitrary method calls through it.
/*
#[tokio::test]
async fn test_method_not_found() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let mut client = app.client();

    // Execute - call non-existent method
    let result = client.call_brp_method("non_existent_method", json!({})).await;

    // Verify it fails with method not found
    assert!(result.is_err(), "Should fail with method not found");
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("not found") || error_msg.contains("unknown"),
        "Error should indicate method not found"
    );

    Ok(())
}
*/

// Note: test_server_unreachable has been moved to error_tests_cli.rs as test_cli_connection_failure

#[tokio::test]
async fn test_cli_invalid_port() -> Result<()> {
    // Setup
    let runner = CliTestRunner::new()?;

    // Execute with invalid port
    let output = runner.run_command(&["--port", "99999", "ready"]).await?;

    // Verify
    assert!(!output.success(), "Command should fail with invalid port");

    Ok(())
}

#[tokio::test]
async fn test_cli_connection_failure() -> Result<()> {
    // Setup
    let runner = CliTestRunner::new()?;

    // Execute with a port that has no app running
    let output = runner.run_command(&["--port", "65432", "ready"]).await?;

    // Verify
    assert!(
        !output.success(),
        "Command should fail when no app is running"
    );
    assert!(
        output.stderr_contains("No app is running")
            || output.stdout_contains("No app is running")
            || output.stderr_contains("Connection")
            || output.stderr_contains("connection"),
        "Should indicate connection failure"
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_invalid_command() -> Result<()> {
    // Setup
    let runner = CliTestRunner::new()?;

    // Execute
    let output = runner.run_command(&["completely-invalid-command"]).await?;

    // Verify
    assert!(!output.success(), "Invalid command should fail");

    Ok(())
}

#[tokio::test]
async fn test_cli_missing_arguments() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // Test missing arguments for get command
    let output = runner.run_command_with_app(&["get"], &app).await?;
    assert!(!output.success(), "get with no arguments should fail");

    // Test missing arguments for spawn command
    let output2 = runner.run_command_with_app(&["spawn"], &app).await?;
    assert!(!output2.success(), "spawn with no arguments should fail");

    Ok(())
}

#[tokio::test]
async fn test_cli_invalid_json() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // Execute spawn with invalid JSON
    let output = runner
        .run_command_with_app(&["spawn", "{invalid json syntax"], &app)
        .await?;

    // Verify
    assert!(!output.success(), "spawn with invalid JSON should fail");

    Ok(())
}

#[tokio::test]
async fn test_cli_help_for_invalid_command() -> Result<()> {
    // Setup
    let runner = CliTestRunner::new()?;

    // Execute
    let output = runner
        .run_command(&["--help-for", "nonexistent-command"])
        .await?;

    // Verify - should succeed with "Unknown command" message
    assert!(output.success(), "Help for invalid command should succeed");
    assert!(
        output.stdout_contains("Unknown command") || output.stdout_contains("not found"),
        "Should indicate command not found"
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_empty_command() -> Result<()> {
    // Setup
    let runner = CliTestRunner::new()?;

    // Execute with no command
    let output = runner.run_command(&[]).await?;

    // Verify
    assert!(!output.success(), "Empty command should fail");
    assert!(
        output.stderr_contains("No command")
            || output.stderr_contains("required")
            || output.stderr_contains("missing")
            || output.stderr_contains("Usage:"),
        "Should indicate missing command"
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_invalid_entity_id() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // Execute with invalid entity ID format
    let output = runner
        .run_command_with_app(&["get", "not-a-number", &test_component_type()], &app)
        .await?;

    // Verify
    assert!(!output.success(), "get with invalid entity ID should fail");

    Ok(())
}

#[tokio::test]
async fn test_cli_concurrent_commands() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;

    // Execute multiple commands concurrently to test that there are no issues
    let handles = (0..3)
        .map(|_| {
            let runner = CliTestRunner::new().unwrap();
            let app_port = app.port();
            tokio::spawn(async move {
                runner
                    .run_command(&["--port", &app_port.to_string(), "ready"])
                    .await
            })
        })
        .collect::<Vec<_>>();

    // Wait for all commands to complete
    let mut results = Vec::new();
    for handle in handles {
        results.push(handle.await);
    }

    // Verify all succeeded
    for result in results {
        let output = result??;
        assert!(output.success(), "Concurrent ready commands should succeed");
    }

    Ok(())
}
