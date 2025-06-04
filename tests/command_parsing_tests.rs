//! Tests for command parsing and formatting round-trip consistency

use anyhow::Result;
use bevy_brp_tool::cli::commands::{Commands, format_command, parse_command_string};

/// Test that all commands can be formatted and then parsed back to the same value
#[test]
fn test_round_trip_consistency() -> Result<()> {
    let test_commands = vec![
        Commands::Destroy { entity: 12345 },
        Commands::Get {
            entity:    12345,
            component: "bevy_transform::components::transform::Transform".to_string(),
        },
        Commands::GetResource {
            resource: "bevy_time::time::Time".to_string(),
        },
        Commands::GetWatch {
            entity:     12345,
            components: vec![
                "bevy_transform::components::transform::Transform".to_string(),
                "bevy_core::name::Name".to_string(),
            ],
        },
        Commands::Insert {
            entity:     12345,
            components: r#"{"bevy_core::name::Name": "TestEntity"}"#.to_string(),
        },
        Commands::InsertResource {
            data: r#"{"my_game::GameSettings": {"difficulty": "hard"}}"#.to_string(),
        },
        Commands::List,
        Commands::ListResources,
        Commands::ListEntities,
        Commands::ListEntity { entity: 12345 },
        Commands::ListWatch { entity: 12345 },
        Commands::Methods,
        Commands::MutateComponent {
            entity:    12345,
            component: "bevy_transform::components::transform::Transform".to_string(),
            patch:     r#"{"translation": [10.0, 0.0, 0.0]}"#.to_string(),
        },
        Commands::MutateResource {
            resource: "my_game::GameSettings".to_string(),
            patch:    r#"{"difficulty": "easy"}"#.to_string(),
        },
        Commands::Query {
            components: vec![
                "bevy_transform::components::transform::Transform".to_string(),
                "bevy_core::name::Name".to_string(),
            ],
        },
        Commands::Ready,
        Commands::Remove {
            entity:    12345,
            component: "bevy_core::name::Name".to_string(),
        },
        Commands::RemoveResource {
            resource: "my_game::GameSettings".to_string(),
        },
        Commands::Reparent {
            child:  12345,
            parent: "67890".to_string(),
        },
        Commands::Screenshot {
            path: "./screenshot.png".to_string(),
        },
        Commands::Shutdown,
        Commands::Spawn {
            components: r#"{"bevy_transform::components::transform::Transform": {"translation": [0, 0, 0]}}"#.to_string(),
        },
        Commands::Schema {
            with_crates:    Some(vec!["bevy".to_string()]),
            without_crates: None,
            with_types:     None,
            without_types:  Some(vec!["Component".to_string()]),
        },
        // Note: Raw commands are excluded from round-trip testing
        // because they have special parsing semantics and don't follow normal
        // command parsing rules
    ];

    for cmd in test_commands {
        // Format the command to string
        let formatted = format_command(cmd.clone());

        // Parse it back to a command
        let parsed = parse_command_string(&formatted)?;

        // Check that they match
        assert_eq!(cmd, parsed, "Round-trip failed for command: {}", formatted);
    }

    Ok(())
}

/// Test specific edge cases that were previously causing issues
#[test]
fn test_list_entity_round_trip() -> Result<()> {
    let cmd = Commands::ListEntity { entity: 42 };
    let formatted = format_command(cmd.clone());
    let parsed = parse_command_string(&formatted)?;

    assert_eq!(cmd, parsed);
    assert_eq!(formatted, "list_entity 42");

    Ok(())
}

/// Test that formatting uses Display trait
#[test]
fn test_format_uses_display_trait() {
    let cmd = Commands::ListEntity { entity: 42 };
    let formatted_direct = cmd.to_string();
    let formatted_via_function = format_command(cmd);

    assert_eq!(formatted_direct, formatted_via_function);
}

/// Test that parsing uses FromStr trait
#[test]
fn test_parse_uses_fromstr_trait() -> Result<()> {
    let command_str = "list_entity 42";
    let parsed_direct: Commands = command_str.parse()?;
    let parsed_via_function = parse_command_string(command_str)?;

    assert_eq!(parsed_direct, parsed_via_function);

    Ok(())
}
