#![forbid(unsafe_code)]

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, mpsc};

use image::ImageEncoder;
use wgpu::util::DeviceExt;
use winit::window::Window;
use wr_core::{CrateBoundary, CrateEntryPoint};
use wr_render_api::{
    CapturedFrameInfo, ColorRgba8, DEBUG_TRIANGLE_PASS_NAME, DEBUG_TRIANGLE_PIPELINE_ID,
    DEBUG_TRIANGLE_SHADER_ID, DebugTriangle, ExtractedRenderScene, FOLIAGE_CARD_PASS_NAME,
    FOLIAGE_CARD_PIPELINE_ID, FOLIAGE_CARD_SHADER_ID, FoliageCard, GraphicsAdapterInfo,
    OffscreenRenderRequest, PipelineAssetDescriptor, PrimitiveTopology, RenderColorSpace,
    RenderFeatureRegistry, RenderGraph, RenderSize, ShaderModuleAsset, ShaderSource,
    debug_triangle_graph,
};

const CLEAR_PASS_SHADER_WGSL: &str = include_str!("clear_pass.wgsl");
const DEBUG_TRIANGLE_SHADER_WGSL: &str = include_str!("debug_triangle.wgsl");
const FOLIAGE_CARD_SHADER_WGSL: &str = include_str!("foliage_card.wgsl");
const TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

pub const fn init_entrypoint() -> CrateEntryPoint {
    CrateEntryPoint::new("wr_render_wgpu", CrateBoundary::Subsystem, false)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OffscreenCaptureOutcome {
    pub adapter: GraphicsAdapterInfo,
    pub frame: CapturedFrameInfo,
}

pub fn shader_source() -> &'static str {
    CLEAR_PASS_SHADER_WGSL
}

pub fn compile_clear_pass_shader() -> Result<GraphicsAdapterInfo, String> {
    let gpu = WgpuContext::new(None)?;
    let _shader = gpu.create_clear_shader_module();
    Ok(gpu.adapter)
}

pub fn render_offscreen_png(
    request: &OffscreenRenderRequest,
    output_path: impl AsRef<Path>,
) -> Result<OffscreenCaptureOutcome, String> {
    request.validate()?;

    let scene = ExtractedRenderScene::new(request.clear_color);
    render_scene_to_png(request.size, &scene, &debug_triangle_graph(), output_path)
}

pub fn render_scene_to_png(
    size: RenderSize,
    scene: &ExtractedRenderScene,
    graph: &RenderGraph,
    output_path: impl AsRef<Path>,
) -> Result<OffscreenCaptureOutcome, String> {
    render_scene_to_png_with_factories(
        size,
        scene,
        graph,
        &[&DebugTrianglePassFactory, &FoliageCardPassFactory],
        output_path,
    )
}

pub fn builtin_scene_registry() -> RenderFeatureRegistry {
    registry_from_factories(&[&DebugTrianglePassFactory, &FoliageCardPassFactory])
}

pub trait RenderPassFactory: Send + Sync {
    fn pass_name(&self) -> &'static str;
    fn shader_asset(&self) -> ShaderModuleAsset;
    fn pipeline_asset(&self) -> PipelineAssetDescriptor;
    fn create_pipeline(
        &self,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        shader: &wgpu::ShaderModule,
    ) -> wgpu::RenderPipeline;
    fn encode(&self, context: RenderPassEncodeContext<'_>) -> Result<(), String>;
}

pub struct RenderPassEncodeContext<'a> {
    pub scene: &'a ExtractedRenderScene,
    pub device: &'a wgpu::Device,
    pub encoder: &'a mut wgpu::CommandEncoder,
    pub target: &'a wgpu::TextureView,
    pub pipeline: &'a wgpu::RenderPipeline,
    pub clear_color: ColorRgba8,
    pub load_existing: bool,
}

pub fn render_scene_to_png_with_factories(
    size: RenderSize,
    scene: &ExtractedRenderScene,
    graph: &RenderGraph,
    factories: &[&dyn RenderPassFactory],
    output_path: impl AsRef<Path>,
) -> Result<OffscreenCaptureOutcome, String> {
    size.validate()?;

    let registry = registry_from_factories(factories);
    graph.validate_with_registry(&registry)?;
    let ordered_passes = graph.ordered_pass_names()?;
    let factory_lookup = build_factory_lookup(factories)?;

    let gpu = WgpuContext::new(None)?;
    let shader_modules = build_shader_modules(&gpu.device, &registry)?;
    let pipelines =
        build_pipelines(&gpu.device, TEXTURE_FORMAT, &registry, &shader_modules, &factory_lookup)?;
    let texture = create_render_texture(&gpu.device, size, TEXTURE_FORMAT);
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let bytes = render_graph_to_rgba8(GraphExecutionContext {
        device: &gpu.device,
        queue: &gpu.queue,
        graph,
        scene,
        ordered_passes: &ordered_passes,
        factory_lookup: &factory_lookup,
        pipelines: &pipelines,
        view: &view,
        texture: Some(&texture),
        size,
    })?;
    write_png(output_path.as_ref(), size, &bytes)?;

    Ok(OffscreenCaptureOutcome {
        adapter: gpu.adapter,
        frame: CapturedFrameInfo {
            size,
            color_space: RenderColorSpace::Rgba8UnormSrgb,
            non_empty_pixels: count_non_empty_pixels(&bytes),
        },
    })
}

pub struct SurfaceRenderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    adapter: GraphicsAdapterInfo,
    max_surface_dimension: u32,
}

impl SurfaceRenderer {
    pub fn new(window: Arc<Window>, size: RenderSize) -> Result<Self, String> {
        size.validate()?;

        let instance = create_instance();
        let surface = instance
            .create_surface(window)
            .map_err(|error| format!("failed to create wgpu surface: {error}"))?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .map_err(|error| format!("failed to request graphics adapter: {error}"))?;
        let adapter_info = adapter_info(&adapter);
        let (device, queue) = request_device(&adapter)?;
        let max_surface_dimension = device.limits().max_texture_dimension_2d;
        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
            .or_else(|| caps.formats.first().copied())
            .ok_or_else(|| String::from("wgpu surface did not report any supported formats"))?;
        let present_mode = if caps.present_modes.contains(&wgpu::PresentMode::Fifo) {
            wgpu::PresentMode::Fifo
        } else {
            caps.present_modes
                .first()
                .copied()
                .ok_or_else(|| String::from("wgpu surface did not report any present modes"))?
        };
        let alpha_mode = caps
            .alpha_modes
            .first()
            .copied()
            .ok_or_else(|| String::from("wgpu surface did not report any alpha modes"))?;

        let size = clamp_surface_size(size, max_surface_dimension);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width,
            height: size.height,
            present_mode,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("wr_render_wgpu::clear_pass"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(CLEAR_PASS_SHADER_WGSL)),
        });
        let pipeline = create_clear_pipeline(&device, format, &shader);

        Ok(Self {
            surface,
            device,
            queue,
            config,
            pipeline,
            adapter: adapter_info,
            max_surface_dimension,
        })
    }

    pub fn adapter(&self) -> &GraphicsAdapterInfo {
        &self.adapter
    }

    pub fn resize(&mut self, size: RenderSize) {
        if size.width == 0 || size.height == 0 {
            return;
        }

        let size = clamp_surface_size(size, self.max_surface_dimension);
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
    }

    pub fn render(&mut self, clear_color: ColorRgba8) -> Result<(), String> {
        let frame = self
            .surface
            .get_current_texture()
            .map_err(|error| format!("failed to acquire next surface texture: {error}"))?;
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("wr_render_wgpu::surface_encoder"),
        });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("wr_render_wgpu::surface_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(color_to_wgpu(clear_color)),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.draw(0..3, 0..1);
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();
        Ok(())
    }

    pub fn render_scene(
        &mut self,
        scene: &ExtractedRenderScene,
        graph: &RenderGraph,
    ) -> Result<(), String> {
        let registry = builtin_scene_registry();
        graph.validate_with_registry(&registry)?;
        let ordered_passes = graph.ordered_pass_names()?;
        let factories: [&dyn RenderPassFactory; 2] =
            [&DebugTrianglePassFactory, &FoliageCardPassFactory];
        let factory_lookup = build_factory_lookup(&factories)?;
        let shader_modules = build_shader_modules(&self.device, &registry)?;
        let pipelines = build_pipelines(
            &self.device,
            self.config.format,
            &registry,
            &shader_modules,
            &factory_lookup,
        )?;
        let frame = self
            .surface
            .get_current_texture()
            .map_err(|error| format!("failed to acquire next surface texture: {error}"))?;
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());

        render_graph_to_target(GraphExecutionContext {
            device: &self.device,
            queue: &self.queue,
            graph,
            scene,
            ordered_passes: &ordered_passes,
            factory_lookup: &factory_lookup,
            pipelines: &pipelines,
            view: &view,
            texture: None,
            size: RenderSize::new(self.config.width, self.config.height),
        })?;

        frame.present();
        Ok(())
    }
}

struct DebugTrianglePassFactory;

impl RenderPassFactory for DebugTrianglePassFactory {
    fn pass_name(&self) -> &'static str {
        DEBUG_TRIANGLE_PASS_NAME
    }

    fn shader_asset(&self) -> ShaderModuleAsset {
        ShaderModuleAsset {
            id: String::from(DEBUG_TRIANGLE_SHADER_ID),
            label: String::from("wr_render_wgpu::debug_triangle"),
            source: ShaderSource::Wgsl(String::from(DEBUG_TRIANGLE_SHADER_WGSL)),
            vertex_entry: String::from("vs_main"),
            fragment_entry: String::from("fs_main"),
        }
    }

    fn pipeline_asset(&self) -> PipelineAssetDescriptor {
        PipelineAssetDescriptor {
            id: String::from(DEBUG_TRIANGLE_PIPELINE_ID),
            shader_id: String::from(DEBUG_TRIANGLE_SHADER_ID),
            topology: PrimitiveTopology::TriangleList,
            color_target: RenderColorSpace::Rgba8UnormSrgb,
        }
    }

    fn create_pipeline(
        &self,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        shader: &wgpu::ShaderModule,
    ) -> wgpu::RenderPipeline {
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("wr_render_wgpu::debug_triangle_layout"),
            bind_group_layouts: &[],
            immediate_size: 0,
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("wr_render_wgpu::debug_triangle_pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("vs_main"),
                buffers: &[debug_vertex_layout()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        })
    }

    fn encode(&self, context: RenderPassEncodeContext<'_>) -> Result<(), String> {
        let vertices = debug_vertex_bytes(context.scene.debug_triangles())?;
        let buffer = context.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("wr_render_wgpu::debug_triangle_vertices"),
            contents: &vertices,
            usage: wgpu::BufferUsages::VERTEX,
        });

        let load_op = if context.load_existing {
            wgpu::LoadOp::Load
        } else {
            wgpu::LoadOp::Clear(color_to_wgpu(context.clear_color))
        };

        let mut pass = context.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("wr_render_wgpu::debug_geometry_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: context.target,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations { load: load_op, store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
            multiview_mask: None,
        });
        pass.set_pipeline(context.pipeline);
        if !vertices.is_empty() {
            pass.set_vertex_buffer(0, buffer.slice(..));
            let vertex_count = u32::try_from(vertices.len() / DEBUG_VERTEX_STRIDE)
                .map_err(|_| String::from("too many debug triangle vertices for one draw call"))?;
            pass.draw(0..vertex_count, 0..1);
        }

        Ok(())
    }
}

struct FoliageCardPassFactory;

impl RenderPassFactory for FoliageCardPassFactory {
    fn pass_name(&self) -> &'static str {
        FOLIAGE_CARD_PASS_NAME
    }

    fn shader_asset(&self) -> ShaderModuleAsset {
        ShaderModuleAsset {
            id: String::from(FOLIAGE_CARD_SHADER_ID),
            label: String::from("wr_render_wgpu::foliage_card"),
            source: ShaderSource::Wgsl(String::from(FOLIAGE_CARD_SHADER_WGSL)),
            vertex_entry: String::from("vs_main"),
            fragment_entry: String::from("fs_main"),
        }
    }

    fn pipeline_asset(&self) -> PipelineAssetDescriptor {
        PipelineAssetDescriptor {
            id: String::from(FOLIAGE_CARD_PIPELINE_ID),
            shader_id: String::from(FOLIAGE_CARD_SHADER_ID),
            topology: PrimitiveTopology::TriangleList,
            color_target: RenderColorSpace::Rgba8UnormSrgb,
        }
    }

    fn create_pipeline(
        &self,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        shader: &wgpu::ShaderModule,
    ) -> wgpu::RenderPipeline {
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("wr_render_wgpu::foliage_card_layout"),
            bind_group_layouts: &[],
            immediate_size: 0,
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("wr_render_wgpu::foliage_card_pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("vs_main"),
                buffers: &[foliage_vertex_layout()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState { cull_mode: None, ..wgpu::PrimitiveState::default() },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        })
    }

    fn encode(&self, context: RenderPassEncodeContext<'_>) -> Result<(), String> {
        let vertices = foliage_vertex_bytes(context.scene.foliage_cards())?;
        let buffer = context.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("wr_render_wgpu::foliage_card_vertices"),
            contents: &vertices,
            usage: wgpu::BufferUsages::VERTEX,
        });

        let load_op = if context.load_existing {
            wgpu::LoadOp::Load
        } else {
            wgpu::LoadOp::Clear(color_to_wgpu(context.clear_color))
        };

        let mut pass = context.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("wr_render_wgpu::foliage_card_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: context.target,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations { load: load_op, store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
            multiview_mask: None,
        });
        pass.set_pipeline(context.pipeline);
        if !vertices.is_empty() {
            pass.set_vertex_buffer(0, buffer.slice(..));
            let vertex_count = u32::try_from(vertices.len() / FOLIAGE_VERTEX_STRIDE)
                .map_err(|_| String::from("too many foliage vertices for one draw call"))?;
            pass.draw(0..vertex_count, 0..1);
        }

        Ok(())
    }
}

struct WgpuContext {
    device: wgpu::Device,
    queue: wgpu::Queue,
    adapter: GraphicsAdapterInfo,
}

impl WgpuContext {
    fn new(compatible_surface: Option<&wgpu::Surface<'_>>) -> Result<Self, String> {
        let instance = create_instance();
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface,
            force_fallback_adapter: false,
        }))
        .map_err(|error| format!("failed to request graphics adapter: {error}"))?;
        let adapter_info = adapter_info(&adapter);
        let (device, queue) = request_device(&adapter)?;

        Ok(Self { device, queue, adapter: adapter_info })
    }

    fn create_clear_shader_module(&self) -> wgpu::ShaderModule {
        self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("wr_render_wgpu::clear_pass"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(CLEAR_PASS_SHADER_WGSL)),
        })
    }
}

fn create_instance() -> wgpu::Instance {
    wgpu::Instance::new(&wgpu::InstanceDescriptor::default())
}

fn request_device(adapter: &wgpu::Adapter) -> Result<(wgpu::Device, wgpu::Queue), String> {
    pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("wr_render_wgpu::device"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::default(),
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
        memory_hints: wgpu::MemoryHints::Performance,
        trace: wgpu::Trace::Off,
    }))
    .map_err(|error| format!("failed to request wgpu device: {error}"))
}

fn adapter_info(adapter: &wgpu::Adapter) -> GraphicsAdapterInfo {
    let info = adapter.get_info();

    GraphicsAdapterInfo {
        backend: format!("{:?}", info.backend).to_ascii_lowercase(),
        name: info.name,
        device_type: format!("{:?}", info.device_type).to_ascii_lowercase(),
        driver: Some(info.driver),
        driver_info: Some(info.driver_info),
        shading_language: String::from("wgsl"),
    }
}

fn create_render_texture(
    device: &wgpu::Device,
    size: RenderSize,
    format: wgpu::TextureFormat,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("wr_render_wgpu::offscreen_texture"),
        size: wgpu::Extent3d { width: size.width, height: size.height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}

fn create_clear_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    shader: &wgpu::ShaderModule,
) -> wgpu::RenderPipeline {
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("wr_render_wgpu::pipeline_layout"),
        bind_group_layouts: &[],
        immediate_size: 0,
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("wr_render_wgpu::clear_pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}

fn registry_from_factories(factories: &[&dyn RenderPassFactory]) -> RenderFeatureRegistry {
    RenderFeatureRegistry {
        shaders: factories.iter().map(|factory| factory.shader_asset()).collect(),
        pipelines: factories.iter().map(|factory| factory.pipeline_asset()).collect(),
    }
}

fn build_factory_lookup<'a>(
    factories: &'a [&dyn RenderPassFactory],
) -> Result<BTreeMap<&'a str, &'a dyn RenderPassFactory>, String> {
    let mut lookup = BTreeMap::new();
    for factory in factories {
        if lookup.insert(factory.pass_name(), *factory).is_some() {
            return Err(format!(
                "render pass factory `{}` was registered twice",
                factory.pass_name()
            ));
        }
    }

    Ok(lookup)
}

fn build_shader_modules(
    device: &wgpu::Device,
    registry: &RenderFeatureRegistry,
) -> Result<BTreeMap<String, wgpu::ShaderModule>, String> {
    let mut modules = BTreeMap::new();

    for shader in &registry.shaders {
        let module = match &shader.source {
            ShaderSource::Wgsl(source) => {
                device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some(shader.label.as_str()),
                    source: wgpu::ShaderSource::Wgsl(Cow::Owned(source.clone())),
                })
            }
        };
        modules.insert(shader.id.clone(), module);
    }

    Ok(modules)
}

fn build_pipelines<'a>(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    registry: &RenderFeatureRegistry,
    shader_modules: &BTreeMap<String, wgpu::ShaderModule>,
    factory_lookup: &BTreeMap<&'a str, &'a dyn RenderPassFactory>,
) -> Result<BTreeMap<String, (&'a dyn RenderPassFactory, wgpu::RenderPipeline)>, String> {
    let mut pipelines = BTreeMap::new();

    for pipeline in &registry.pipelines {
        let Some(factory) =
            factory_lookup.values().find(|factory| factory.pipeline_asset().id == pipeline.id)
        else {
            return Err(format!(
                "no render pass factory was registered for pipeline `{}`",
                pipeline.id
            ));
        };
        let shader = shader_modules
            .get(&pipeline.shader_id)
            .ok_or_else(|| format!("shader module `{}` was not compiled", pipeline.shader_id))?;
        let gpu_pipeline = factory.create_pipeline(device, format, shader);
        pipelines.insert(pipeline.id.clone(), (*factory, gpu_pipeline));
    }

    Ok(pipelines)
}

struct GraphExecutionContext<'a> {
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
    graph: &'a RenderGraph,
    scene: &'a ExtractedRenderScene,
    ordered_passes: &'a [String],
    factory_lookup: &'a BTreeMap<&'a str, &'a dyn RenderPassFactory>,
    pipelines: &'a BTreeMap<String, (&'a dyn RenderPassFactory, wgpu::RenderPipeline)>,
    view: &'a wgpu::TextureView,
    texture: Option<&'a wgpu::Texture>,
    size: RenderSize,
}

fn render_graph_to_target<'a>(context: GraphExecutionContext<'a>) -> Result<(), String> {
    let mut encoder = context.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("wr_render_wgpu::offscreen_encoder"),
    });

    for (index, pass_name) in context.ordered_passes.iter().enumerate() {
        let pass = context.graph.pass(pass_name).ok_or_else(|| {
            format!("validated render pass `{pass_name}` disappeared during execution")
        })?;
        let factory = context
            .factory_lookup
            .get(pass.name.as_str())
            .copied()
            .ok_or_else(|| format!("no render pass factory registered for `{}`", pass.name))?;
        let (pipeline_factory, pipeline) = context
            .pipelines
            .get(&pass.pipeline_id)
            .ok_or_else(|| format!("render pipeline `{}` was not built", pass.pipeline_id))?;

        if pipeline_factory.pass_name() != pass.name {
            return Err(format!(
                "render pass `{}` was wired to pipeline `{}` owned by `{}`",
                pass.name,
                pass.pipeline_id,
                pipeline_factory.pass_name()
            ));
        }

        factory.encode(RenderPassEncodeContext {
            scene: context.scene,
            device: context.device,
            encoder: &mut encoder,
            target: context.view,
            pipeline,
            clear_color: context.scene.clear_color,
            load_existing: index > 0,
        })?;
    }

    context.queue.submit(Some(encoder.finish()));
    Ok(())
}

fn render_graph_to_rgba8<'a>(context: GraphExecutionContext<'a>) -> Result<Vec<u8>, String> {
    render_graph_to_target(GraphExecutionContext {
        device: context.device,
        queue: context.queue,
        graph: context.graph,
        scene: context.scene,
        ordered_passes: context.ordered_passes,
        factory_lookup: context.factory_lookup,
        pipelines: context.pipelines,
        view: context.view,
        texture: None,
        size: context.size,
    })?;

    let texture = context
        .texture
        .ok_or_else(|| String::from("offscreen graph execution requires a texture target"))?;
    read_texture_to_rgba8(context.device, context.queue, texture, context.size)
}

fn read_texture_to_rgba8(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    size: RenderSize,
) -> Result<Vec<u8>, String> {
    let bytes_per_pixel = 4_u32;
    let unpadded_bytes_per_row = size.width.saturating_mul(bytes_per_pixel);
    let padded_bytes_per_row = align_to(unpadded_bytes_per_row, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let buffer_size = u64::from(padded_bytes_per_row) * u64::from(size.height);
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("wr_render_wgpu::readback"),
        size: buffer_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("wr_render_wgpu::readback_encoder"),
    });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_bytes_per_row),
                rows_per_image: Some(size.height),
            },
        },
        wgpu::Extent3d { width: size.width, height: size.height, depth_or_array_layers: 1 },
    );

    queue.submit(Some(encoder.finish()));

    let slice = readback.slice(..);
    let (sender, receiver) = mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    let _ = device.poll(wgpu::PollType::wait_indefinitely());
    receiver
        .recv()
        .map_err(|error| format!("failed to receive readback completion: {error}"))?
        .map_err(|error| format!("failed to map readback buffer: {error}"))?;

    let data = slice.get_mapped_range();
    let mut pixels =
        vec![0_u8; size.width as usize * size.height as usize * bytes_per_pixel as usize];
    for row in 0..size.height as usize {
        let source_start = row * padded_bytes_per_row as usize;
        let source_end = source_start + unpadded_bytes_per_row as usize;
        let destination_start = row * unpadded_bytes_per_row as usize;
        let destination_end = destination_start + unpadded_bytes_per_row as usize;
        pixels[destination_start..destination_end].copy_from_slice(&data[source_start..source_end]);
    }
    drop(data);
    readback.unmap();

    Ok(pixels)
}

fn write_png(path: &Path, size: RenderSize, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create PNG parent directory: {error}"))?;
    }

    let file = std::fs::File::create(path)
        .map_err(|error| format!("failed to create PNG output: {error}"))?;
    let encoder = image::codecs::png::PngEncoder::new(file);
    encoder
        .write_image(bytes, size.width, size.height, image::ExtendedColorType::Rgba8)
        .map_err(|error| format!("failed to encode PNG: {error}"))
}

fn count_non_empty_pixels(bytes: &[u8]) -> u64 {
    bytes.chunks_exact(4).filter(|pixel| pixel.iter().any(|component| *component != 0)).count()
        as u64
}

fn align_to(value: u32, alignment: u32) -> u32 {
    value.div_ceil(alignment) * alignment
}

fn clamp_surface_size(size: RenderSize, max_dimension: u32) -> RenderSize {
    RenderSize::new(size.width.min(max_dimension), size.height.min(max_dimension))
}

fn color_to_wgpu(color: ColorRgba8) -> wgpu::Color {
    let normalized = color.normalized();
    wgpu::Color { r: normalized[0], g: normalized[1], b: normalized[2], a: normalized[3] }
}

const DEBUG_VERTEX_STRIDE: usize = std::mem::size_of::<f32>() * 7;
const FOLIAGE_VERTEX_STRIDE: usize =
    (std::mem::size_of::<f32>() * 8) + (std::mem::size_of::<u32>() * 2);

fn debug_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: DEBUG_VERTEX_STRIDE as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: 0,
                shader_location: 0,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x4,
                offset: (std::mem::size_of::<f32>() * 3) as u64,
                shader_location: 1,
            },
        ],
    }
}

fn foliage_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: FOLIAGE_VERTEX_STRIDE as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: 0,
                shader_location: 0,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: (std::mem::size_of::<f32>() * 3) as u64,
                shader_location: 1,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: (std::mem::size_of::<f32>() * 5) as u64,
                shader_location: 2,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Uint32x2,
                offset: (std::mem::size_of::<f32>() * 8) as u64,
                shader_location: 3,
            },
        ],
    }
}

fn debug_vertex_bytes<'a>(
    triangles: impl Iterator<Item = &'a DebugTriangle>,
) -> Result<Vec<u8>, String> {
    let triangle_list = triangles.collect::<Vec<_>>();
    let capacity = triangle_list
        .len()
        .checked_mul(3)
        .and_then(|vertices| vertices.checked_mul(DEBUG_VERTEX_STRIDE))
        .ok_or_else(|| String::from("debug triangle vertex buffer overflow"))?;
    let mut bytes = Vec::with_capacity(capacity);

    for triangle in triangle_list {
        for vertex in triangle.vertices {
            bytes.extend_from_slice(&vertex.position.x.to_le_bytes());
            bytes.extend_from_slice(&vertex.position.y.to_le_bytes());
            bytes.extend_from_slice(&vertex.position.z.to_le_bytes());
            bytes.extend_from_slice(&vertex.color.red.to_le_bytes());
            bytes.extend_from_slice(&vertex.color.green.to_le_bytes());
            bytes.extend_from_slice(&vertex.color.blue.to_le_bytes());
            bytes.extend_from_slice(&vertex.color.alpha.to_le_bytes());
        }
    }

    Ok(bytes)
}

fn foliage_vertex_bytes<'a>(
    cards: impl Iterator<Item = &'a FoliageCard>,
) -> Result<Vec<u8>, String> {
    let card_list = cards.collect::<Vec<_>>();
    let capacity = card_list
        .len()
        .checked_mul(6)
        .and_then(|vertices| vertices.checked_mul(FOLIAGE_VERTEX_STRIDE))
        .ok_or_else(|| String::from("foliage vertex buffer overflow"))?;
    let mut bytes = Vec::with_capacity(capacity);

    for card in card_list {
        for vertex in triangulate_card(card) {
            bytes.extend_from_slice(&vertex.position.x.to_le_bytes());
            bytes.extend_from_slice(&vertex.position.y.to_le_bytes());
            bytes.extend_from_slice(&vertex.position.z.to_le_bytes());
            bytes.extend_from_slice(&vertex.uv[0].to_le_bytes());
            bytes.extend_from_slice(&vertex.uv[1].to_le_bytes());
            bytes.extend_from_slice(&vertex.normal.x.to_le_bytes());
            bytes.extend_from_slice(&vertex.normal.y.to_le_bytes());
            bytes.extend_from_slice(&vertex.normal.z.to_le_bytes());
            bytes.extend_from_slice(&vertex.packed_material_params[0].to_le_bytes());
            bytes.extend_from_slice(&vertex.packed_material_params[1].to_le_bytes());
        }
    }

    Ok(bytes)
}

fn triangulate_card(card: &FoliageCard) -> [wr_render_api::FoliageCardVertex; 6] {
    [
        card.vertices[0],
        card.vertices[1],
        card.vertices[2],
        card.vertices[0],
        card.vertices[2],
        card.vertices[3],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use wr_render_api::{
        DebugVertex, FoliageCard, FoliageCardVertex, LinearColor, RenderPassNode, ScenePrimitive,
        Vec3,
    };

    struct DummyFactory;

    impl RenderPassFactory for DummyFactory {
        fn pass_name(&self) -> &'static str {
            "dummy"
        }

        fn shader_asset(&self) -> ShaderModuleAsset {
            ShaderModuleAsset {
                id: String::from("dummy_shader"),
                label: String::from("dummy"),
                source: ShaderSource::Wgsl(String::from(DEBUG_TRIANGLE_SHADER_WGSL)),
                vertex_entry: String::from("vs_main"),
                fragment_entry: String::from("fs_main"),
            }
        }

        fn pipeline_asset(&self) -> PipelineAssetDescriptor {
            PipelineAssetDescriptor {
                id: String::from("dummy_pipeline"),
                shader_id: String::from("dummy_shader"),
                topology: PrimitiveTopology::TriangleList,
                color_target: RenderColorSpace::Rgba8UnormSrgb,
            }
        }

        fn create_pipeline(
            &self,
            device: &wgpu::Device,
            format: wgpu::TextureFormat,
            shader: &wgpu::ShaderModule,
        ) -> wgpu::RenderPipeline {
            DebugTrianglePassFactory.create_pipeline(device, format, shader)
        }

        fn encode(&self, context: RenderPassEncodeContext<'_>) -> Result<(), String> {
            DebugTrianglePassFactory.encode(context)
        }
    }

    fn assert_render_pass_factory<T: RenderPassFactory>() {}

    fn sample_scene() -> ExtractedRenderScene {
        let triangle = DebugTriangle::new([
            DebugVertex::new(Vec3::new(-0.8, -0.8, 0.0), LinearColor::new(1.0, 0.0, 0.0, 1.0)),
            DebugVertex::new(Vec3::new(0.8, -0.8, 0.0), LinearColor::new(0.0, 1.0, 0.0, 1.0)),
            DebugVertex::new(Vec3::new(0.0, 0.8, 0.0), LinearColor::new(0.0, 0.0, 1.0, 1.0)),
        ]);
        let mut scene = ExtractedRenderScene::new(ColorRgba8::new(12, 18, 24, 255));
        scene.push_primitive(ScenePrimitive::DebugTriangle(triangle));
        scene.push_primitive(ScenePrimitive::FoliageCard(FoliageCard::new([
            FoliageCardVertex::new(
                Vec3::new(-0.35, -0.15, 0.02),
                [0.0, 0.0],
                Vec3::new(0.0, 0.0, 1.0),
                [0xA0607040, 0x0000C080],
            ),
            FoliageCardVertex::new(
                Vec3::new(0.35, -0.15, 0.02),
                [1.0, 0.0],
                Vec3::new(0.0, 0.0, 1.0),
                [0xA0607040, 0x0000C080],
            ),
            FoliageCardVertex::new(
                Vec3::new(0.35, 0.55, 0.02),
                [1.0, 1.0],
                Vec3::new(0.0, 0.0, 1.0),
                [0xA0607040, 0x0000C080],
            ),
            FoliageCardVertex::new(
                Vec3::new(-0.35, 0.55, 0.02),
                [0.0, 1.0],
                Vec3::new(0.0, 0.0, 1.0),
                [0xA0607040, 0x0000C080],
            ),
        ])));
        scene
    }

    #[test]
    fn clear_pass_shader_compiles() {
        let adapter = compile_clear_pass_shader().expect("clear pass shader should compile");

        assert_eq!(adapter.shading_language, "wgsl");
        assert!(!adapter.name.is_empty());
    }

    #[test]
    fn builtin_registry_validates() {
        builtin_scene_registry()
            .validate()
            .expect("builtin render factories should register valid assets");
    }

    #[test]
    fn compile_time_factory_contract_accepts_custom_factories() {
        assert_render_pass_factory::<DummyFactory>();
    }

    #[test]
    fn offscreen_capture_writes_png() {
        let temp = tempdir().expect("tempdir should exist");
        let output_path = temp.path().join("capture.png");
        let outcome = render_offscreen_png(
            &OffscreenRenderRequest {
                size: RenderSize::new(96, 64),
                clear_color: ColorRgba8::new(48, 96, 160, 255),
            },
            &output_path,
        )
        .expect("offscreen render should succeed");

        assert!(output_path.exists());
        assert_eq!(outcome.frame.size, RenderSize::new(96, 64));
        assert_eq!(outcome.frame.color_space, RenderColorSpace::Rgba8UnormSrgb);
        assert!(outcome.frame.non_empty_pixels > 0);
    }

    #[test]
    fn rendered_scene_png_contains_triangle_pixels() {
        let temp = tempdir().expect("tempdir should exist");
        let output_path = temp.path().join("scene.png");
        render_scene_to_png(
            RenderSize::new(96, 96),
            &sample_scene(),
            &debug_triangle_graph(),
            &output_path,
        )
        .expect("scene render should succeed");

        let image = image::open(&output_path).expect("png should load").into_rgba8();
        let center = image.get_pixel(48, 48).0;

        assert_eq!(image.width(), 96);
        assert_eq!(image.height(), 96);
        assert_ne!(center, [12, 18, 24, 255]);
    }

    #[test]
    fn rendered_scene_png_contains_foliage_pixels() {
        let temp = tempdir().expect("tempdir should exist");
        let output_path = temp.path().join("foliage.png");
        render_scene_to_png(
            RenderSize::new(96, 96),
            &sample_scene(),
            &wr_render_api::debug_triangle_and_foliage_graph(),
            &output_path,
        )
        .expect("scene render should succeed");

        let image = image::open(&output_path).expect("png should load").into_rgba8();
        let foliage_pixel = image.get_pixel(48, 28).0;

        assert_ne!(foliage_pixel, [12, 18, 24, 255]);
    }

    #[test]
    fn scene_render_supports_custom_factory_registration() {
        let temp = tempdir().expect("tempdir should exist");
        let output_path = temp.path().join("custom.png");
        let graph = RenderGraph::default()
            .declare_resource(wr_render_api::RenderGraphResource::new(
                wr_render_api::DEBUG_COLOR_TARGET_RESOURCE,
                wr_render_api::RenderResourceKind::ColorTarget,
                true,
            ))
            .add_pass(
                RenderPassNode::new("dummy", "dummy_pipeline")
                    .writes(wr_render_api::DEBUG_COLOR_TARGET_RESOURCE),
            );

        render_scene_to_png_with_factories(
            RenderSize::new(64, 64),
            &sample_scene(),
            &graph,
            &[&DummyFactory],
            &output_path,
        )
        .expect("custom render factory should be executable");

        assert!(output_path.exists());
    }
}
