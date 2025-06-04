use anyhow::Result;
use strum::IntoEnumIterator;

use super::commands::{CommandTemplate, commands_by_category, find_command_by_name};
use super::constants::{BEVY_COMMAND_PREFIX, BIN_NAME, BRP_TOOL_COMMAND_PREFIX};
use super::{help_builder, support};
use crate::include_help;

/// Get the detected app information with binary path if available
fn get_detected_app_info(profile: Option<&str>) -> Option<(String, Option<std::path::PathBuf>)> {
    help_builder::get_detected_app().map(|app_name| {
        match support::detect_bevy_app(Some(app_name.clone())) {
            Ok((_, _, target_dir)) => {
                match support::find_workspace_binary_with_target_dir(
                    &app_name,
                    &target_dir,
                    profile,
                ) {
                    Ok(binary_path) => (app_name, Some(binary_path)),
                    Err(_) => (app_name, None),
                }
            }
            Err(_) => (app_name, None),
        }
    })
}

/// Replace {{DETECTED_APP}} placeholder in help text files with actual detection results
fn replace_detected_app(text: &str, profile: Option<&str>) -> String {
    if text.contains("{{DETECTED_APP}}") {
        let detected_app_info = match get_detected_app_info(profile) {
            Some((app_name, binary_path)) => match binary_path {
                Some(path) => format!("  Detected app: {} (binary: {})", app_name, path.display()),
                None => format!("  Detected app: {}", app_name),
            },
            None => "  No Bevy app detected in current location".to_string(),
        };
        text.replace("{{DETECTED_APP}}", &detected_app_info)
    } else {
        text.to_string()
    }
}

/// Get all known commands with their full names
fn get_all_known_full_names() -> Vec<&'static str> {
    CommandTemplate::iter()
        .filter_map(|template| template.to_command())
        .map(|cmd| cmd.primary_name())
        .collect()
}

/// Display help for a specific command
pub fn display_command_help(command: &str, profile: Option<&str>) {
    // Special handling for --managed-commands flag
    if command == "managed-commands" || command == "--managed-commands" {
        display_managed_commands_flag_help();
        return;
    }

    // Try to find the command
    match find_command_by_name(command) {
        Some(cmd) => {
            let primary_name = cmd.primary_name();
            // Extract short name (without prefix)
            let short_name = if primary_name == "bevy/registry/schema" {
                "schema"
            } else if let Some(name) = primary_name.strip_prefix(BEVY_COMMAND_PREFIX) {
                name
            } else if let Some(name) = primary_name.strip_prefix(BRP_TOOL_COMMAND_PREFIX) {
                name
            } else {
                primary_name
            };

            println!("Help for command: {}", short_name);
            if primary_name != short_name {
                println!("Bevy Remote Protocol name: {}\n", primary_name);
            } else {
                println!(); // Add blank line for consistency
            }
            let help_text = cmd.detailed_help();
            let help_text = replace_detected_app(&help_text, profile);
            println!("{}", help_text);
        }
        None => {
            println!("Unknown command: {}", command);
            println!("\nAvailable commands:");

            // Show all known commands grouped by category
            let mut all_commands = get_all_known_full_names();

            // Also check if there's a similar command
            let similar = all_commands
                .iter()
                .filter(|&&cmd| {
                    let cmd_lower = cmd.to_lowercase();
                    let search_lower = command.to_lowercase();
                    cmd_lower.contains(&search_lower) || search_lower.contains(&cmd_lower)
                })
                .copied()
                .collect::<Vec<_>>();

            if !similar.is_empty() {
                println!("\nDid you mean one of these?");
                for cmd in similar {
                    // Get the short form if available
                    let short_form = if cmd.contains('/') {
                        cmd.split('/').next_back().unwrap_or(cmd)
                    } else {
                        cmd
                    };
                    if short_form != cmd {
                        println!("  {} (or {})", cmd, short_form);
                    } else {
                        println!("  {}", cmd);
                    }
                }
            }

            all_commands.sort();

            println!("\nBevy Commands:");
            for cmd in &all_commands {
                if cmd.starts_with("bevy/") {
                    let short_form = cmd.strip_prefix("bevy/").unwrap_or(cmd);
                    println!("  {} (or {})", cmd, short_form);
                }
            }

            println!("\nBevy Watch Commands:");
            for cmd in &all_commands {
                if cmd.contains("+watch") {
                    let short_form = if cmd.starts_with("bevy/") {
                        cmd.strip_prefix("bevy/").unwrap_or(cmd)
                    } else {
                        cmd
                    };
                    if short_form != *cmd {
                        println!("  {} (or {})", cmd, short_form);
                    } else {
                        println!("  {}", cmd);
                    }
                }
            }

            println!("\nBRP Tool Commands:");
            for cmd in &all_commands {
                if cmd.starts_with("brp_tool/") {
                    let short_form = cmd.strip_prefix("brp_tool/").unwrap_or(cmd);
                    println!("  {} (or {})", cmd, short_form);
                }
            }

            println!("\nUse --help-for <COMMAND> to get detailed help for a specific command.");
            println!("Use --workflows to see complete workflow examples.");
            println!("Use --agent to see instructions for coding agents.");
            println!("Use --brp to see BRP configuration requirements.");

            println!("\n=== General Tips ===");
            println!("• Always use fully qualified component names (from 'list' command)");
            println!("• Entity IDs are u64 integers from query results");
            println!("• Use --managed-commands for complex commands");
        }
    }
}

/// Display all available commands organized by category
pub fn display_all_commands() {
    println!("======================================");
    println!("BRP TOOL - AVAILABLE COMMANDS");
    println!("======================================\n");

    println!("All commands can be used in direct mode or with --managed-commands.");
    println!("Both full (bevy/list) and short (list) command names work.\n");

    println!("COMMAND           BRP NAME               DESCRIPTION");
    println!("-------           --------               -----------\n");

    for (category, commands) in commands_by_category() {
        if !commands.is_empty() {
            println!("{}:", category);
            for cmd in commands {
                let primary_name = cmd.primary_name();
                let display_name = if primary_name == "bevy/registry/schema" {
                    "schema"
                } else if let Some(name) = primary_name.strip_prefix(BEVY_COMMAND_PREFIX) {
                    name
                } else if let Some(name) = primary_name.strip_prefix(BRP_TOOL_COMMAND_PREFIX) {
                    name
                } else {
                    primary_name
                };

                // Create padded command display
                let padded_display = format!("{:<17}", display_name);

                // Handle commands without a bevy namespace
                let padded_primary = match primary_name {
                    "ready" | "methods" | "list_entities" | "list_entity" | "raw" => {
                        format!("{:<22}", "[composite command]")
                    }
                    _ => format!("{:<22}", primary_name),
                };

                println!(
                    "{} {} {}",
                    padded_display,
                    padded_primary,
                    cmd.brief_description()
                );
            }
            println!(); // Empty line between categories
        }
    }

    println!("EXAMPLES:");
    println!(
        "  {} list                                  # Connect to existing app",
        BIN_NAME
    );
    println!(
        "  {} --managed-commands 'spawn {{}}'         # Start app and run command",
        BIN_NAME
    );
    println!(
        "  {} --help-for spawn                      # Get detailed help for spawn",
        BIN_NAME
    );

    println!(
        "\nFor detailed help on any command, use: {} --help-for <command>",
        BIN_NAME
    );
}

/// Display help for the --managed-commands flag
fn display_managed_commands_flag_help() {
    println!("========================================");
    println!("USING THE --managed-commands FLAG");
    println!("========================================\n");

    println!("The --managed-commands flag starts the app and executes commands directly.\n");

    println!("USAGE:");
    println!(
        "  {} --managed-commands '<command1>,<command2>,...'",
        BIN_NAME
    );
    println!(
        "  {} -m '<command1>,<command2>,...'  # Short form",
        BIN_NAME
    );
    println!();

    println!("BEHAVIOR:");
    println!("  1. Starts the specified app (or auto-detects)");
    println!("  2. Waits for app to be ready");
    println!("  3. Executes all commands in sequence");
    println!("  4. Shuts down the app automatically");
    println!("  5. Returns results to stdout");
    println!();

    println!("EXAMPLES:");
    println!("  # Single command");
    println!("  {} --managed-commands 'list'", BIN_NAME);
    println!();
    println!("  # Multiple commands");
    println!(
        "  {} --managed-commands 'list,query bevy_transform::components::transform::Transform'",
        BIN_NAME
    );
    println!();
    println!("  # With explicit app");
    println!(
        "  {} --app ./my_game --managed-commands 'spawn {{\"bevy_core::name::Name\": \"Test\"}}'",
        BIN_NAME
    );
    println!();
    println!("  # With wait commands");
    println!(
        "  {} --managed-commands 'screenshot /tmp/before.png,wait:2,screenshot /tmp/after.png'",
        BIN_NAME
    );
    println!();

    println!("SPECIAL COMMANDS:");
    println!("  wait:N - Wait for N seconds between commands");
    println!();

    println!("NOTES:");
    println!("  • Commands are separated by commas");
    println!("  • Complex JSON can use shell escaping or heredocs");
    println!("  • App output goes to stderr, command results to stdout");
    println!("  • Exit code 0 on success, non-zero on any failure");
    println!();

    println!("See also: {} --workflows for complete examples", BIN_NAME);
}

/// Display comprehensive workflow examples showing how to chain commands together
pub fn display_workflow_examples() {
    println!("{}", include_help!("workflows"));
}

/// Display instructions for coding agents
pub fn display_agent_instructions() {
    println!("{}", include_help!("agent"));
}

/// Display BRP configuration requirements
pub fn display_brp_configuration() {
    println!("{}", include_help!("brp"));
}

/// Display detected app information
pub fn display_detected_app(profile: Option<&str>) -> Result<()> {
    println!("🔍 Bevy App Detection");
    println!("===================");

    match get_detected_app_info(profile) {
        Some((app_name, binary_path)) => {
            println!("✅ Detected app: {}", app_name);

            match binary_path {
                Some(path) => {
                    println!("📁 Binary path: {}", path.display());
                    println!("\n💡 This app will be used when running:");
                    println!("   brp --detached");
                    println!("   brp --managed-commands '<commands>'");
                }
                None => {
                    println!("⚠️  Binary not found");
                    println!("   The app was detected in Cargo.toml but binary doesn't exist");
                    match profile {
                        Some(p) if p != "debug" => {
                            println!("   Try running: cargo build --profile {}", p)
                        }
                        _ => println!("   Try running: cargo build"),
                    }
                }
            }
        }
        None => {
            println!("❌ No Bevy app detected in current directory");
            println!("\n💡 For auto-detection to work:");
            println!("   1. Run from a Rust project/workspace directory that uses Bevy");
            println!("   2. Ensure a binary target depends on 'bevy'");
            println!("   3. Build the project: cargo build");
            println!("\n🔧 Alternatively, specify the app manually:");
            println!("   brp --app ./target/debug/my_game --detached");
        }
    }

    Ok(())
}
