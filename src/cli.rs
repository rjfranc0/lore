use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "lore", version, disable_version_flag = true, disable_help_subcommand = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Bootstrap ~/.agents/ and wire Claude integration
    Init,
    /// Install skill(s) from the current directory
    Install {
        #[arg(required = true)]
        skills: Vec<String>,
    },
    /// Uninstall skill(s)
    Remove {
        #[arg(required = true)]
        skills: Vec<String>,
    },
    /// Manage behaviors
    Behavior {
        #[command(subcommand)]
        action: BehaviorAction,
    },
    /// Reconcile AGENTS.md from disk
    Sync,
    /// Show installed skills and behaviors
    List,
    /// Print the version
    Version,
    /// Show the full manual
    Help,
}

#[derive(Subcommand)]
pub enum BehaviorAction {
    /// Add behavior(s) from the current directory
    Add {
        #[arg(required = true)]
        names: Vec<String>,
    },
    /// Remove behavior(s)
    Remove {
        #[arg(required = true)]
        names: Vec<String>,
    },
}
