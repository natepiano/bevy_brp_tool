use clap::Parser;

use super::types::Commands;
use crate::{DEFAULT_REMOTE_PORT, include_help};

#[derive(Parser)]
#[command(name = "brp")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Control Bevy apps from the command line")]
#[command(next_display_order = None)] // Force alphabetical sorting
#[command(before_help = include_help!("help"))]
#[command(help_template = "{before-help}
{all-args}{after-help}
")]
#[command(
    long_about = "Control running Bevy apps remotely using the Bevy Remote Protocol (BRP).

Use --help-for <command> for detailed help on specific commands.
Use --list-commands to see all available commands.
Use --brp to see BRP configuration requirements."
)]
#[command(disable_help_subcommand = true)]
pub struct Cli {
    /// Port to connect to [default: 15702]
    #[arg(short, long, default_value_t = DEFAULT_REMOTE_PORT, hide_default_value = true, long_help = include_help!("port"))]
    pub port: u16,

    /// Start app and execute commands directly (comma-separated)
    #[arg(short = 'm', long, long_help = include_help!("managed_commands"))]
    pub managed_commands: Option<String>,

    /// App binary to run in managed or detached mode.
    /// If not specified, will attempt to detect a Bevy app in the current workspace.
    #[arg(short, long, long_help = include_help!("app"))]
    pub app: Option<String>,

    /// Build profile to use [default: debug]
    #[arg(short = 'P', long, long_help = include_help!("profile"))]
    pub profile: Option<String>,

    /// Start app in detached mode (persistent session with temp log file)
    #[arg(short = 'd', long, long_help = include_help!("detached"))]
    pub detached: bool,

    /// Show help for a specific command
    #[arg(short = 'f', long = "help-for", value_name = "COMMAND")]
    pub help_for: Option<String>,

    /// List all known commands without connecting to an app
    #[arg(short, long = "list-commands")]
    pub list_commands: bool,

    /// Show complete workflow examples
    #[arg(
        short,
        long = "workflows",
        alias = "examples",
        help_heading = "Tutorial"
    )]
    pub workflows: bool,

    /// Show instructions for coding agents
    #[arg(short = 'A', long = "agent", help_heading = "Tutorial")]
    pub agent: bool,

    /// Show BRP configuration requirements
    #[arg(short, long = "brp", help_heading = "Tutorial")]
    pub brp: bool,

    /// Show information about the current detached session
    #[arg(short, long = "info", long_help = include_help!("info"))]
    pub info: bool,

    /// Clean up session log files from temp directory
    #[arg(short = 'c', long = "cleanup-logs", long_help = include_help!("cleanup_logs"))]
    pub cleanup_logs: bool,

    /// Show detected Bevy app in current workspace
    #[arg(short = 'D', long = "detect")]
    pub detect: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}
