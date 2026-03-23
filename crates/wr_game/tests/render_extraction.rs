use bevy_ecs::component::Component;
use tempfile::tempdir;
use wr_render_api::{
    ColorRgba8, DebugTriangle, DebugVertex, ExtractedRenderScene, LinearColor, ScenePrimitive,
    Vec3, debug_triangle_graph,
};
use wr_render_wgpu::render_scene_to_png;

#[derive(Debug, Clone, Component, PartialEq)]
struct RenderableDebugTriangle(DebugTriangle);

fn extract_scene(
    runtime: &mut wr_ecs::EcsRuntime,
    clear_color: ColorRgba8,
) -> ExtractedRenderScene {
    let mut scene = ExtractedRenderScene::new(clear_color);
    let world = runtime.world_mut();
    let mut query = world.query::<&RenderableDebugTriangle>();
    for triangle in query.iter(world) {
        scene.push_primitive(ScenePrimitive::DebugTriangle(triangle.0.clone()));
    }

    scene
}

fn triangle(left_x: f32) -> DebugTriangle {
    DebugTriangle::new([
        DebugVertex::new(Vec3::new(left_x, -0.75, 0.0), LinearColor::new(1.0, 0.0, 0.0, 1.0)),
        DebugVertex::new(Vec3::new(left_x + 1.1, -0.75, 0.0), LinearColor::new(0.0, 1.0, 0.0, 1.0)),
        DebugVertex::new(Vec3::new(left_x + 0.55, 0.75, 0.0), LinearColor::new(0.0, 0.0, 1.0, 1.0)),
    ])
}

#[test]
fn extracted_scene_is_frame_safe_and_renders_through_the_graph() {
    let mut runtime = wr_game::compose_game_runtime(std::iter::empty())
        .expect("empty runtime composition should succeed");
    let entity = runtime.world_mut().spawn(RenderableDebugTriangle(triangle(-0.85))).id();

    let first_scene = extract_scene(&mut runtime, ColorRgba8::new(10, 12, 16, 255));
    runtime.world_mut().entity_mut(entity).insert(RenderableDebugTriangle(triangle(-0.15)));
    let second_scene = extract_scene(&mut runtime, ColorRgba8::new(10, 12, 16, 255));

    assert_ne!(first_scene, second_scene);

    let temp = tempdir().expect("tempdir should exist");
    let output_path = temp.path().join("ecs-scene.png");
    render_scene_to_png(
        wr_render_api::RenderSize::new(96, 96),
        &first_scene,
        &debug_triangle_graph(),
        &output_path,
    )
    .expect("rendering an extracted ECS scene should succeed");

    let image = image::open(&output_path).expect("png should load").into_rgba8();
    let center = image.get_pixel(32, 48).0;

    assert_eq!(first_scene.primitives.len(), 1);
    assert_eq!(second_scene.primitives.len(), 1);
    assert_eq!(image.width(), 96);
    assert_eq!(image.height(), 96);
    assert_ne!(center, [10, 12, 16, 255]);
}
