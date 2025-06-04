use std::fmt;
use std::str::FromStr;

use anyhow::Result;

use super::types::Commands;
use crate::cli::constants::BIN_NAME;
use crate::cli::support::parse_entity_arg;

/// Parse a string command into a Commands enum
pub fn parse_command_string(command: &str) -> Result<Commands> {
    command.parse()
}

/// Convert a command enum to a string for managed mode
pub fn format_command(cmd: Commands) -> String {
    cmd.to_string()
}

/// Extract command name from clap error message
pub fn extract_command_from_error(error_msg: &str) -> Option<String> {
    // Look for "Usage: <bin_name> <command>" pattern
    let usage_pattern = format!("Usage: {} ", BIN_NAME);
    if let Some(usage_start) = error_msg.find(&usage_pattern) {
        let usage_line = &error_msg[usage_start + usage_pattern.len()..]; // Skip "Usage: <bin_name> "
        if let Some(space_pos) = usage_line.find(' ') {
            let command = &usage_line[..space_pos];
            return Some(command.to_string());
        } else if let Some(newline_pos) = usage_line.find('\n') {
            let command = &usage_line[..newline_pos];
            return Some(command.to_string());
        }
    }
    None
}

impl fmt::Display for Commands {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Commands::Destroy { entity } => write!(f, "destroy {}", entity),
            Commands::Get { entity, component } => write!(f, "get {} {}", entity, component),
            Commands::GetResource { resource } => write!(f, "get_resource {}", resource),
            Commands::GetWatch { entity, components } => {
                write!(f, "get+watch {} {}", entity, components.join(" "))
            }
            Commands::Insert { entity, components } => {
                write!(f, "insert {} {}", entity, components)
            }
            Commands::InsertResource { data } => write!(f, "insert_resource {}", data),
            Commands::List => write!(f, "list"),
            Commands::ListResources => write!(f, "list_resources"),
            Commands::ListEntities => write!(f, "list_entities"),
            Commands::ListEntity { entity } => write!(f, "list_entity {}", entity),
            Commands::ListWatch { entity } => write!(f, "list+watch {}", entity),
            Commands::Methods => write!(f, "methods"),
            Commands::MutateComponent {
                entity,
                component,
                patch,
            } => {
                write!(f, "mutate_component {} {} {}", entity, component, patch)
            }
            Commands::MutateResource { resource, patch } => {
                write!(f, "mutate_resource {} {}", resource, patch)
            }
            Commands::Query { components } => write!(f, "query {}", components.join(" ")),
            Commands::Ready => write!(f, "ready"),
            Commands::Remove { entity, component } => write!(f, "remove {} {}", entity, component),
            Commands::RemoveResource { resource } => write!(f, "remove_resource {}", resource),
            Commands::Reparent { child, parent } => write!(f, "reparent {} {}", child, parent),
            Commands::Screenshot { path } => write!(f, "screenshot {}", path),
            Commands::Shutdown => write!(f, "shutdown"),
            Commands::Spawn { components } => write!(f, "spawn {}", components),
            Commands::Schema {
                with_crates,
                without_crates,
                with_types,
                without_types,
            } => {
                let mut parts = vec!["schema".to_string()];
                if let Some(crates) = with_crates {
                    parts.push(format!("--with-crates {}", crates.join(" ")));
                }
                if let Some(crates) = without_crates {
                    parts.push(format!("--without-crates {}", crates.join(" ")));
                }
                if let Some(types) = with_types {
                    parts.push(format!("--with-types {}", types.join(" ")));
                }
                if let Some(types) = without_types {
                    parts.push(format!("--without-types {}", types.join(" ")));
                }
                write!(f, "{}", parts.join(" "))
            }
            Commands::Raw { args } => write!(f, "{}", args.join(" ")),
        }
    }
}

impl FromStr for Commands {
    type Err = anyhow::Error;

    fn from_str(command: &str) -> Result<Self> {
        // Helper functions for parsing
        fn validate_arg_count(
            args: &[&str],
            required: usize,
            command_name: &str,
            description: &str,
        ) -> Result<()> {
            if args.len() < required {
                anyhow::bail!("{} requires {}", command_name, description);
            }
            Ok(())
        }

        fn join_args_from(args: &[&str], start_index: usize) -> String {
            args[start_index..].join(" ")
        }

        fn get_arg_string(args: &[&str], index: usize) -> String {
            args[index].to_string()
        }

        fn args_to_strings(args: &[&str]) -> Vec<String> {
            args.iter().map(|s| s.to_string()).collect()
        }

        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            anyhow::bail!("Empty command");
        }

        let cmd_name = parts[0];
        let args = &parts[1..];

        match cmd_name {
            "destroy" => {
                validate_arg_count(args, 1, "destroy", "entity ID")?;
                Ok(Commands::Destroy {
                    entity: parse_entity_arg(args)?,
                })
            }
            "get" => {
                validate_arg_count(args, 2, "get", "entity ID and component name")?;
                Ok(Commands::Get {
                    entity: parse_entity_arg(args)?,
                    component: get_arg_string(args, 1),
                })
            }
            "get_resource" => {
                validate_arg_count(args, 1, "get_resource", "resource name")?;
                Ok(Commands::GetResource {
                    resource: join_args_from(args, 0),
                })
            }
            "get+watch" => {
                validate_arg_count(
                    args,
                    2,
                    "get+watch",
                    "entity ID and at least one component name",
                )?;
                Ok(Commands::GetWatch {
                    entity: parse_entity_arg(args)?,
                    components: args_to_strings(&args[1..]),
                })
            }
            "insert" => {
                validate_arg_count(args, 2, "insert", "entity ID and JSON object")?;
                Ok(Commands::Insert {
                    entity: parse_entity_arg(args)?,
                    components: join_args_from(args, 1),
                })
            }
            "insert_resource" => {
                validate_arg_count(args, 1, "insert_resource", "JSON object with resource data")?;
                Ok(Commands::InsertResource {
                    data: join_args_from(args, 0),
                })
            }
            "list" => Ok(Commands::List),
            "list_resources" => Ok(Commands::ListResources),
            "list_entities" => Ok(Commands::ListEntities),
            "list_entity" => {
                validate_arg_count(args, 1, "list_entity", "entity ID")?;
                Ok(Commands::ListEntity {
                    entity: parse_entity_arg(args)?,
                })
            }
            "list+watch" => {
                validate_arg_count(args, 1, "list+watch", "entity ID")?;
                Ok(Commands::ListWatch {
                    entity: parse_entity_arg(args)?,
                })
            }
            "methods" => Ok(Commands::Methods),
            "mutate_component" => {
                validate_arg_count(
                    args,
                    3,
                    "mutate_component",
                    "entity ID, component name, and JSON patch",
                )?;
                Ok(Commands::MutateComponent {
                    entity: parse_entity_arg(args)?,
                    component: get_arg_string(args, 1),
                    patch: join_args_from(args, 2),
                })
            }
            "mutate_resource" => {
                validate_arg_count(args, 2, "mutate_resource", "resource name and JSON patch")?;
                Ok(Commands::MutateResource {
                    resource: get_arg_string(args, 0),
                    patch: join_args_from(args, 1),
                })
            }
            "query" => {
                validate_arg_count(args, 1, "query", "at least one component name")?;
                Ok(Commands::Query {
                    components: args_to_strings(args),
                })
            }
            "ready" => Ok(Commands::Ready),
            "remove" => {
                validate_arg_count(args, 2, "remove", "entity ID and component name")?;
                Ok(Commands::Remove {
                    entity: parse_entity_arg(args)?,
                    component: get_arg_string(args, 1),
                })
            }
            "remove_resource" => {
                validate_arg_count(args, 1, "remove_resource", "resource name")?;
                Ok(Commands::RemoveResource {
                    resource: join_args_from(args, 0),
                })
            }
            "reparent" => {
                validate_arg_count(args, 2, "reparent", "child ID and parent ID (or 'null')")?;
                Ok(Commands::Reparent {
                    child: parse_entity_arg(args)?,
                    parent: get_arg_string(args, 1),
                })
            }
            "screenshot" => {
                validate_arg_count(args, 1, "screenshot", "file path")?;
                Ok(Commands::Screenshot {
                    path: join_args_from(args, 0),
                })
            }
            "shutdown" => Ok(Commands::Shutdown),
            "spawn" => {
                validate_arg_count(args, 1, "spawn", "JSON object with component data")?;
                Ok(Commands::Spawn {
                    components: join_args_from(args, 0),
                })
            }
            "schema" => {
                // Parse schema flags
                let mut with_crates = None;
                let mut without_crates = None;
                let mut with_types = None;
                let mut without_types = None;

                let mut i = 0;
                while i < args.len() {
                    match args[i] {
                        "--with-crates" => {
                            if i + 1 < args.len() {
                                let values: Vec<String> = args[i + 1..]
                                    .iter()
                                    .take_while(|&arg| !arg.starts_with("--"))
                                    .map(|s| s.to_string())
                                    .collect();
                                if !values.is_empty() {
                                    i += values.len();
                                    with_crates = Some(values);
                                }
                            }
                            i += 1;
                        }
                        "--without-crates" => {
                            if i + 1 < args.len() {
                                let values: Vec<String> = args[i + 1..]
                                    .iter()
                                    .take_while(|&arg| !arg.starts_with("--"))
                                    .map(|s| s.to_string())
                                    .collect();
                                if !values.is_empty() {
                                    i += values.len();
                                    without_crates = Some(values);
                                }
                            }
                            i += 1;
                        }
                        "--with-types" => {
                            if i + 1 < args.len() {
                                let values: Vec<String> = args[i + 1..]
                                    .iter()
                                    .take_while(|&arg| !arg.starts_with("--"))
                                    .map(|s| s.to_string())
                                    .collect();
                                if !values.is_empty() {
                                    i += values.len();
                                    with_types = Some(values);
                                }
                            }
                            i += 1;
                        }
                        "--without-types" => {
                            if i + 1 < args.len() {
                                let values: Vec<String> = args[i + 1..]
                                    .iter()
                                    .take_while(|&arg| !arg.starts_with("--"))
                                    .map(|s| s.to_string())
                                    .collect();
                                if !values.is_empty() {
                                    i += values.len();
                                    without_types = Some(values);
                                }
                            }
                            i += 1;
                        }
                        _ => {
                            i += 1;
                        }
                    }
                }

                Ok(Commands::Schema {
                    with_crates,
                    without_crates,
                    with_types,
                    without_types,
                })
            }
            "raw" => {
                validate_arg_count(args, 1, "raw", "at least one command argument")?;
                Ok(Commands::Raw {
                    args: args_to_strings(args),
                })
            }
            _ => {
                // Unknown command
                anyhow::bail!(
                    "Unknown command: '{}'. Use 'raw {}' for direct BRP method calls",
                    cmd_name,
                    parts.join(" ")
                )
            }
        }
    }
}
