#![forbid(unsafe_code)]

use wr_core::{CrateBoundary, CrateEntryPoint};

pub const fn init_entrypoint() -> CrateEntryPoint {
    CrateEntryPoint::new("xtask", CrateBoundary::Tooling, false)
}

pub const fn supported_commands() -> &'static [&'static str] {
    &["help", "scaffold-status"]
}

pub fn run(mut args: impl Iterator<Item = String>) -> i32 {
    match args.next().as_deref() {
        None | Some("help") | Some("--help") | Some("-h") => {
            println!("xtask scaffold only; full task automation lands in later roadmap PRs.");
            println!("available stub commands: {}", supported_commands().join(", "));
            0
        }
        Some("scaffold-status") => {
            println!("workspace scaffold present; see PR-001 for harness contract work.");
            0
        }
        Some(command) => {
            eprintln!(
                "unsupported xtask command `{command}` in scaffold phase; implement it in its owning roadmap task"
            );
            1
        }
    }
}
