mod tweaks;

pub use tweaks::{
    TWEAK_PACK_SCHEMA_VERSION, TweakDefinition, TweakError, TweakNamespace, TweakPack,
    TweakRegistry, TweakValue, TweakValueKind, default_tweak_definitions, load_tweak_pack_ron,
    parse_tweak_pack_ron, serialize_tweak_pack_ron, write_tweak_pack_ron,
};

#![forbid(unsafe_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrateBoundary {
    Subsystem,
    Composition,
    AppShell,
    Tooling,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrateEntryPoint {
    pub crate_name: &'static str,
    pub boundary: CrateBoundary,
    pub integration_only: bool,
}

impl CrateEntryPoint {
    pub const fn new(
        crate_name: &'static str,
        boundary: CrateBoundary,
        integration_only: bool,
    ) -> Self {
        Self { crate_name, boundary, integration_only }
    }
}

pub const fn init_entrypoint() -> CrateEntryPoint {
    CrateEntryPoint::new("wr_core", CrateBoundary::Subsystem, false)
}
