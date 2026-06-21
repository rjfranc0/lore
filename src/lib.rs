pub mod agents_md;
pub mod cli;
pub mod commands;
pub mod config;
pub mod output;
pub mod paths;
pub mod symlink;
pub mod wire;

use std::process::ExitCode;
use clap::Parser;
use cli::{AccountsAction, BehaviorAction, Cli, Command};

pub fn run() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(c) => c,
        Err(e) => {
            match e.kind() {
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion => {
                    e.print().ok();
                    return ExitCode::SUCCESS;
                }
                _ => {
                    eprint!("{SHORT_HELP}");
                    return ExitCode::FAILURE;
                }
            }
        }
    };

    let result = match cli.command {
        None => {
            print!("{SHORT_HELP}");
            return ExitCode::SUCCESS;
        }
        Some(Command::Version) => {
            println!("lore {}", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
        Some(Command::Help) => {
            commands::help::run();
            return ExitCode::SUCCESS;
        }
        Some(Command::Init { account })            => commands::init::run(account),
        Some(Command::Install { skills, account }) => commands::install::run(skills, account),
        Some(Command::Remove  { skills, account }) => commands::remove::run(skills, account),
        Some(Command::Sync)                  => commands::sync::run(),
        Some(Command::List)                  => commands::list::run(),
        Some(Command::Behavior { action }) => match action {
            BehaviorAction::Add    { names } => commands::behavior::add(names),
            BehaviorAction::Remove { names } => commands::behavior::remove(names),
        },
        Some(Command::Accounts { action }) => match action {
            AccountsAction::List            => commands::accounts::list(),
            AccountsAction::Remove { name } => commands::accounts::remove(name),
            AccountsAction::Sync            => commands::accounts::sync(),
        },
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("✗ {e}");
            ExitCode::FAILURE
        }
    }
}

const SHORT_HELP: &str = r#"lore — Layered Orchestration for Rules and Extensions

  lore init [--account <name>]                   bootstrap ~/.agents + Claude integration
  lore install <skill> [...] [--account <name>]  install skill(s) from current directory
  lore remove  <skill> [...] [--account <name>]  uninstall skill(s)
  lore behavior add    <name> [...]              add behavior(s) from current directory
  lore behavior remove <name> [...]              remove behavior(s)
  lore accounts list                             show registered accounts
  lore accounts remove <name>                    remove an account from the registry
  lore accounts sync                             re-wire accounts broken on disk
  lore sync                                      reconcile AGENTS.md from disk
  lore list                                      show installed skills and behaviors
  lore version                                   print version
  lore help                                      full manual

  LORE_CONF   override config file path (default: ~/.config/lore/lore.toml)
"#;
