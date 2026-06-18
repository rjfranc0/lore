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
use cli::{Cli, Command, BehaviorAction};

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
        Some(Command::Init)                  => commands::init::run(),
        Some(Command::Install { skills })    => commands::install::run(skills),
        Some(Command::Remove  { skills })    => commands::remove::run(skills),
        Some(Command::Sync)                  => commands::sync::run(),
        Some(Command::List)                  => commands::list::run(),
        Some(Command::Behavior { action }) => match action {
            BehaviorAction::Add    { names } => commands::behavior::add(names),
            BehaviorAction::Remove { names } => commands::behavior::remove(names),
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

  lore init                           bootstrap ~/.agents + Claude integration
  lore install <skill> [...]          install skill(s) from current directory
  lore remove  <skill> [...]          uninstall skill(s)
  lore behavior add    <name> [...]   add behavior(s) from current directory
  lore behavior remove <name> [...]   remove behavior(s)
  lore sync                           reconcile AGENTS.md from disk
  lore list                           show installed skills and behaviors
  lore version                        print version
  lore help                           full manual

  AGENTS_DIR   override base dir (default: ~/.agents)
  CLAUDE_DIR   override Claude config dir (default: ~/.claude)
"#;
