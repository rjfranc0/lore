use crate::helpers::Env;

#[test]
fn version_prints_lore_version_string() {
    let env = Env::new();
    env.lore()
        .arg("version")
        .assert()
        .success()
        .stdout(predicates::str::starts_with("lore "));
}
