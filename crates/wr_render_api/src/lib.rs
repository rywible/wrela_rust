#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use wr_core::{CrateBoundary, CrateEntryPoint};

pub const DEBUG_COLOR_TARGET_RESOURCE: &str = "color_target";
pub const DEBUG_TRIANGLE_PASS_NAME: &str = "debug_geometry";
pub const DEBUG_TRIANGLE_SHADER_ID: &str = "debug_triangle";
pub const DEBUG_TRIANGLE_PIPELINE_ID: &str = "debug_triangle_pipeline";

pub const fn init_entrypoint() -> CrateEntryPoint {
    CrateEntryPoint::new("wr_render_api", CrateBoundary::Subsystem, false)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RenderSize {
    pub width: u32,
    pub height: u32,
}

impl RenderSize {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub const fn pixel_count(self) -> u64 {
        self.width as u64 * self.height as u64
    }

    pub fn validate(self) -> Result<Self, String> {
        if self.width == 0 || self.height == 0 {
            return Err(String::from("render target dimensions must be greater than zero"));
        }

        Ok(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ColorRgba8 {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

impl ColorRgba8 {
    pub const fn new(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self { red, green, blue, alpha }
    }

    pub const fn normalized(self) -> [f64; 4] {
        [
            self.red as f64 / 255.0,
            self.green as f64 / 255.0,
            self.blue as f64 / 255.0,
            self.alpha as f64 / 255.0,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct LinearColor {
    pub red: f32,
    pub green: f32,
    pub blue: f32,
    pub alpha: f32,
}

impl LinearColor {
    pub const fn new(red: f32, green: f32, blue: f32, alpha: f32) -> Self {
        Self { red, green, blue, alpha }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct DebugVertex {
    pub position: Vec3,
    pub color: LinearColor,
}

impl DebugVertex {
    pub const fn new(position: Vec3, color: LinearColor) -> Self {
        Self { position, color }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct DebugTriangle {
    pub vertices: [DebugVertex; 3],
}

impl DebugTriangle {
    pub const fn new(vertices: [DebugVertex; 3]) -> Self {
        Self { vertices }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ScenePrimitive {
    DebugTriangle(DebugTriangle),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ExtractedRenderScene {
    pub clear_color: ColorRgba8,
    pub primitives: Vec<ScenePrimitive>,
}

impl ExtractedRenderScene {
    pub const fn new(clear_color: ColorRgba8) -> Self {
        Self { clear_color, primitives: Vec::new() }
    }

    pub fn push_primitive(&mut self, primitive: ScenePrimitive) {
        self.primitives.push(primitive);
    }

    pub fn debug_triangles(&self) -> impl Iterator<Item = &DebugTriangle> {
        self.primitives.iter().map(|primitive| match primitive {
            ScenePrimitive::DebugTriangle(triangle) => triangle,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RenderColorSpace {
    Rgba8UnormSrgb,
}

impl RenderColorSpace {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Rgba8UnormSrgb => "rgba8_unorm_srgb",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GraphicsAdapterInfo {
    pub backend: String,
    pub name: String,
    pub device_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub driver: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub driver_info: Option<String>,
    pub shading_language: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct OffscreenRenderRequest {
    pub size: RenderSize,
    pub clear_color: ColorRgba8,
}

impl OffscreenRenderRequest {
    pub fn validate(&self) -> Result<(), String> {
        self.size.validate()?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CapturedFrameInfo {
    pub size: RenderSize,
    pub color_space: RenderColorSpace,
    pub non_empty_pixels: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ShaderSource {
    Wgsl(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ShaderModuleAsset {
    pub id: String,
    pub label: String,
    pub source: ShaderSource,
    pub vertex_entry: String,
    pub fragment_entry: String,
}

impl ShaderModuleAsset {
    pub fn validate(&self) -> Result<(), String> {
        if self.id.trim().is_empty() {
            return Err(String::from("shader asset id must not be empty"));
        }
        if self.vertex_entry.trim().is_empty() || self.fragment_entry.trim().is_empty() {
            return Err(format!(
                "shader asset `{}` must define both vertex and fragment entry points",
                self.id
            ));
        }

        match &self.source {
            ShaderSource::Wgsl(source) if source.trim().is_empty() => {
                Err(format!("shader asset `{}` must include WGSL source", self.id))
            }
            ShaderSource::Wgsl(_) => Ok(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PrimitiveTopology {
    TriangleList,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PipelineAssetDescriptor {
    pub id: String,
    pub shader_id: String,
    pub topology: PrimitiveTopology,
    pub color_target: RenderColorSpace,
}

impl PipelineAssetDescriptor {
    pub fn validate(&self) -> Result<(), String> {
        if self.id.trim().is_empty() {
            return Err(String::from("pipeline id must not be empty"));
        }
        if self.shader_id.trim().is_empty() {
            return Err(format!("pipeline `{}` must reference a shader asset id", self.id));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RenderFeatureRegistry {
    pub shaders: Vec<ShaderModuleAsset>,
    pub pipelines: Vec<PipelineAssetDescriptor>,
}

impl RenderFeatureRegistry {
    pub fn validate(&self) -> Result<(), String> {
        let mut shader_ids = BTreeSet::new();
        for shader in &self.shaders {
            shader.validate()?;
            if !shader_ids.insert(shader.id.as_str()) {
                return Err(format!("shader asset `{}` was registered more than once", shader.id));
            }
        }

        let mut pipeline_ids = BTreeSet::new();
        for pipeline in &self.pipelines {
            pipeline.validate()?;
            if !pipeline_ids.insert(pipeline.id.as_str()) {
                return Err(format!("pipeline `{}` was registered more than once", pipeline.id));
            }
            if !shader_ids.contains(pipeline.shader_id.as_str()) {
                return Err(format!(
                    "pipeline `{}` references unknown shader `{}`",
                    pipeline.id, pipeline.shader_id
                ));
            }
        }

        Ok(())
    }

    pub fn has_pipeline(&self, id: &str) -> bool {
        self.pipelines.iter().any(|pipeline| pipeline.id == id)
    }

    pub fn shader(&self, id: &str) -> Option<&ShaderModuleAsset> {
        self.shaders.iter().find(|shader| shader.id == id)
    }

    pub fn pipeline(&self, id: &str) -> Option<&PipelineAssetDescriptor> {
        self.pipelines.iter().find(|pipeline| pipeline.id == id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RenderResourceKind {
    ColorTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RenderGraphResource {
    pub name: String,
    pub kind: RenderResourceKind,
    pub external: bool,
}

impl RenderGraphResource {
    pub fn new(name: impl Into<String>, kind: RenderResourceKind, external: bool) -> Self {
        Self { name: name.into(), kind, external }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RenderPassNode {
    pub name: String,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub reads: Vec<String>,
    #[serde(default)]
    pub writes: Vec<String>,
    pub pipeline_id: String,
}

impl RenderPassNode {
    pub fn new(name: impl Into<String>, pipeline_id: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            dependencies: Vec::new(),
            reads: Vec::new(),
            writes: Vec::new(),
            pipeline_id: pipeline_id.into(),
        }
    }

    pub fn depends_on(mut self, dependency: impl Into<String>) -> Self {
        self.dependencies.push(dependency.into());
        self
    }

    pub fn reads(mut self, resource: impl Into<String>) -> Self {
        self.reads.push(resource.into());
        self
    }

    pub fn writes(mut self, resource: impl Into<String>) -> Self {
        self.writes.push(resource.into());
        self
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RenderGraph {
    pub resources: Vec<RenderGraphResource>,
    pub passes: Vec<RenderPassNode>,
}

impl RenderGraph {
    pub fn declare_resource(mut self, resource: RenderGraphResource) -> Self {
        self.resources.push(resource);
        self
    }

    pub fn add_pass(mut self, pass: RenderPassNode) -> Self {
        self.passes.push(pass);
        self
    }

    pub fn validate(&self) -> Result<(), String> {
        self.validate_internal(None)
    }

    pub fn validate_with_registry(&self, registry: &RenderFeatureRegistry) -> Result<(), String> {
        registry.validate()?;
        self.validate_internal(Some(registry))
    }

    pub fn ordered_pass_names(&self) -> Result<Vec<String>, String> {
        self.validate_structure()?;
        let dependencies = self.pass_dependency_map()?;
        self.topological_order_from_dependencies(&dependencies)
    }

    pub fn pass(&self, name: &str) -> Option<&RenderPassNode> {
        self.passes.iter().find(|pass| pass.name == name)
    }

    fn validate_internal(&self, registry: Option<&RenderFeatureRegistry>) -> Result<(), String> {
        self.validate_structure_with_registry(registry)?;
        let dependencies = self.pass_dependency_map()?;
        let _ = self.topological_order_from_dependencies(&dependencies)?;
        Ok(())
    }

    fn validate_structure(&self) -> Result<(), String> {
        self.validate_structure_with_registry(None)
    }

    fn validate_structure_with_registry(
        &self,
        registry: Option<&RenderFeatureRegistry>,
    ) -> Result<(), String> {
        let resource_map = self.resource_map()?;
        let writer_map = self.resource_writer_map()?;
        let pass_names = self.pass_names()?;

        for pass in &self.passes {
            if pass.pipeline_id.trim().is_empty() {
                return Err(format!("render pass `{}` must reference a pipeline id", pass.name));
            }
            if let Some(registry) = registry
                && !registry.has_pipeline(&pass.pipeline_id)
            {
                return Err(format!(
                    "render pass `{}` references unknown pipeline `{}`",
                    pass.name, pass.pipeline_id
                ));
            }

            for dependency in &pass.dependencies {
                if dependency == &pass.name {
                    return Err(format!("render pass `{}` cannot depend on itself", pass.name));
                }
                if !pass_names.contains(dependency.as_str()) {
                    return Err(format!(
                        "render pass `{}` references unknown dependency `{dependency}`",
                        pass.name
                    ));
                }
            }

            for resource in pass.reads.iter().chain(&pass.writes) {
                if !resource_map.contains_key(resource.as_str()) {
                    return Err(format!(
                        "render pass `{}` references undeclared resource `{resource}`",
                        pass.name
                    ));
                }
            }

            for resource in &pass.reads {
                let declaration = resource_map
                    .get(resource.as_str())
                    .expect("resource existence was checked above");
                if !declaration.external && !writer_map.contains_key(resource.as_str()) {
                    return Err(format!(
                        "render pass `{}` reads `{resource}` before any pass writes it",
                        pass.name
                    ));
                }
            }
        }

        Ok(())
    }

    fn topological_order_from_dependencies(
        &self,
        dependencies: &BTreeMap<&str, BTreeSet<String>>,
    ) -> Result<Vec<String>, String> {
        let mut indegrees = BTreeMap::new();
        let mut reverse_edges: BTreeMap<&str, Vec<&str>> = BTreeMap::new();

        for pass in &self.passes {
            indegrees.insert(pass.name.as_str(), dependencies[pass.name.as_str()].len());
            for dependency in &dependencies[pass.name.as_str()] {
                reverse_edges.entry(dependency.as_str()).or_default().push(pass.name.as_str());
            }
        }

        let mut ready = indegrees
            .iter()
            .filter_map(|(name, indegree)| (*indegree == 0).then_some(*name))
            .collect::<VecDeque<_>>();
        let mut ordered = Vec::with_capacity(self.passes.len());

        while let Some(pass_name) = ready.pop_front() {
            ordered.push(pass_name.to_owned());
            if let Some(children) = reverse_edges.get(pass_name) {
                for child in children {
                    let indegree =
                        indegrees.get_mut(child).expect("child pass should exist in indegree map");
                    *indegree = indegree.saturating_sub(1);
                    if *indegree == 0 {
                        ready.push_back(child);
                    }
                }
            }
        }

        if ordered.len() != self.passes.len() {
            return Err(String::from("render graph contains a cycle"));
        }

        Ok(ordered)
    }

    fn resource_map(&self) -> Result<BTreeMap<&str, &RenderGraphResource>, String> {
        let mut resources = BTreeMap::new();
        for resource in &self.resources {
            if resource.name.trim().is_empty() {
                return Err(String::from("render graph resources must have a non-empty name"));
            }
            if resources.insert(resource.name.as_str(), resource).is_some() {
                return Err(format!(
                    "render graph resource `{}` was declared more than once",
                    resource.name
                ));
            }
        }
        Ok(resources)
    }

    fn pass_names(&self) -> Result<BTreeSet<&str>, String> {
        let mut passes = BTreeSet::new();
        for pass in &self.passes {
            if pass.name.trim().is_empty() {
                return Err(String::from("render graph passes must have a non-empty name"));
            }
            if !passes.insert(pass.name.as_str()) {
                return Err(format!("render pass `{}` was declared more than once", pass.name));
            }
        }
        Ok(passes)
    }

    fn resource_writer_map(&self) -> Result<BTreeMap<&str, &str>, String> {
        let mut writers = BTreeMap::new();
        for pass in &self.passes {
            for resource in &pass.writes {
                if let Some(previous) = writers.insert(resource.as_str(), pass.name.as_str()) {
                    return Err(format!(
                        "resource `{resource}` is written by both `{previous}` and `{}`",
                        pass.name
                    ));
                }
            }
        }
        Ok(writers)
    }

    fn pass_dependency_map(&self) -> Result<BTreeMap<&str, BTreeSet<String>>, String> {
        let writers = self.resource_writer_map()?;
        let mut dependencies = BTreeMap::new();

        for pass in &self.passes {
            let mut pass_dependencies = BTreeSet::new();
            for dependency in &pass.dependencies {
                pass_dependencies.insert(dependency.clone());
            }
            for resource in &pass.reads {
                if let Some(writer) = writers.get(resource.as_str())
                    && *writer != pass.name
                {
                    pass_dependencies.insert((*writer).to_owned());
                }
            }
            dependencies.insert(pass.name.as_str(), pass_dependencies);
        }

        Ok(dependencies)
    }
}

pub fn debug_triangle_graph() -> RenderGraph {
    RenderGraph::default()
        .declare_resource(RenderGraphResource::new(
            DEBUG_COLOR_TARGET_RESOURCE,
            RenderResourceKind::ColorTarget,
            true,
        ))
        .add_pass(
            RenderPassNode::new(DEBUG_TRIANGLE_PASS_NAME, DEBUG_TRIANGLE_PIPELINE_ID)
                .writes(DEBUG_COLOR_TARGET_RESOURCE),
        )
}

pub fn clear_color_from_seed_hex(seed_hex: &str) -> ColorRgba8 {
    let trimmed = seed_hex.trim().trim_start_matches("0x").trim_start_matches("0X");

    let mut bytes = [0_u8; 4];
    for (index, chunk) in trimmed.as_bytes().rchunks(2).take(4).enumerate() {
        let value = std::str::from_utf8(chunk)
            .ok()
            .and_then(|chunk| u8::from_str_radix(chunk, 16).ok())
            .unwrap_or(0);
        bytes[3 - index] = value;
    }

    ColorRgba8 {
        red: bytes[0].saturating_add(32),
        green: bytes[1].saturating_add(48),
        blue: bytes[2].saturating_add(64),
        alpha: 255,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> RenderFeatureRegistry {
        RenderFeatureRegistry {
            shaders: vec![ShaderModuleAsset {
                id: DEBUG_TRIANGLE_SHADER_ID.to_owned(),
                label: String::from("debug triangle"),
                source: ShaderSource::Wgsl(String::from(
                    "@vertex fn vs_main() -> @builtin(position) vec4<f32> { return vec4<f32>(); }\n@fragment fn fs_main() -> @location(0) vec4<f32> { return vec4<f32>(); }",
                )),
                vertex_entry: String::from("vs_main"),
                fragment_entry: String::from("fs_main"),
            }],
            pipelines: vec![PipelineAssetDescriptor {
                id: DEBUG_TRIANGLE_PIPELINE_ID.to_owned(),
                shader_id: DEBUG_TRIANGLE_SHADER_ID.to_owned(),
                topology: PrimitiveTopology::TriangleList,
                color_target: RenderColorSpace::Rgba8UnormSrgb,
            }],
        }
    }

    #[test]
    fn seed_derived_clear_colors_are_opaque_and_non_zero() {
        let color = clear_color_from_seed_hex("0x00000000DEADBEEF");

        assert_eq!(color.alpha, 255);
        assert!(u16::from(color.red) + u16::from(color.green) + u16::from(color.blue) > 0);
    }

    #[test]
    fn render_size_rejects_zero_dimensions() {
        let error = RenderSize::new(0, 360).validate().expect_err("zero width should fail");

        assert!(error.contains("greater than zero"));
    }

    #[test]
    fn feature_registry_rejects_pipeline_with_unknown_shader() {
        let error = RenderFeatureRegistry {
            shaders: Vec::new(),
            pipelines: vec![PipelineAssetDescriptor {
                id: String::from("main"),
                shader_id: String::from("missing"),
                topology: PrimitiveTopology::TriangleList,
                color_target: RenderColorSpace::Rgba8UnormSrgb,
            }],
        }
        .validate()
        .expect_err("unknown shader should be rejected");

        assert!(error.contains("unknown shader"));
    }

    #[test]
    fn render_graph_orders_resource_dependencies() {
        let graph = RenderGraph::default()
            .declare_resource(RenderGraphResource::new(
                "gbuffer",
                RenderResourceKind::ColorTarget,
                false,
            ))
            .declare_resource(RenderGraphResource::new(
                DEBUG_COLOR_TARGET_RESOURCE,
                RenderResourceKind::ColorTarget,
                true,
            ))
            .add_pass(RenderPassNode::new("opaque", DEBUG_TRIANGLE_PIPELINE_ID).writes("gbuffer"))
            .add_pass(
                RenderPassNode::new("composite", DEBUG_TRIANGLE_PIPELINE_ID)
                    .reads("gbuffer")
                    .writes(DEBUG_COLOR_TARGET_RESOURCE),
            );

        let ordered = graph.ordered_pass_names().expect("graph should be valid");

        assert_eq!(ordered, vec![String::from("opaque"), String::from("composite")]);
    }

    #[test]
    fn render_graph_rejects_dependency_cycles() {
        let graph = RenderGraph::default()
            .declare_resource(RenderGraphResource::new(
                DEBUG_COLOR_TARGET_RESOURCE,
                RenderResourceKind::ColorTarget,
                true,
            ))
            .add_pass(
                RenderPassNode::new("first", DEBUG_TRIANGLE_PIPELINE_ID)
                    .depends_on("second")
                    .writes(DEBUG_COLOR_TARGET_RESOURCE),
            )
            .add_pass(
                RenderPassNode::new("second", DEBUG_TRIANGLE_PIPELINE_ID)
                    .depends_on("first")
                    .writes("history"),
            )
            .declare_resource(RenderGraphResource::new(
                "history",
                RenderResourceKind::ColorTarget,
                false,
            ));

        let error = graph.validate().expect_err("cyclic dependencies should fail");

        assert!(error.contains("cycle"));
    }

    #[test]
    fn render_graph_validates_pipeline_bindings_against_registry() {
        debug_triangle_graph()
            .validate_with_registry(&registry())
            .expect("debug graph should validate against the registered assets");
    }

    #[test]
    fn extracted_scene_exposes_debug_triangle_iterators() {
        let triangle = DebugTriangle::new([
            DebugVertex::new(Vec3::new(-0.5, -0.5, 0.0), LinearColor::new(1.0, 0.0, 0.0, 1.0)),
            DebugVertex::new(Vec3::new(0.5, -0.5, 0.0), LinearColor::new(0.0, 1.0, 0.0, 1.0)),
            DebugVertex::new(Vec3::new(0.0, 0.5, 0.0), LinearColor::new(0.0, 0.0, 1.0, 1.0)),
        ]);
        let mut scene = ExtractedRenderScene::new(ColorRgba8::new(1, 2, 3, 255));
        scene.push_primitive(ScenePrimitive::DebugTriangle(triangle.clone()));

        let triangles = scene.debug_triangles().cloned().collect::<Vec<_>>();

        assert_eq!(triangles, vec![triangle]);
    }
}
