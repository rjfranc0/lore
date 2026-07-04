use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "lore",
    version,
    disable_version_flag = true,
    disable_help_subcommand = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Bootstrap ~/.agents/ and wire Claude integration
    Init {
        #[arg(long)]
        account: Option<String>,
    },
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
    /// Manage Claude accounts
    Accounts {
        #[command(subcommand)]
        action: AccountsAction,
    },
    /// Reconcile AGENTS.md from disk
    Sync,
    /// Re-link a skill or behavior whose source has moved
    Update {
        name: Option<String>,
        #[arg(long)]
        all: bool,
        #[arg(long)]
        path: Option<String>,
    },
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

#[derive(Subcommand)]
pub enum AccountsAction {
    /// List registered accounts
    List,
    /// Remove an account from the registry (registry only — no disk changes)
    Remove {
        #[arg(required = true)]
        name: String,
    },
    /// Re-wire any registered account that's broken on disk
    Sync,
}
