//! Special command tests for bevy_brp_tool

mod support;
use anyhow::Result;
use serde_json::json;
use support::*;

#[tokio::test]
async fn test_list_entities_detailed() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // This test verifies that we can still get entity information using standard BRP,
    // but with some limitations compared to the old custom endpoint.

    // Note: BRP doesn't provide a direct way to list all entities with their component types.
    // We need to use specific queries to get entities with known component types.

    // Test 1: Query entities with TestComponent
    let output = runner
        .run_command_with_app(&["query", &test_component_type()], &app)
        .await?;
    assert!(output.success());
    let test_entities = output.parse_json()?;
    let test_array = test_entities.as_array().expect("Expected array");
    assert!(
        test_array.len() >= 3,
        "Should have at least 3 entities with TestComponent"
    );

    // Test 2: Query entities with SecondaryComponent
    let output = runner
        .run_command_with_app(&["query", &secondary_component_type()], &app)
        .await?;
    assert!(output.success());
    let secondary_entities = output.parse_json()?;
    let secondary_array = secondary_entities.as_array().expect("Expected array");
    assert!(
        secondary_array.len() >= 2,
        "Should have at least 2 entities with SecondaryComponent"
    );

    // Test 3: Query entities with both TestComponent and SecondaryComponent
    let output = runner
        .run_command_with_app(
            &["query", &test_component_type(), &secondary_component_type()],
            &app,
        )
        .await?;
    assert!(output.success());
    let both_entities = output.parse_json()?;
    let both_array = both_entities.as_array().expect("Expected array");
    assert!(
        !both_array.is_empty(),
        "Should have at least 1 entity with both components"
    );

    // Test 4: For a specific entity, we can get its known components
    if let Some(first_entity) = test_array.first() {
        if let Some(entity_id) = first_entity.get("entity").and_then(|e| e.as_u64()) {
            // Get TestComponent data
            let output = runner
                .run_command_with_app(
                    &["get", &entity_id.to_string(), &test_component_type()],
                    &app,
                )
                .await?;
            assert!(output.success());
            let component_result = output.parse_json()?;
            // CLI returns component data directly
            assert!(component_result.get("value").is_some());

            // Check if it also has SecondaryComponent
            let output = runner
                .run_command_with_app(
                    &["get", &entity_id.to_string(), &secondary_component_type()],
                    &app,
                )
                .await?;
            if output.success() {
                let result = output.parse_json();
                // This should fail for most entities that only have TestComponent
                if let Ok(data) = result {
                    // If no error, this entity has both components
                    if data.get("errors").is_none() {
                        assert!(data.get("data").is_some());
                    }
                }
            }
        }
    }

    // Note: Without the custom endpoint, we can't easily:
    // 1. List ALL entities regardless of components
    // 2. Get a list of component types for an unknown entity
    // 3. Discover what components an entity has without querying each type
    //
    // However, for most use cases, querying by known component types is sufficient.

    Ok(())
}

#[tokio::test]
async fn test_schema_basic() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // Execute - get schema without filters
    let output = runner.run_command_with_app(&["schema"], &app).await?;
    assert!(output.success());
    let response = output.parse_json()?;

    // Verify we get schema information - the schema is a flat object with type names as keys
    // Look for our registered test types
    assert!(
        response.as_object().is_some() && !response.as_object().unwrap().is_empty(),
        "Schema should return type definitions"
    );

    // Verify that our test types are present
    assert!(
        response.get(test_component_type()).is_some(),
        "Schema should include registered TestComponent"
    );
    assert!(
        response.get(test_resource_type()).is_some(),
        "Schema should include registered TestResource"
    );

    Ok(())
}

#[tokio::test]
async fn test_schema_filtered() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // Get unfiltered schema for comparison
    let output = runner.run_command_with_app(&["schema"], &app).await?;
    assert!(output.success());
    let unfiltered = output.parse_json()?;
    let unfiltered_count = unfiltered.as_object().unwrap().len();

    // Test 1: Include filter - only types from bevy_brp_tool crate
    let output = runner
        .run_command_with_app(&["schema", "--with-crates", "bevy_brp_tool"], &app)
        .await?;
    assert!(output.success());
    let with_crates_response = output.parse_json()?;

    let with_crates_count = with_crates_response.as_object().unwrap().len();

    // Verify with_crates filter reduces the number of types
    assert!(
        with_crates_count < unfiltered_count,
        "with_crates filter should reduce type count from {} to fewer, got {}",
        unfiltered_count,
        with_crates_count
    );

    // The with_crates filter for "bevy_brp_tool" should include basic types
    // that don't have explicit crate prefixes
    assert!(
        with_crates_response.get("()").is_some(),
        "Basic types should be included in bevy_brp_tool filter"
    );

    // It should NOT include bevy-specific types
    assert!(
        with_crates_response
            .get("bevy_ecs::entity::Entity")
            .is_none(),
        "bevy_ecs types should not be included when filtering for bevy_brp_tool"
    );

    // Test 2: Exclude filter - exclude bevy_ecs crate types
    let output = runner
        .run_command_with_app(&["schema", "--without-crates", "bevy_ecs"], &app)
        .await?;
    assert!(output.success());
    let without_crates_response = output.parse_json()?;

    let without_crates_count = without_crates_response.as_object().unwrap().len();

    // Verify without_crates filter reduces the number of types
    assert!(
        without_crates_count < unfiltered_count,
        "without_crates filter should reduce type count from {} to fewer, got {}",
        unfiltered_count,
        without_crates_count
    );

    // Verify that bevy_ecs types are excluded
    assert!(
        without_crates_response
            .get("bevy_ecs::entity::Entity")
            .is_none(),
        "bevy_ecs::entity::Entity should be excluded when filtering out bevy_ecs crate"
    );
    assert!(
        without_crates_response
            .get("bevy_ecs::hierarchy::ChildOf")
            .is_none(),
        "bevy_ecs::hierarchy::ChildOf should be excluded when filtering out bevy_ecs crate"
    );

    // Our test types should still be present when excluding bevy_ecs
    assert!(
        without_crates_response.get(test_component_type()).is_some(),
        "TestComponent should still be present when excluding bevy_ecs crate"
    );

    Ok(())
}

#[tokio::test]
async fn test_shutdown() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Loop).await?; // Use Loop mode so it stays running
    let runner = CliTestRunner::new()?;

    // Execute
    let output = runner.run_command_with_app(&["shutdown"], &app).await?;
    assert!(output.success());
    let response = output.parse_json()?;

    // Verify response
    assert_eq!(
        response.get("success").and_then(|v| v.as_bool()),
        Some(true)
    );
    assert_eq!(
        response.get("message").and_then(|v| v.as_str()),
        Some("Shutdown initiated")
    );

    // Give the app a moment to actually shut down
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Verify app is no longer responsive
    let output = runner.run_command_with_app(&["ready"], &app).await?;
    assert!(
        !output.success(),
        "App should no longer be responsive after shutdown"
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_methods() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // Execute
    let output = runner.run_command_with_app(&["methods"], &app).await?;

    // Verify
    assert!(output.success(), "methods command should succeed");

    let json = output.parse_json()?;
    let methods = json
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
async fn test_cli_schema_basic() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // Execute
    let output = runner.run_command_with_app(&["schema"], &app).await?;

    // Verify
    assert!(output.success(), "schema command should succeed");

    let json = output.parse_json()?;
    assert!(json.is_object(), "Schema response should be an object");

    // Should have schemas for our test types
    let schemas = json.as_object().expect("Expected object");
    assert!(!schemas.is_empty(), "Should have schema data");

    Ok(())
}

#[tokio::test]
async fn test_cli_schema_with_filters() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // Execute with crate filter
    let output = runner
        .run_command_with_app(&["schema", "--with-crates", "bevy_brp_tool"], &app)
        .await?;

    // Verify
    assert!(output.success(), "schema with crate filter should succeed");

    let json = output.parse_json()?;
    assert!(json.is_object(), "Schema response should be an object");

    Ok(())
}

#[tokio::test]
async fn test_cli_raw_bevy_list() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // Execute
    let output = runner
        .run_command_with_app(&["raw", "bevy/list"], &app)
        .await?;

    // Verify
    assert!(output.success(), "raw bevy/list should succeed");

    let json = output.parse_json()?;
    let component_types = json.as_array().expect("Expected array of component types");
    assert!(!component_types.is_empty(), "Should have component types");

    Ok(())
}

#[tokio::test]
async fn test_cli_raw_rpc_discover() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // Execute
    let output = runner
        .run_command_with_app(&["raw", "rpc.discover"], &app)
        .await?;

    // Verify
    assert!(output.success(), "raw rpc.discover should succeed");

    let json = output.parse_json()?;
    let methods = json
        .get("methods")
        .and_then(|m| m.as_array())
        .expect("Expected methods array");
    assert!(!methods.is_empty(), "Should have methods");

    Ok(())
}

#[tokio::test]
async fn test_cli_raw_with_parameters() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // First spawn an entity to query
    let components_json = json!({
        &test_component_type(): {
            "value": 777,
            "name": "RawTestEntity",
            "enabled": true
        }
    });

    let spawn_output = runner
        .run_command_with_app(&["spawn", &components_json.to_string()], &app)
        .await?;
    assert!(spawn_output.success(), "spawn should succeed");

    // Execute raw query command
    let query_params = json!({
        "data": {
            "components": [&test_component_type()]
        }
    });

    let output = runner
        .run_command_with_app(&["raw", "bevy/query", &query_params.to_string()], &app)
        .await?;

    // Verify
    assert!(output.success(), "raw bevy/query should succeed");

    let json = output.parse_json()?;
    let entities = json.as_array().expect("Expected array of entities");
    assert!(!entities.is_empty(), "Should have found entities");

    Ok(())
}

#[tokio::test]
async fn test_cli_raw_invalid_method() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // Execute
    let output = runner
        .run_command_with_app(&["raw", "nonexistent/method"], &app)
        .await?;

    // Verify
    assert!(!output.success(), "raw with invalid method should fail");

    Ok(())
}

#[tokio::test]
async fn test_cli_raw_no_arguments() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // Execute
    let output = runner.run_command_with_app(&["raw"], &app).await?;

    // Verify
    assert!(!output.success(), "raw with no arguments should fail");
    assert!(
        output.stderr_contains("required arguments")
            || output.stderr_contains("missing")
            || output.stderr_contains("<ARGS>"),
        "Should indicate missing arguments"
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_ready() -> Result<()> {
    // Setup
    let app = TestApp::new(TestRunMode::Once).await?;
    let runner = CliTestRunner::new()?;

    // Execute
    let output = runner.run_command_with_app(&["ready"], &app).await?;

    // Verify
    assert!(output.success(), "ready command should succeed");

    let json = output.parse_json()?;
    assert_eq!(json["ready"], true);
    assert!(json["message"].as_str().unwrap().contains("ready"));

    Ok(())
}

#[tokio::test]
async fn test_cli_methods_help() -> Result<()> {
    // Setup
    let runner = CliTestRunner::new()?;

    // Execute
    let output = runner.run_command(&["--help-for", "methods"]).await?;

    // Verify
    assert!(output.success(), "methods help should work");
    assert!(output.stdout_contains("methods") || output.stdout_contains("Methods"));

    Ok(())
}

#[tokio::test]
async fn test_cli_schema_help() -> Result<()> {
    // Setup
    let runner = CliTestRunner::new()?;

    // Execute
    let output = runner.run_command(&["--help-for", "schema"]).await?;

    // Verify
    assert!(output.success(), "schema help should work");
    assert!(output.stdout_contains("schema") || output.stdout_contains("Schema"));

    Ok(())
}

#[tokio::test]
async fn test_cli_raw_help() -> Result<()> {
    // Setup
    let runner = CliTestRunner::new()?;

    // Execute
    let output = runner.run_command(&["--help-for", "raw"]).await?;

    // Verify
    assert!(output.success(), "raw help should work");
    assert!(output.stdout_contains("raw") || output.stdout_contains("Raw"));

    Ok(())
}
