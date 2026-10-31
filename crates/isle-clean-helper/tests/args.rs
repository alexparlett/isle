//! The helper takes one known subcommand and nothing more.

use std::process::Command;

fn status(args: &[&str]) -> Option<i32> {
    Command::new(env!("CARGO_BIN_EXE_isle-clean-helper"))
        .args(args)
        .status()
        .unwrap()
        .code()
}

#[test]
fn it_takes_one_known_subcommand_and_nothing_more() {
    assert_eq!(status(&[]), Some(2));
    assert_eq!(status(&["rm"]), Some(2));
    assert_eq!(
        status(&["orphans", "linux"]),
        Some(2),
        "no package from the caller"
    );
    assert_eq!(
        status(&["coredumps", "/etc"]),
        Some(2),
        "no path from the caller"
    );
}
