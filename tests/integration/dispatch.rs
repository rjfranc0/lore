use crate::helpers::Env;
use predicates::prelude::PredicateBooleanExt;

#[test]
fn no_args_prints_short_summary_exits_0() {
    let env = Env::new();
    env.lore()
        .assert()
        .success()
        .stdout(predicates::str::contains("lore — Layered Orchestration"))
        .stdout(predicates::str::contains("LORE(1)").not());
}

#[test]
fn unknown_subcommand_exits_1_with_lore_summary() {
    let env = Env::new();
    env.lore()
        .arg("definitely-not-a-command")
        .assert()
        .failure()
        .code(1)
        .stderr(predicates::str::contains("lore — Layered Orchestration"));
}

#[test]
fn help_exits_0_and_prints_manual() {
    let env = Env::new();
    env.lore()
        .arg("help")
        .assert()
        .success()
        .stdout(predicates::str::contains("LORE(1)"));
}

#[test]
fn dash_dash_help_exits_0() {
    let env = Env::new();
    env.lore().arg("--help").assert().success();
}

#[test]
fn unknown_subcommand_man_exits_1_with_lore_summary() {
    let env = Env::new();
    env.lore()
        .arg("man")
        .assert()
        .failure()
        .code(1)
        .stderr(predicates::str::contains("lore — Layered Orchestration"));
}
