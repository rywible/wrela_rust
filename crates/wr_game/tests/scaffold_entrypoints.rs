use std::collections::BTreeSet;

use wr_core::CrateBoundary;

#[test]
fn every_workspace_package_exposes_an_init_entrypoint() {
    let mut entrypoints = Vec::from(wr_game::scaffold_members());
    entrypoints.extend([
        wr_game::init_entrypoint(),
        xtask::init_entrypoint(),
        wr_client::init_entrypoint(),
        wr_headless::init_entrypoint(),
        wr_agentd::init_entrypoint(),
    ]);

    let names: BTreeSet<_> = entrypoints.iter().map(|entry| entry.crate_name).collect();
    assert_eq!(names.len(), entrypoints.len(), "crate names should be unique");
    assert!(names.contains("wr_game"));
    assert!(names.contains("xtask"));
    assert!(names.contains("wr_client"));
    assert!(names.contains("wr_headless"));
    assert!(names.contains("wr_agentd"));

    let subsystem_count = entrypoints
        .iter()
        .filter(|entry| matches!(entry.boundary, CrateBoundary::Subsystem))
        .count();
    assert_eq!(subsystem_count, 21);

    let integration_only: BTreeSet<_> = entrypoints
        .iter()
        .filter(|entry| entry.integration_only)
        .map(|entry| entry.crate_name)
        .collect();
    assert_eq!(
        integration_only,
        BTreeSet::from(["wr_agentd", "wr_client", "wr_game", "wr_headless"])
    );
}
