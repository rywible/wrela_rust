#![forbid(unsafe_code)]

use std::borrow::Cow;
use std::path::Path;
use std::sync::{Arc, mpsc};

use image::ImageEncoder;
use winit::window::Window;
use wr_core::{CrateBoundary, CrateEntryPoint};
use wr_render_api::{
    CapturedFrameInfo, ColorRgba8, GraphicsAdapterInfo, OffscreenRenderRequest, RenderColorSpace,
    RenderSize,
};

const CLEAR_PASS_SHADER_WGSL: &str = include_str!("clear_pass.wgsl");
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
    let _shader = gpu.create_shader_module();
    Ok(gpu.adapter)
}

pub fn render_offscreen_png(
    request: &OffscreenRenderRequest,
    output_path: impl AsRef<Path>,
) -> Result<OffscreenCaptureOutcome, String> {
    request.validate()?;

    let gpu = WgpuContext::new(None)?;
    let shader = gpu.create_shader_module();
    let texture = create_render_texture(&gpu.device, request.size, TEXTURE_FORMAT);
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let pipeline = create_render_pipeline(&gpu.device, TEXTURE_FORMAT, &shader);

    let bytes = render_to_rgba8(
        &gpu.device,
        &gpu.queue,
        &pipeline,
        &view,
        &texture,
        request.size,
        request.clear_color,
    )?;
    write_png(output_path.as_ref(), request.size, &bytes)?;

    Ok(OffscreenCaptureOutcome {
        adapter: gpu.adapter,
        frame: CapturedFrameInfo {
            size: request.size,
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
        let pipeline = create_render_pipeline(&device, format, &shader);

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
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: clear_color.normalized()[0],
                            g: clear_color.normalized()[1],
                            b: clear_color.normalized()[2],
                            a: clear_color.normalized()[3],
                        }),
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

    fn create_shader_module(&self) -> wgpu::ShaderModule {
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

fn create_render_pipeline(
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

fn render_to_rgba8(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    pipeline: &wgpu::RenderPipeline,
    view: &wgpu::TextureView,
    texture: &wgpu::Texture,
    size: RenderSize,
    clear_color: ColorRgba8,
) -> Result<Vec<u8>, String> {
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("wr_render_wgpu::offscreen_encoder"),
    });

    {
        let color = clear_color.normalized();
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("wr_render_wgpu::offscreen_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: color[0],
                        g: color[1],
                        b: color[2],
                        a: color[3],
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
            multiview_mask: None,
        });
        pass.set_pipeline(pipeline);
        pass.draw(0..3, 0..1);
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn clear_pass_shader_compiles() {
        let adapter = compile_clear_pass_shader().expect("clear pass shader should compile");

        assert_eq!(adapter.shading_language, "wgsl");
        assert!(!adapter.name.is_empty());
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
    fn captured_image_has_expected_dimensions_and_visible_pixels() {
        let temp = tempdir().expect("tempdir should exist");
        let output_path = temp.path().join("image.png");
        render_offscreen_png(
            &OffscreenRenderRequest {
                size: RenderSize::new(80, 48),
                clear_color: ColorRgba8::new(64, 120, 208, 255),
            },
            &output_path,
        )
        .expect("offscreen render should succeed");

        let image = image::open(&output_path).expect("png should load").into_rgba8();

        assert_eq!(image.width(), 80);
        assert_eq!(image.height(), 48);
        assert!(image.pixels().any(|pixel| pixel.0 != [0, 0, 0, 0]));
    }
}
