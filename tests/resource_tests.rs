//! Resource operations CLI tests for bevy_brp_tool

mod support;
use anyhow::Result;
use serde_json::json;
use support::*;

#[tokio::test]
async fn test_cli_list_resources() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // Execute
    let output = runner
        .run_command_with_app(&["list_resources"], &app)
        .await?;

    // Verify
    assert!(output.success(), "list_resources command should succeed");

    let json = output.parse_json()?;
    let resources = json.as_array().expect("Expected array of resources");

    // Should have at least our TestResource and some system resources
    assert!(!resources.is_empty(), "Should have at least 1 resource");

    // Check that our TestResource is present
    let resource_names: Vec<String> = resources
        .iter()
        .filter_map(|r| r.as_str().map(|s| s.to_string()))
        .collect();

    assert!(
        resource_names.contains(&test_resource_type()),
        "Should contain TestResource in list"
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_get_resource() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // Execute
    let output = runner
        .run_command_with_app(&["get_resource", &test_resource_type()], &app)
        .await?;

    // Verify
    assert!(output.success(), "get_resource command should succeed");

    let json = output.parse_json()?;
    let value = json.get("value").expect("Expected value field");

    // The resource should have default values from TestResource::default()
    assert_eq!(value.get("counter").and_then(|v| v.as_u64()), Some(0));
    assert_eq!(value.get("message").and_then(|v| v.as_str()), Some(""));

    Ok(())
}

#[tokio::test]
async fn test_cli_insert_resource() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // Execute - insert/update resource with new values
    let resource_json = json!({
        &test_resource_type(): {
            "counter": 42,
            "message": "Hello from CLI test"
        }
    });

    let output = runner
        .run_command_with_app(&["insert_resource", &resource_json.to_string()], &app)
        .await?;

    // Verify
    assert!(output.success(), "insert_resource command should succeed");

    // Verify resource was updated by getting it
    let get_output = runner
        .run_command_with_app(&["get_resource", &test_resource_type()], &app)
        .await?;

    assert!(
        get_output.success(),
        "get_resource after insert should succeed"
    );
    let get_json = get_output.parse_json()?;
    let value = get_json.get("value").expect("Expected value field");

    assert_eq!(value.get("counter").and_then(|v| v.as_u64()), Some(42));
    assert_eq!(
        value.get("message").and_then(|v| v.as_str()),
        Some("Hello from CLI test")
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_mutate_resource() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // First insert some initial values
    let initial_json = json!({
        &test_resource_type(): {
            "counter": 100,
            "message": "Initial message"
        }
    });

    let insert_output = runner
        .run_command_with_app(&["insert_resource", &initial_json.to_string()], &app)
        .await?;
    assert!(insert_output.success(), "insert_resource should succeed");

    // Execute - mutate only the counter field
    let patch = json!({
        "counter": 200
    });

    let output = runner
        .run_command_with_app(
            &["mutate_resource", &test_resource_type(), &patch.to_string()],
            &app,
        )
        .await?;

    // Verify
    assert!(output.success(), "mutate_resource command should succeed");

    // Verify the changes by getting the resource
    let get_output = runner
        .run_command_with_app(&["get_resource", &test_resource_type()], &app)
        .await?;

    assert!(
        get_output.success(),
        "get_resource after mutate should succeed"
    );
    let get_json = get_output.parse_json()?;
    let value = get_json.get("value").expect("Expected value field");

    // Counter should be updated, message should remain unchanged
    assert_eq!(value.get("counter").and_then(|v| v.as_u64()), Some(200));
    assert_eq!(
        value.get("message").and_then(|v| v.as_str()),
        Some("Initial message")
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_remove_resource() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // First ensure the resource exists by setting it
    let initial_json = json!({
        &test_resource_type(): {
            "counter": 999,
            "message": "To be removed"
        }
    });

    let insert_output = runner
        .run_command_with_app(&["insert_resource", &initial_json.to_string()], &app)
        .await?;
    assert!(insert_output.success(), "insert_resource should succeed");

    // Execute - remove the resource
    let output = runner
        .run_command_with_app(&["remove_resource", &test_resource_type()], &app)
        .await?;

    // Verify
    assert!(output.success(), "remove_resource command should succeed");

    // Verify resource was removed by trying to get it
    let get_output = runner
        .run_command_with_app(&["get_resource", &test_resource_type()], &app)
        .await?;

    // Should fail since resource no longer exists
    assert!(
        !get_output.success(),
        "get_resource after remove should fail"
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_get_resource_nonexistent() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // Execute - try to get a non-existent resource
    let output = runner
        .run_command_with_app(&["get_resource", "NonExistentResource"], &app)
        .await?;

    // Verify
    assert!(
        !output.success(),
        "get_resource on non-existent resource should fail"
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_mutate_resource_nonexistent() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // Execute - try to mutate a non-existent resource
    let patch = json!({
        "counter": 123
    });

    let output = runner
        .run_command_with_app(
            &["mutate_resource", "NonExistentResource", &patch.to_string()],
            &app,
        )
        .await?;

    // Verify
    assert!(
        !output.success(),
        "mutate_resource on non-existent resource should fail"
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_insert_resource_invalid_json() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // Execute - try to insert with invalid JSON
    let output = runner
        .run_command_with_app(&["insert_resource", "{invalid json}"], &app)
        .await?;

    // Verify
    assert!(
        !output.success(),
        "insert_resource with invalid JSON should fail"
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_mutate_resource_partial_update() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // First insert some initial values
    let initial_json = json!({
        &test_resource_type(): {
            "counter": 555,
            "message": "Partial update test"
        }
    });

    let insert_output = runner
        .run_command_with_app(&["insert_resource", &initial_json.to_string()], &app)
        .await?;
    assert!(insert_output.success(), "insert_resource should succeed");

    // Execute - mutate only the message field
    let patch = json!({
        "message": "Updated message only"
    });

    let output = runner
        .run_command_with_app(
            &["mutate_resource", &test_resource_type(), &patch.to_string()],
            &app,
        )
        .await?;

    // Verify
    assert!(output.success(), "mutate_resource command should succeed");

    // Verify the changes by getting the resource
    let get_output = runner
        .run_command_with_app(&["get_resource", &test_resource_type()], &app)
        .await?;

    assert!(
        get_output.success(),
        "get_resource after mutate should succeed"
    );
    let get_json = get_output.parse_json()?;
    let value = get_json.get("value").expect("Expected value field");

    // Counter should remain unchanged, message should be updated
    assert_eq!(value.get("counter").and_then(|v| v.as_u64()), Some(555));
    assert_eq!(
        value.get("message").and_then(|v| v.as_str()),
        Some("Updated message only")
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_insert_resource_unregistered_type() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // Try to insert a resource type that isn't registered at startup
    // This should fail because Bevy requires resources to be registered before use
    let unregistered_resource_type = "bevy_brp_tool::tests::support::UnregisteredResource";

    let resource_json = json!({
        unregistered_resource_type: {
            "value": 123,
            "name": "test"
        }
    });

    // Execute
    let output = runner
        .run_command_with_app(&["insert_resource", &resource_json.to_string()], &app)
        .await?;

    // Verify it fails appropriately - BRP should reject unregistered resource types
    assert!(
        !output.success(),
        "Inserting unregistered resource type should fail"
    );

    // Verify error message indicates resource type is not registered
    assert!(
        output.stderr_contains("not found")
            || output.stderr_contains("unknown")
            || output.stderr_contains("registered")
            || output.stderr_contains("No such resource type"),
        "Error should indicate resource type is not registered. Got: {}",
        output.stderr
    );

    Ok(())
}
