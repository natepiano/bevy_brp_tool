use clap::CommandFactory;

use crate::cli::commands::Cli;
use crate::cli::constants::{ENTITY_ID_EXAMPLE, ENTITY_ID_TYPE};

/// Display enhanced error messages for missing arguments
pub fn display_missing_args_error(command_name: &str, missing_args: &[(String, String, String)]) {
    eprintln!("error: missing required arguments");
    eprintln!();
    eprintln!("Required arguments:");

    // Calculate column widths
    let max_name_width = missing_args
        .iter()
        .map(|(name, _, _)| name.len())
        .max()
        .unwrap_or(10)
        .max(10); // minimum width for "Name" header
    let max_type_width = missing_args
        .iter()
        .map(|(_, arg_type, _)| arg_type.len())
        .max()
        .unwrap_or(8)
        .max(8); // minimum width for "Type" header

    // Header
    eprintln!(
        "  {:<width_name$} {:<width_type$} Example",
        "Name",
        "Type",
        width_name = max_name_width,
        width_type = max_type_width
    );
    eprintln!(
        "  {:<width_name$} {:<width_type$} {}",
        "-".repeat(max_name_width),
        "-".repeat(max_type_width),
        "-".repeat(30),
        width_name = max_name_width,
        width_type = max_type_width
    );

    // Arguments
    for (name, arg_type, example) in missing_args {
        eprintln!(
            "  {:<width_name$} {:<width_type$} {}",
            name,
            arg_type,
            example,
            width_name = max_name_width,
            width_type = max_type_width
        );
    }

    eprintln!();

    // Usage line
    let usage_args: Vec<String> = missing_args
        .iter()
        .map(|(name, _, _)| name.clone())
        .collect();
    eprintln!("Usage: {} {}", command_name, usage_args.join(" "));
    eprintln!();
    eprintln!("For more information, try '--help'.");
}

/// Extract argument info from Clap command using introspection
fn get_command_args_from_clap(command_name: &str) -> Vec<(String, String, String)> {
    let cli = Cli::command();

    // Find the subcommand
    if let Some(subcmd) = cli.find_subcommand(command_name) {
        let mut args = Vec::new();

        // Iterate through all arguments of the subcommand
        for arg in subcmd.get_arguments() {
            // Skip help and other built-in flags
            if arg.get_id().as_str() == "help" || arg.get_id().as_str() == "version" {
                continue;
            }

            // Skip non-positional arguments (e.g., --with-crates)
            if !arg.is_positional() {
                continue;
            }

            // Get the display name from value_names or construct from ID
            let name = if let Some(value_names) = arg.get_value_names() {
                if !value_names.is_empty() {
                    // Handle variadic arguments (e.g., COMPONENT_TYPES...)
                    let base_name = &value_names[0];
                    if arg.is_positional()
                        && arg.get_num_args().map(|n| n.max_values()).unwrap_or(1) > 1
                    {
                        format!("<{}>...", base_name)
                    } else {
                        format!("<{}>", base_name)
                    }
                } else {
                    format!("<{}>", arg.get_id().as_str().to_uppercase())
                }
            } else {
                format!("<{}>", arg.get_id().as_str().to_uppercase())
            };

            // Extract type and example from help text
            let (arg_type, example) = extract_type_and_example_from_help(
                arg.get_help().map(|h| h.to_string()).unwrap_or_default(),
                &name,
            );

            args.push((name, arg_type, example));
        }

        args
    } else {
        vec![]
    }
}

/// Helper function to extract type and example from help text
fn extract_type_and_example_from_help(help: String, arg_name: &str) -> (String, String) {
    // Default values
    let mut arg_type = "string".to_string();
    let mut example = String::new();

    // Special handling based on argument name patterns
    match arg_name {
        "<ENTITY_ID>" | "<CHILD_ID>" => {
            arg_type = ENTITY_ID_TYPE.to_string();
            example = ENTITY_ID_EXAMPLE.to_string();
        }
        "<PARENT_ID>" => {
            arg_type = format!("{}|null", ENTITY_ID_TYPE);
            example = "67890 or null".to_string();
        }
        "<JSON>" | "<JSON_PATCH>" => {
            arg_type = "JSON".to_string();
            // Try to extract example from help text
            if let Some(start) = help.find("(e.g., '") {
                if let Some(end) = help[start + 8..].find("')") {
                    example = help[start + 8..start + 8 + end].to_string();
                }
            } else if let Some(start) = help.find("e.g., '") {
                if let Some(end) = help[start + 7..].find("'") {
                    example = help[start + 7..start + 7 + end].to_string();
                }
            }
        }
        "<FILE_PATH>" => {
            arg_type = "path".to_string();
            example = "./screenshot.png".to_string();
        }
        "<COMPONENT_TYPE>" | "<RESOURCE_TYPE>" => {
            // Extract example from help text
            if let Some(start) = help.find("(e.g., ") {
                if let Some(end) = help[start + 7..].find(')') {
                    example = help[start + 7..start + 7 + end].to_string();
                }
            }
        }
        "<COMPONENT_TYPES>..." => {
            arg_type = "string[]".to_string();
            // Extract example from help text
            if let Some(start) = help.find("(e.g., ") {
                // Find the closing parenthesis, handling multi-line examples
                let remaining = &help[start + 7..];
                if let Some(end) = remaining.find(')') {
                    example = remaining[..end].replace('\n', " ").trim().to_string();
                }
            }
        }
        _ => {
            // Generic extraction from help text
            if let Some(start) = help.find("(e.g., ") {
                if let Some(end) = help[start + 7..].find(')') {
                    example = help[start + 7..start + 7 + end].to_string();
                }
            }
        }
    }

    // Extract type info from help text if present
    if help.contains("u64 integer") {
        arg_type = "u64".to_string();
    } else if help.contains("JSON object") || help.contains("JSON patch") {
        arg_type = "JSON".to_string();
    }

    (arg_type, example)
}

/// Get argument information for specific commands
pub fn get_command_args(command: &str) -> Vec<(String, String, String)> {
    // Use Clap introspection to dynamically get command arguments
    let mut args = get_command_args_from_clap(command);

    // Special handling for get+watch to match legacy behavior
    // The hardcoded version showed singular COMPONENT_TYPE instead of plural
    if command == "get+watch" {
        for (name, _, _) in &mut args {
            if name == "<COMPONENT_TYPES>..." {
                *name = "<COMPONENT_TYPE>".to_string();
            }
        }
    }

    args
}
