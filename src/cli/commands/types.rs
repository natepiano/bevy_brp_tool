use std::fmt;

use clap::Subcommand;
use strum::{EnumIter, IntoEnumIterator};

use crate::include_help;

/// Metadata for a command including all its descriptive information
pub struct CommandMetadata {
    /// All possible names for this command (primary name first)
    pub names: &'static [&'static str],
    /// Brief one-line description
    pub brief: &'static str,
}

#[derive(Subcommand, Debug, Clone, PartialEq)]
pub enum Commands {
    /// Destroy an entity
    Destroy {
        /// Entity ID to destroy (u64 integer, e.g., 12345)
        #[arg(value_name = "ENTITY_ID")]
        entity: u64,
    },

    /// Get component data for an entity
    Get {
        /// Entity ID (u64 integer, e.g., 12345)
        #[arg(value_name = "ENTITY_ID")]
        entity: u64,
        /// Component type name (e.g., bevy_transform::components::transform::Transform)
        #[arg(value_name = "COMPONENT_TYPE")]
        component: String,
    },

    /// Get resource data
    #[command(name = "get_resource")]
    GetResource {
        /// Resource type name (e.g., bevy_time::time::Time)
        #[arg(value_name = "RESOURCE_TYPE")]
        resource: String,
    },

    /// Watch component data changes on an entity (streaming - press Ctrl+C to stop)
    #[command(name = "get+watch")]
    GetWatch {
        /// Entity ID to watch (u64 integer, e.g., 12345)
        #[arg(value_name = "ENTITY_ID")]
        entity: u64,
        /// Component types to watch (e.g., bevy_transform::components::transform::Transform
        /// bevy_core::name::Name)
        #[arg(value_name = "COMPONENT_TYPES", required = true)]
        components: Vec<String>,
    },

    /// Insert a component on an entity
    Insert {
        /// Entity ID (u64 integer, e.g., 12345)
        #[arg(value_name = "ENTITY_ID")]
        entity: u64,
        /// JSON object with component type and data (e.g., '{"bevy_core::name::Name":
        /// "MyEntity"}')
        #[arg(value_name = "JSON")]
        components: String,
    },

    /// Insert or update a resource
    #[command(name = "insert_resource")]
    InsertResource {
        /// JSON object with resource type and data (e.g., '{"my_game::GameSettings":
        /// {"difficulty": "hard"}}')
        #[arg(value_name = "JSON")]
        data: String,
    },

    /// List all component types
    List,

    /// List all resources
    #[command(name = "list_resources")]
    ListResources,

    /// List all entities with their components
    #[command(name = "list_entities")]
    ListEntities,

    /// Get all component data for a single entity
    #[command(name = "list_entity")]
    ListEntity {
        /// Entity ID to get all component data for (u64 integer, e.g., 12345)
        #[arg(value_name = "ENTITY_ID")]
        entity: u64,
    },

    /// Watch component changes on an entity (streaming - press Ctrl+C to stop)
    #[command(name = "list+watch")]
    ListWatch {
        /// Entity ID to watch for component changes (u64 integer, e.g., 12345)
        #[arg(value_name = "ENTITY_ID")]
        entity: u64,
    },

    /// List available remote methods
    Methods,

    /// Modify specific fields of a component
    #[command(name = "mutate_component")]
    MutateComponent {
        /// Entity ID (u64 integer, e.g., 12345)
        #[arg(value_name = "ENTITY_ID")]
        entity: u64,
        /// Component type name (e.g., bevy_transform::components::transform::Transform)
        #[arg(value_name = "COMPONENT_TYPE")]
        component: String,
        /// JSON patch with fields to update (e.g., '{"translation": [10.0, 0.0, 0.0]}')
        #[arg(value_name = "JSON_PATCH")]
        patch: String,
    },

    /// Modify specific fields of a resource
    #[command(name = "mutate_resource")]
    MutateResource {
        /// Resource type name (e.g., my_game::GameSettings)
        #[arg(value_name = "RESOURCE_TYPE")]
        resource: String,
        /// JSON patch with fields to update (e.g., '{"difficulty": "easy"}')
        #[arg(value_name = "JSON_PATCH")]
        patch: String,
    },

    /// Query entities with specific components
    Query {
        /// Component type names to query for (e.g.,
        /// bevy_transform::components::transform::Transform bevy_core::name::Name)
        #[arg(value_name = "COMPONENT_TYPES", required = true)]
        components: Vec<String>,
    },

    /// Check if app is ready
    Ready,

    /// Remove a component from an entity
    Remove {
        /// Entity ID (u64 integer, e.g., 12345)
        #[arg(value_name = "ENTITY_ID")]
        entity: u64,
        /// Component type to remove (e.g., bevy_core::name::Name)
        #[arg(value_name = "COMPONENT_TYPE")]
        component: String,
    },

    /// Remove a resource
    #[command(name = "remove_resource")]
    RemoveResource {
        /// Resource type name (e.g., my_game::GameSettings)
        #[arg(value_name = "RESOURCE_TYPE")]
        resource: String,
    },

    /// Change entity parent-child relationship
    Reparent {
        /// Child entity ID (u64 integer, e.g., 12345)
        #[arg(value_name = "CHILD_ID")]
        child: u64,
        /// Parent entity ID (u64 integer, e.g., 67890) or 'null' for no parent
        #[arg(value_name = "PARENT_ID")]
        parent: String,
    },

    /// Take a screenshot
    Screenshot {
        /// Path to save the screenshot (e.g., ./screenshot.png or /tmp/capture.png)
        #[arg(value_name = "FILE_PATH")]
        path: String,
    },

    /// Shutdown the app
    Shutdown,

    /// Spawn a new entity with components
    Spawn {
        /// JSON object with component data (e.g.,
        /// '{"bevy_transform::components::transform::Transform": {"translation": [0, 0, 0]}}')
        #[arg(value_name = "JSON")]
        components: String,
    },

    /// Get JSON schemas for all registered types in the Bevy app
    Schema {
        /// Include only types from these crates
        #[arg(long = "with-crates")]
        with_crates: Option<Vec<String>>,
        /// Exclude types from these crates
        #[arg(long = "without-crates")]
        without_crates: Option<Vec<String>>,
        /// Include only types with these reflect traits
        #[arg(long = "with-types")]
        with_types: Option<Vec<String>>,
        /// Exclude types with these reflect traits
        #[arg(long = "without-types")]
        without_types: Option<Vec<String>>,
    },

    /// Execute a raw command string (e.g., bevy/list, bevy/registry/schema)
    Raw {
        /// Command and arguments to pass directly to the server
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, required = true)]
        args: Vec<String>,
    },
}

impl Commands {
    /// Get metadata for this command
    fn metadata(&self) -> CommandMetadata {
        match self {
            Commands::List => CommandMetadata {
                names: &["bevy/list", "list"],
                brief: "List all component types in the world",
            },
            Commands::Query { .. } => CommandMetadata {
                names: &["bevy/query", "query"],
                brief: "Query entities with specific components",
            },
            Commands::Get { .. } => CommandMetadata {
                names: &["bevy/get", "get"],
                brief: "Get component data from a specific entity",
            },
            Commands::Spawn { .. } => CommandMetadata {
                names: &["bevy/spawn", "spawn"],
                brief: "Spawn new entities with components",
            },
            Commands::Destroy { .. } => CommandMetadata {
                names: &["bevy/destroy", "destroy"],
                brief: "Destroy entities",
            },
            Commands::Insert { .. } => CommandMetadata {
                names: &["bevy/insert", "insert"],
                brief: "Insert components on existing entities",
            },
            Commands::Remove { .. } => CommandMetadata {
                names: &["bevy/remove", "remove"],
                brief: "Remove components from entities",
            },
            Commands::Reparent { .. } => CommandMetadata {
                names: &["bevy/reparent", "reparent"],
                brief: "Change entity parent-child relationships",
            },
            Commands::MutateComponent { .. } => CommandMetadata {
                names: &["bevy/mutate_component", "mutate_component"],
                brief: "Modify specific fields of a component",
            },
            Commands::ListResources => CommandMetadata {
                names: &["bevy/list_resources", "list_resources"],
                brief: "List all resources in the world",
            },
            Commands::GetResource { .. } => CommandMetadata {
                names: &["bevy/get_resource", "get_resource"],
                brief: "Get current value of a resource",
            },
            Commands::InsertResource { .. } => CommandMetadata {
                names: &["bevy/insert_resource", "insert_resource"],
                brief: "Insert or update a resource",
            },
            Commands::RemoveResource { .. } => CommandMetadata {
                names: &["bevy/remove_resource", "remove_resource"],
                brief: "Remove a resource from the world",
            },
            Commands::MutateResource { .. } => CommandMetadata {
                names: &["bevy/mutate_resource", "mutate_resource"],
                brief: "Modify specific fields of a resource",
            },
            Commands::ListWatch { .. } => CommandMetadata {
                names: &["bevy/list+watch", "list+watch"],
                brief: "Watch component changes on an entity",
            },
            Commands::GetWatch { .. } => CommandMetadata {
                names: &["bevy/get+watch", "get+watch"],
                brief: "Watch component data changes on an entity",
            },
            Commands::Schema { .. } => CommandMetadata {
                names: &["bevy/registry/schema", "schema"],
                brief: "Get JSON schemas for registered types",
            },
            Commands::Screenshot { .. } => CommandMetadata {
                names: &["brp_tool/screenshot", "screenshot"],
                brief: "Take a screenshot and save to file",
            },
            Commands::Ready => CommandMetadata {
                names: &["ready"],
                brief: "Check if app is ready for commands",
            },
            Commands::Shutdown => CommandMetadata {
                names: &["brp_tool/shutdown", "shutdown"],
                brief: "Gracefully shutdown the application",
            },
            Commands::Methods => CommandMetadata {
                names: &["methods"],
                brief: "List commands available from running app",
            },
            Commands::ListEntities => CommandMetadata {
                names: &["list_entities"],
                brief: "List all entities with their components",
            },
            Commands::ListEntity { .. } => CommandMetadata {
                names: &["list_entity"],
                brief: "Get all component data for a single entity",
            },
            Commands::Raw { .. } => CommandMetadata {
                names: &["raw"],
                brief: "Execute any command directly (bypass CLI parsing)",
            },
        }
    }

    /// Get all possible names for this command
    pub fn names(&self) -> Vec<&'static str> {
        self.metadata().names.to_vec()
    }

    /// Get the brief description for this command
    pub fn brief_description(&self) -> &'static str {
        self.metadata().brief
    }

    /// Get the primary (full) name for this command
    pub fn primary_name(&self) -> &'static str {
        let names = self.names();
        names.first().copied().unwrap_or("unknown")
    }

    /// Get the detailed help text for this command
    pub fn detailed_help(&self) -> String {
        match self {
            Commands::List => include_help!("list").to_string(),
            Commands::Query { .. } => include_help!("query").to_string(),
            Commands::Get { .. } => include_help!("get").to_string(),
            Commands::Spawn { .. } => include_help!("spawn").to_string(),
            Commands::Destroy { .. } => include_help!("destroy").to_string(),
            Commands::Insert { .. } => include_help!("insert").to_string(),
            Commands::Remove { .. } => include_help!("remove").to_string(),
            Commands::Reparent { .. } => include_help!("reparent").to_string(),
            Commands::MutateComponent { .. } => include_help!("mutate_component").to_string(),
            Commands::ListResources => include_help!("list_resources").to_string(),
            Commands::GetResource { .. } => include_help!("get_resource").to_string(),
            Commands::InsertResource { .. } => include_help!("insert_resource").to_string(),
            Commands::RemoveResource { .. } => include_help!("remove_resource").to_string(),
            Commands::MutateResource { .. } => include_help!("mutate_resource").to_string(),
            Commands::ListWatch { .. } => include_help!("list_watch").to_string(),
            Commands::GetWatch { .. } => include_help!("get_watch").to_string(),
            Commands::Schema { .. } => include_help!("schema").to_string(),
            Commands::Screenshot { .. } => include_help!("screenshot").to_string(),
            Commands::Ready => include_help!("ready").to_string(),
            Commands::Shutdown => include_help!("shutdown").to_string(),
            Commands::Methods => include_help!("methods").to_string(),
            Commands::ListEntities => include_help!("list_entities").to_string(),
            Commands::ListEntity { .. } => include_help!("list_entity").to_string(),
            Commands::Raw { .. } => include_help!("raw").to_string(),
        }
    }

    /// Get the category for this command
    pub fn category(&self) -> CommandCategory {
        match self {
            Commands::List
            | Commands::Query { .. }
            | Commands::Get { .. }
            | Commands::Spawn { .. }
            | Commands::Destroy { .. }
            | Commands::Insert { .. }
            | Commands::Remove { .. }
            | Commands::Reparent { .. }
            | Commands::MutateComponent { .. }
            | Commands::Schema { .. }
            | Commands::ListEntities
            | Commands::ListEntity { .. } => CommandCategory::BevyEntity,
            Commands::ListResources
            | Commands::GetResource { .. }
            | Commands::InsertResource { .. }
            | Commands::RemoveResource { .. }
            | Commands::MutateResource { .. } => CommandCategory::BevyResource,
            Commands::ListWatch { .. } | Commands::GetWatch { .. } => CommandCategory::BevyWatch,
            Commands::Screenshot { .. } | Commands::Shutdown => CommandCategory::BrpTool,
            Commands::Methods | Commands::Ready => CommandCategory::Special,
            Commands::Raw { .. } => CommandCategory::Special,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandCategory {
    BevyEntity,
    BevyResource,
    BevyWatch,
    BrpTool,
    Special,
}

impl fmt::Display for CommandCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandCategory::BevyEntity => write!(f, "ENTITY/COMPONENT COMMANDS"),
            CommandCategory::BevyResource => write!(f, "RESOURCE COMMANDS"),
            CommandCategory::BevyWatch => {
                write!(f, "WATCH COMMANDS (streaming - press Ctrl+C to stop)")
            }
            CommandCategory::BrpTool => write!(f, "BRP TOOL COMMANDS (requires BrpToolPlugin)"),
            CommandCategory::Special => write!(f, "SPECIAL COMMANDS"),
        }
    }
}

/// Command template enum without fields for strum iteration
#[derive(Debug, Clone, Copy, EnumIter)]
pub enum CommandTemplate {
    Destroy,
    Get,
    GetResource,
    GetWatch,
    Insert,
    InsertResource,
    List,
    ListResources,
    ListEntities,
    ListEntity,
    ListWatch,
    Methods,
    MutateComponent,
    MutateResource,
    Query,
    Ready,
    Remove,
    RemoveResource,
    Reparent,
    Screenshot,
    Shutdown,
    Spawn,
    Schema,
    Raw,
}

impl CommandTemplate {
    /// Convert template to actual command with default values
    pub fn to_command(self) -> Option<Commands> {
        match self {
            CommandTemplate::Destroy => Some(Commands::Destroy { entity: 0 }),
            CommandTemplate::Get => Some(Commands::Get {
                entity: 0,
                component: String::new(),
            }),
            CommandTemplate::GetResource => Some(Commands::GetResource {
                resource: String::new(),
            }),
            CommandTemplate::GetWatch => Some(Commands::GetWatch {
                entity: 0,
                components: vec![],
            }),
            CommandTemplate::Insert => Some(Commands::Insert {
                entity: 0,
                components: String::new(),
            }),
            CommandTemplate::InsertResource => Some(Commands::InsertResource {
                data: String::new(),
            }),
            CommandTemplate::List => Some(Commands::List),
            CommandTemplate::ListResources => Some(Commands::ListResources),
            CommandTemplate::ListEntities => Some(Commands::ListEntities),
            CommandTemplate::ListEntity => Some(Commands::ListEntity { entity: 0 }),
            CommandTemplate::ListWatch => Some(Commands::ListWatch { entity: 0 }),
            CommandTemplate::Methods => Some(Commands::Methods),
            CommandTemplate::MutateComponent => Some(Commands::MutateComponent {
                entity: 0,
                component: String::new(),
                patch: String::new(),
            }),
            CommandTemplate::MutateResource => Some(Commands::MutateResource {
                resource: String::new(),
                patch: String::new(),
            }),
            CommandTemplate::Query => Some(Commands::Query { components: vec![] }),
            CommandTemplate::Ready => Some(Commands::Ready),
            CommandTemplate::Remove => Some(Commands::Remove {
                entity: 0,
                component: String::new(),
            }),
            CommandTemplate::RemoveResource => Some(Commands::RemoveResource {
                resource: String::new(),
            }),
            CommandTemplate::Reparent => Some(Commands::Reparent {
                child: 0,
                parent: String::new(),
            }),
            CommandTemplate::Screenshot => Some(Commands::Screenshot {
                path: String::new(),
            }),
            CommandTemplate::Shutdown => Some(Commands::Shutdown),
            CommandTemplate::Spawn => Some(Commands::Spawn {
                components: String::new(),
            }),
            CommandTemplate::Schema => Some(Commands::Schema {
                with_crates: None,
                without_crates: None,
                with_types: None,
                without_types: None,
            }),
            CommandTemplate::Raw => Some(Commands::Raw { args: vec![] }), /* Empty vec for */
                                                                          /* display purposes */
        }
    }
}

/// Normalize any command name variant to find the matching command
pub fn normalize_command_name(input: &str) -> String {
    // If it doesn't have a namespace, try to find the full name
    if !input.contains('/') {
        // Check against all command variants to find the full name
        for template in CommandTemplate::iter() {
            if let Some(cmd) = template.to_command() {
                for name in cmd.names() {
                    if name.ends_with(&format!("/{}", input)) || name == input {
                        return name.to_string();
                    }
                }
            }
        }
    }

    input.to_string()
}

/// Find a command by any of its name variants
pub fn find_command_by_name(name: &str) -> Option<Commands> {
    let normalized = normalize_command_name(name);

    for template in CommandTemplate::iter() {
        if let Some(cmd) = template.to_command() {
            if cmd.names().contains(&normalized.as_str()) {
                return Some(cmd);
            }
        }
    }

    // Also check without normalization for exact matches
    for template in CommandTemplate::iter() {
        if let Some(cmd) = template.to_command() {
            if cmd.names().contains(&name) {
                return Some(cmd);
            }
        }
    }

    None
}

/// Get all commands grouped by category
pub fn commands_by_category() -> Vec<(CommandCategory, Vec<Commands>)> {
    let mut categories = vec![
        (CommandCategory::BevyEntity, Vec::new()),
        (CommandCategory::BevyResource, Vec::new()),
        (CommandCategory::BevyWatch, Vec::new()),
        (CommandCategory::BrpTool, Vec::new()),
        (CommandCategory::Special, Vec::new()),
    ];

    for template in CommandTemplate::iter() {
        if let Some(cmd) = template.to_command() {
            let category = cmd.category();
            if let Some((_, commands)) = categories.iter_mut().find(|(cat, _)| *cat == category) {
                commands.push(cmd);
            }
        }
    }

    // Sort commands alphabetically within each category by their short name
    for (_, commands) in categories.iter_mut() {
        commands.sort_by(|a, b| {
            let a_primary = a.primary_name();
            let b_primary = b.primary_name();

            // Get short names for sorting
            let a_short = if a_primary.contains('/') {
                a_primary.split('/').next_back().unwrap_or(a_primary)
            } else {
                a_primary
            };

            let b_short = if b_primary.contains('/') {
                b_primary.split('/').next_back().unwrap_or(b_primary)
            } else {
                b_primary
            };

            a_short.cmp(b_short)
        });
    }

    categories
}
