mod cli;
mod execution;
mod parsing;
mod types;

pub use cli::Cli;
pub use execution::*;
pub use parsing::{extract_command_from_error, format_command, parse_command_string};
pub use types::{CommandTemplate, Commands, commands_by_category, find_command_by_name};
