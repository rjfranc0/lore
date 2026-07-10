pub mod accounts;
pub mod behavior;
pub mod help;
pub mod init;
pub mod install;
pub mod list;
pub mod remove;
pub mod sync;
pub mod update;

/// Normalizes a skill/behavior name argument: a trailing slash (e.g. from
/// shell tab-completion of a directory) is equivalent to the same name
/// without one.
pub fn normalize_name(raw: &str) -> &str {
    raw.trim_end_matches('/')
}
