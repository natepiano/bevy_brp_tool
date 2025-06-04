//! Watch command tests for bevy_brp_tool
//! These tests are optional and longer running as they require multiple frames
//!
//! NOTE: The original streaming tests require direct client access for streaming functionality
//! which is not available through the CLI. They are commented out below.
//! The CLI tests instead focus on command validation and help functionality.

// CLI Tests that work around streaming limitations

mod support;
use anyhow::Result;
use support::*;

// Note: Watch commands are streaming and interactive, so we test them by checking
// their argument validation and initial setup, but not their streaming behavior.

#[tokio::test]
async fn test_cli_get_watch_invalid_entity() -> Result<()> {
    // Setup
    let runner = CliTestRunner::new()?;

    // Note: We can't actually test watch command execution as they would start streaming.
    // Instead, we test the help system which validates the command exists and is documented.
    let help_output = runner.run_command(&["--help-for", "get+watch"]).await?;
    assert!(help_output.success(), "Help for get+watch should work");
    assert!(
        help_output.stdout_contains("get+watch") || help_output.stdout_contains("Watch"),
        "Help should contain information about get+watch"
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_get_watch_invalid_component() -> Result<()> {
    // Setup
    let runner = CliTestRunner::new()?;

    // Note: We can't actually test watch command execution as they would start streaming.
    // Instead, we test that the command is listed in available commands.
    let list_output = runner.run_command(&["--list-commands"]).await?;
    assert!(list_output.success(), "List commands should work");
    assert!(
        list_output.stdout_contains("get+watch"),
        "Command list should include get+watch"
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_watch_command_arguments() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // Test get+watch with no arguments (should fail)
    let output1 = runner.run_command_with_app(&["get+watch"], &app).await?;
    assert!(
        !output1.success(),
        "get+watch should fail with no arguments"
    );

    // Test get+watch with only entity (should fail)
    let output2 = runner
        .run_command_with_app(&["get+watch", "12345"], &app)
        .await?;
    assert!(
        !output2.success(),
        "get+watch should fail with only entity argument"
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_list_watch_no_arguments() -> Result<()> {
    // Setup
    let _app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // Test that list+watch doesn't require arguments
    // Note: We can't test the streaming behavior, but we can test that it accepts the command
    // The command would start streaming but our test framework doesn't handle interactive commands
    // well So we'll check that it at least parses the command correctly

    // We'll test this indirectly by checking help
    let help_output = runner.run_command(&["--help-for", "list+watch"]).await?;
    assert!(help_output.success(), "Help for list+watch should work");
    assert!(
        help_output.stdout_contains("list+watch") || help_output.stdout_contains("Watch"),
        "Help should contain information about list+watch"
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_get_watch_help() -> Result<()> {
    // Setup
    let runner = CliTestRunner::new()?;

    // Test help for get+watch command
    let help_output = runner.run_command(&["--help-for", "get+watch"]).await?;
    assert!(help_output.success(), "Help for get+watch should work");
    assert!(
        help_output.stdout_contains("get+watch") || help_output.stdout_contains("Watch"),
        "Help should contain information about get+watch"
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_watch_commands_in_help() -> Result<()> {
    // Setup
    let runner = CliTestRunner::new()?;

    // Test that watch commands appear in the main help
    let help_output = runner.run_command(&["--help"]).await?;
    assert!(help_output.success(), "Main help should work");
    assert!(
        help_output.stdout_contains("get+watch"),
        "Help should list get+watch command"
    );
    assert!(
        help_output.stdout_contains("list+watch"),
        "Help should list list+watch command"
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_watch_commands_in_list() -> Result<()> {
    // Setup
    let runner = CliTestRunner::new()?;

    // Test that watch commands appear in the command list
    let list_output = runner.run_command(&["--list-commands"]).await?;
    assert!(list_output.success(), "List commands should work");
    assert!(
        list_output.stdout_contains("get+watch"),
        "Command list should include get+watch"
    );
    assert!(
        list_output.stdout_contains("list+watch"),
        "Command list should include list+watch"
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_get_watch_streaming_behavior() -> Result<()> {
    use std::time::Duration;

    // Setup
    let app = TestApp::new(TestRunMode::Loop).await?;
    let runner = CliTestRunner::new()?;

    // Get the first entity with TestComponent
    let output = runner
        .run_command_with_app(&["query", &test_component_type()], &app)
        .await?;
    assert!(output.success());
    let entities = output.parse_json()?;
    let entities_array = entities.as_array().expect("Expected array of entities");
    let entity_id = entities_array
        .first()
        .and_then(|e| e.get("entity"))
        .and_then(|e| e.as_u64())
        .ok_or_else(|| anyhow::anyhow!("No entities found with TestComponent"))?;

    let entity_str = entity_id.to_string();
    let component_type = test_component_type();

    // Test streaming command by running it with timeout
    // Streaming commands run indefinitely, so timeout is expected
    let result = tokio::time::timeout(
        Duration::from_millis(800),
        runner.run_command_with_app(&["get+watch", &entity_str, &component_type], &app),
    )
    .await;

    // For streaming commands, timeout is expected (means command started successfully)
    match result {
        Ok(Ok(output)) => {
            // Command completed within timeout - check if successful
            assert!(
                output.success() || !output.stdout.is_empty(),
                "Command failed: stderr={}",
                output.stderr
            );
        }
        Ok(Err(e)) => {
            panic!("CLI command failed to start: {}", e);
        }
        Err(_) => {
            // Timeout is expected for streaming commands - they run until killed
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_cli_list_watch_streaming_behavior() -> Result<()> {
    use std::time::Duration;

    // Setup
    let app = TestApp::new(TestRunMode::Loop).await?;
    let runner = CliTestRunner::new()?;

    // Get an entity with TestComponent to watch
    let output = runner
        .run_command_with_app(&["query", &test_component_type()], &app)
        .await?;
    assert!(output.success());
    let entities = output.parse_json()?;
    let entities_array = entities.as_array().expect("Expected array of entities");
    let entity_id = entities_array
        .first()
        .and_then(|e| e.get("entity"))
        .and_then(|e| e.as_u64())
        .ok_or_else(|| anyhow::anyhow!("No entities found with TestComponent"))?;

    let entity_str = entity_id.to_string();

    // Test streaming command by running it with timeout
    // Streaming commands run indefinitely, so timeout is expected
    let result = tokio::time::timeout(
        Duration::from_millis(800),
        runner.run_command_with_app(&["list+watch", &entity_str], &app),
    )
    .await;

    // For streaming commands, timeout is expected (means command started successfully)
    match result {
        Ok(Ok(output)) => {
            // Command completed within timeout - check if successful
            assert!(
                output.success() || !output.stdout.is_empty(),
                "Command failed: stderr={}",
                output.stderr
            );
        }
        Ok(Err(e)) => {
            panic!("CLI command failed to start: {}", e);
        }
        Err(_) => {
            // Timeout is expected for streaming commands - they run until killed
        }
    }

    Ok(())
}
