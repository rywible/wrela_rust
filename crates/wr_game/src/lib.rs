#![forbid(unsafe_code)]

use wr_core::{CrateBoundary, CrateEntryPoint};

pub const fn init_entrypoint() -> CrateEntryPoint {
    CrateEntryPoint::new("wr_game", CrateBoundary::Composition, true)
}

pub const fn scaffold_members() -> [CrateEntryPoint; 21] {
    [
        wr_core::init_entrypoint(),
        wr_math::init_entrypoint(),
        wr_world_seed::init_entrypoint(),
        wr_ecs::init_entrypoint(),
        wr_platform::init_entrypoint(),
        wr_render_api::init_entrypoint(),
        wr_render_wgpu::init_entrypoint(),
        wr_render_atmo::init_entrypoint(),
        wr_render_scene::init_entrypoint(),
        wr_render_post::init_entrypoint(),
        wr_world_gen::init_entrypoint(),
        wr_procgeo::init_entrypoint(),
        wr_physics::init_entrypoint(),
        wr_combat::init_entrypoint(),
        wr_ai::init_entrypoint(),
        wr_actor_player::init_entrypoint(),
        wr_actor_wraith::init_entrypoint(),
        wr_vfx::init_entrypoint(),
        wr_tools_ui::init_entrypoint(),
        wr_tools_harness::init_entrypoint(),
        wr_telemetry::init_entrypoint(),
    ]
}
