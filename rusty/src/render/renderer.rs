use crate::World;
use crate::render::components::{SpriteComponent, TextureId, Transform2D};
use bytemuck::{Pod, Zeroable};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::path::Path;
use wgpu::util::DeviceExt;
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use winit::window::Window;

const QUAD_VERTICES: &[f32] = &[
    0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 1.0, 0.0, 1.0,
];

const QUAD_INDICES: &[u16] = &[0, 1, 2, 2, 3, 0];

/// Rendering errors produced by the 2D renderer.
#[derive(Debug)]
pub enum RenderError {
    /// No compatible GPU adapter was found.
    AdapterNotFound,
    /// GPU device creation failed.
    DeviceRequest(String),
    /// Surface creation failed for the target window.
    SurfaceCreation(String),
    /// No compatible surface format is available.
    SurfaceFormatUnavailable,
    /// Failed to acquire the current surface texture.
    SurfaceAcquire(String),
    /// Image bytes failed to decode.
    TextureDecode(image::ImageError),
    /// Underlying I/O operation failed.
    Io(std::io::Error),
    /// Raw texture byte slice length does not match width/height expectations.
    InvalidTextureDataLength { expected: usize, actual: usize },
    /// A sprite references a texture id that is not loaded.
    MissingTexture(TextureId),
}

impl Display for RenderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AdapterNotFound => write!(f, "no compatible GPU adapter found"),
            Self::DeviceRequest(e) => write!(f, "failed to create device: {e}"),
            Self::SurfaceCreation(e) => write!(f, "failed to create surface: {e}"),
            Self::SurfaceFormatUnavailable => write!(f, "no compatible surface format found"),
            Self::SurfaceAcquire(e) => write!(f, "surface error: {e}"),
            Self::TextureDecode(e) => write!(f, "texture decode error: {e}"),
            Self::Io(e) => write!(f, "io error: {e}"),
            Self::InvalidTextureDataLength { expected, actual } => {
                write!(f, "invalid texture data length: expected {expected}, got {actual}")
            }
            Self::MissingTexture(id) => write!(f, "missing texture for id {}", id.0),
        }
    }
}

impl std::error::Error for RenderError {}

impl From<std::io::Error> for RenderError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<image::ImageError> for RenderError {
    fn from(value: image::ImageError) -> Self {
        Self::TextureDecode(value)
    }
}

/// Mutable 2D camera used for orthographic projection.
#[derive(Debug, Clone)]
pub struct Camera2D {
    /// Camera world position.
    pub position: (f32, f32),
    /// Zoom multiplier (`>= 0.01`).
    pub zoom: f32,
    /// Viewport dimensions in pixels.
    pub viewport: (u32, u32),
}

impl Camera2D {
    fn new(viewport: (u32, u32)) -> Self {
        Self {
            position: (0.0, 0.0),
            zoom: 1.0,
            viewport,
        }
    }

    /// Moves the camera by a delta.
    pub fn move_by(&mut self, dx: f32, dy: f32) {
        self.position.0 += dx;
        self.position.1 += dy;
    }

    /// Sets camera world position.
    pub fn set_position(&mut self, x: f32, y: f32) {
        self.position = (x, y);
    }

    /// Sets camera zoom, clamped to `0.01` minimum.
    pub fn set_zoom(&mut self, z: f32) {
        self.zoom = z.max(0.01);
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    pos: [f32; 2],
    uv: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct InstanceGpu {
    world_pos: [f32; 2],
    rotation: f32,
    _pad0: f32,
    scale: [f32; 2],
    draw_size: [f32; 2],
    uv_rect: [f32; 4],
    tint: [f32; 4],
    z: i32,
    _pad1: [u32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct CameraUniform {
    camera_pos: [f32; 2],
    zoom: f32,
    _pad0: f32,
    viewport: [f32; 2],
    _pad1: [f32; 2],
}

#[derive(Clone)]
pub(crate) struct SpriteBatchItem {
    pub texture_id: TextureId,
    pub z: i32,
    pub world_pos: (f32, f32),
    pub rotation: f32,
    pub scale: (f32, f32),
    pub draw_size: (f32, f32),
    pub src_rect: Option<[f32; 4]>,
    pub tint: [f32; 4],
}

struct GpuTexture {
    bind_group: wgpu::BindGroup,
    width: u32,
    height: u32,
}

struct TextureRegistry {
    textures: HashMap<TextureId, GpuTexture>,
}

impl TextureRegistry {
    fn new() -> Self {
        Self {
            textures: HashMap::new(),
        }
    }

    //noinspection ALL
    fn len(&self) -> usize {
        self.textures.len()
    }

    fn get(&self, id: TextureId) -> Option<&GpuTexture> {
        self.textures.get(&id)
    }

    fn insert(&mut self, id: TextureId, tex: GpuTexture) {
        self.textures.insert(id, tex);
    }
}

/// 2D sprite renderer backed by `wgpu`.
pub struct Renderer2D {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    camera_bind_group: wgpu::BindGroup,
    camera_buffer: wgpu::Buffer,
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    texture_registry: TextureRegistry,
    camera: Camera2D,
    background: [f32; 4],
}

impl Renderer2D {
    /// Creates a new renderer for a `winit` window.
    pub fn new(window: &Window) -> Result<Self, RenderError> {
        pollster::block_on(Self::new_async(window))
    }

    /// Async constructor used internally by [`Self::new`].
    async fn new_async(window: &Window) -> Result<Self, RenderError> {
        let size = window.inner_size();
        let instance = wgpu::Instance::default();
        let raw_display_handle = window
            .display_handle()
            .map_err(|e| RenderError::SurfaceCreation(e.to_string()))?
            .as_raw();
        let raw_window_handle = window
            .window_handle()
            .map_err(|e| RenderError::SurfaceCreation(e.to_string()))?
            .as_raw();
        let target = wgpu::SurfaceTargetUnsafe::RawHandle {
            raw_display_handle: Some(raw_display_handle),
            raw_window_handle,
        };
        let surface = {
            // SAFETY: Raw handles are fetched from a live `winit::window::Window` and remain valid
            // while the window outlives the renderer.
            let value = unsafe { instance.create_surface_unsafe(target) }
                .map_err(|e| RenderError::SurfaceCreation(e.to_string()))?;
            value
        };

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .map_err(|_| RenderError::AdapterNotFound)?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("renderer2d-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                trace: wgpu::Trace::Off,
            })
            .await
            .map_err(|e| RenderError::DeviceRequest(e.to_string()))?;

        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
            .or_else(|| caps.formats.first().copied())
            .ok_or(RenderError::SurfaceFormatUnavailable)?;

        let present_mode = if caps.present_modes.contains(&wgpu::PresentMode::Immediate) {
            wgpu::PresentMode::Immediate
        } else if caps.present_modes.contains(&wgpu::PresentMode::Mailbox) {
            wgpu::PresentMode::Mailbox
        } else {
            caps.present_modes
                .first()
                .copied()
                .unwrap_or(wgpu::PresentMode::Fifo)
        };

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode,
            alpha_mode: caps
                .alpha_modes
                .first()
                .copied()
                .unwrap_or(wgpu::CompositeAlphaMode::Opaque),
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("sprite-texture-layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("camera-layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera-buffer"),
            contents: bytemuck::bytes_of(&CameraUniform {
                camera_pos: [0.0, 0.0],
                zoom: 1.0,
                _pad0: 0.0,
                viewport: [size.width.max(1) as f32, size.height.max(1) as f32],
                _pad1: [0.0, 0.0],
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera-bind-group"),
            layout: &camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sprite-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/sprite.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sprite-pipeline-layout"),
            bind_group_layouts: &[Some(&texture_bind_group_layout), Some(&camera_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sprite-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: size_of::<Vertex>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[
                            wgpu::VertexAttribute {
                                offset: 0,
                                shader_location: 0,
                                format: wgpu::VertexFormat::Float32x2,
                            },
                            wgpu::VertexAttribute {
                                offset: size_of::<[f32; 2]>() as wgpu::BufferAddress,
                                shader_location: 1,
                                format: wgpu::VertexFormat::Float32x2,
                            },
                        ],
                    },
                    wgpu::VertexBufferLayout {
                        array_stride: size_of::<InstanceGpu>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &[
                            wgpu::VertexAttribute { offset: 0, shader_location: 2, format: wgpu::VertexFormat::Float32x2 },
                            wgpu::VertexAttribute { offset: 8, shader_location: 3, format: wgpu::VertexFormat::Float32 },
                            wgpu::VertexAttribute { offset: 16, shader_location: 4, format: wgpu::VertexFormat::Float32x2 },
                            wgpu::VertexAttribute { offset: 24, shader_location: 5, format: wgpu::VertexFormat::Float32x2 },
                            wgpu::VertexAttribute { offset: 32, shader_location: 6, format: wgpu::VertexFormat::Float32x4 },
                            wgpu::VertexAttribute { offset: 48, shader_location: 7, format: wgpu::VertexFormat::Float32x4 },
                            wgpu::VertexAttribute { offset: 64, shader_location: 8, format: wgpu::VertexFormat::Sint32 },
                        ],
                    },
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let vertices: Vec<Vertex> = QUAD_VERTICES
            .chunks_exact(4)
            .map(|c| Vertex {
                pos: [c[0], c[1]],
                uv: [c[2], c[3]],
            })
            .collect();

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("quad-vertex-buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("quad-index-buffer"),
            contents: bytemuck::cast_slice(QUAD_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        Ok(Self {
            device,
            queue,
            surface,
            config,
            texture_bind_group_layout,
            camera_bind_group,
            camera_buffer,
            pipeline,
            vertex_buffer,
            index_buffer,
            num_indices: QUAD_INDICES.len() as u32,
            texture_registry: TextureRegistry::new(),
            camera: Camera2D::new((size.width.max(1), size.height.max(1))),
            background: [0.0, 0.0, 0.0, 1.0],
        })
    }

    /// Sets the clear color used for frame rendering.
    pub fn set_background(&mut self, color: [f32; 4]) {
        self.background = color;
    }

    //noinspection ALL
    pub(crate) fn background(&self) -> [f32; 4] {
        self.background
    }

    /// Returns a mutable camera reference.
    pub fn camera_mut(&mut self) -> &mut Camera2D {
        &mut self.camera
    }

    fn update_camera_uniform(&self) -> CameraUniform {
        CameraUniform {
            camera_pos: [self.camera.position.0, self.camera.position.1],
            zoom: self.camera.zoom,
            _pad0: 0.0,
            viewport: [self.camera.viewport.0 as f32, self.camera.viewport.1 as f32],
            _pad1: [0.0, 0.0],
        }
    }

    /// Loads an image file into the texture registry.
    pub fn load_texture(&mut self, id: TextureId, path: &str) -> Result<(), RenderError> {
        let bytes = std::fs::read(Path::new(path))?;
        let image = image::load_from_memory(&bytes)?.to_rgba8();
        let (w, h) = image.dimensions();
        self.load_texture_rgba(id, w, h, image.as_raw())
    }

    /// Loads raw RGBA bytes into the texture registry.
    pub fn load_texture_rgba(
        &mut self,
        id: TextureId,
        w: u32,
        h: u32,
        data: &[u8],
    ) -> Result<(), RenderError> {
        let expected = (w as usize)
            .checked_mul(h as usize)
            .and_then(|v| v.checked_mul(4))
            .ok_or(RenderError::InvalidTextureDataLength {
                expected: usize::MAX,
                actual: data.len(),
            })?;
        if data.len() != expected {
            return Err(RenderError::InvalidTextureDataLength {
                expected,
                actual: data.len(),
            });
        }

        let size = wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("sprite-texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * w),
                rows_per_image: Some(h),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sprite-texture-bind-group"),
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        self.texture_registry
            .insert(id, GpuTexture { bind_group, width: w, height: h });
        Ok(())
    }

    /// Reconfigures render targets for a resized viewport.
    pub fn resize(&mut self, w: u32, h: u32) {
        self.config.width = w.max(1);
        self.config.height = h.max(1);
        self.surface.configure(&self.device, &self.config);
        self.camera.viewport = (self.config.width, self.config.height);
    }

    pub(crate) fn texture_count(&self) -> usize {
        self.texture_registry.len()
    }

    pub(crate) fn has_texture(&self, id: TextureId) -> bool {
        self.texture_registry.get(id).is_some()
    }

    /// Renders all visible `Transform2D + SpriteComponent` entities.
    pub fn render_world(&mut self, world: &World) -> Result<(), RenderError> {
        let batch = collect_visible_sprites(world);
        if batch.is_empty() {
            return self.render_batch(&[]);
        }
        let mut gpu_instances = Vec::with_capacity(batch.len());
        for item in &batch {
            let tex = self
                .texture_registry
                .get(item.texture_id)
                .ok_or(RenderError::MissingTexture(item.texture_id))?;
            let uv_rect = item
                .src_rect
                .map(|rect| {
                    [
                        rect[0] / tex.width as f32,
                        rect[1] / tex.height as f32,
                        rect[2] / tex.width as f32,
                        rect[3] / tex.height as f32,
                    ]
                })
                .unwrap_or([0.0, 0.0, 1.0, 1.0]);
            gpu_instances.push(InstanceGpu {
                world_pos: [item.world_pos.0, item.world_pos.1],
                rotation: item.rotation,
                _pad0: 0.0,
                scale: [item.scale.0, item.scale.1],
                draw_size: [item.draw_size.0, item.draw_size.1],
                uv_rect,
                tint: item.tint,
                z: item.z,
                _pad1: [0, 0, 0],
            });
        }
        self.render_batch_with_order(&gpu_instances, &batch)
    }

    fn render_batch(&mut self, instances: &[InstanceGpu]) -> Result<(), RenderError> {
        let empty_order: Vec<SpriteBatchItem> = Vec::new();
        self.render_batch_with_order(instances, &empty_order)
    }

    fn render_batch_with_order(
        &mut self,
        instances: &[InstanceGpu],
        order: &[SpriteBatchItem],
    ) -> Result<(), RenderError> {
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err(RenderError::SurfaceAcquire(
                    "surface validation error while acquiring current texture".to_string(),
                ));
            }
        };
        {
            let view = frame
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            let mut encoder =
                self.device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("renderer2d-encoder"),
                    });

            let instance_buffer = if instances.is_empty() {
                None
            } else {
                Some(
                    self.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("sprite-instance-buffer"),
                            contents: bytemuck::cast_slice(instances),
                            usage: wgpu::BufferUsages::VERTEX,
                        }),
                )
            };
            self.queue.write_buffer(
                &self.camera_buffer,
                0,
                bytemuck::bytes_of(&self.update_camera_uniform()),
            );

            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("renderer2d-pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        depth_slice: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: self.background[0] as f64,
                                g: self.background[1] as f64,
                                b: self.background[2] as f64,
                                a: self.background[3] as f64,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                pass.set_pipeline(&self.pipeline);
                pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                if let Some(instance_buffer) = &instance_buffer {
                    pass.set_vertex_buffer(1, instance_buffer.slice(..));
                }
                pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

                for (i, item) in order.iter().enumerate() {
                    if let Some(texture) = self.texture_registry.get(item.texture_id) {
                        pass.set_bind_group(0, &texture.bind_group, &[]);
                        pass.set_bind_group(1, &self.camera_bind_group, &[]);
                        let idx = i as u32;
                        pass.draw_indexed(0..self.num_indices, 0, idx..(idx + 1));
                    }
                }
            }

            self.queue.submit(std::iter::once(encoder.finish()));
        }
        frame.present();
        Ok(())
    }
}

impl Drop for Renderer2D {
    fn drop(&mut self) {
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
    }
}

pub(crate) fn collect_visible_sprites(world: &World) -> Vec<SpriteBatchItem> {
    let mut items = Vec::new();
    for entity in world.query_entities::<Transform2D>() {
        let transform = if let Some(value) = world.get_component::<Transform2D>(entity) {
            value
        } else {
            continue;
        };
        let sprite = if let Some(value) = world.get_component::<SpriteComponent>(entity) {
            value
        } else {
            continue;
        };
        if !sprite.visible {
            continue;
        }
        items.push(SpriteBatchItem {
            texture_id: sprite.texture_id,
            z: sprite.z,
            world_pos: transform.position,
            rotation: transform.rotation,
            scale: transform.scale,
            draw_size: sprite.draw_size,
            src_rect: sprite.src_rect,
            tint: sprite.tint,
        });
    }

    items.sort_by_key(|item| item.z);
    items
}
