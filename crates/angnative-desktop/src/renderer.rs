/**
 * renderer.rs — wgpu GPU renderer for AngNative desktop.
 *
 * Renders the native view tree computed by the Rust host using GPU-accelerated
 * drawing. Each na-* element becomes a draw call with its layout rectangle.
 *
 * No WebView, no Electron — pure GPU-accelerated native desktop rendering.
 */

use wgpu::util::DeviceExt;
use winit::window::Window;

use angnative_host_layout::LayoutResult;

/**
 * RectUniform — per-instance uniform data for each view node.
 */
#[repr(C)]
#[derive(Copy, Clone)]
struct RectUniform {
    rect: [f32; 4], // x, y, w, h in NDC
    color: [f32; 4], // RGBA
}

unsafe impl bytemuck::Pod for RectUniform {}
unsafe impl bytemuck::Zeroable for RectUniform {}

const SHADER_SRC: &str = r#"
struct RectUniform {
    rect: vec4<f32>,
    color: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> rectUniform: RectUniform;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @instance_index instance_index: u32,
) -> VertexOutput {
    let positions = array<vec2<f32>, 4>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0,  1.0),
    );

    let rect = rectUniform.rect;
    let pos = positions[vertex_index] * rect.zw + rect.xy;

    var output: VertexOutput;
    output.position = vec4<f32>(pos, 0.0, 1.0);
    output.color = rectUniform.color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}
"#;

/**
 * DesktopRenderer — wgpu-based renderer for the desktop shell.
 *
 * Draws colored rectangles for each native view node, sized and positioned
 * according to the Taffy layout computed by the Rust host.
 */
pub struct DesktopRenderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
}

impl DesktopRenderer {
    /// Create a new renderer for the given window.
    pub async fn new(window: &Window) -> Result<Self, String> {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(window).map_err(|e| e.to_string())?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .map_err(|e| format!("Failed to request adapter: {e}"))?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    label: Some("angnative-device"),
                },
                None,
            )
            .await
            .map_err(|e| format!("Failed to create device: {e}"))?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("angnative-shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER_SRC.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("angnative-pipeline-layout"),
            bind_group_layouts: &[
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("rect-uniform-layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                }),
            ],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("angnative-render-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<RectUniform>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &[
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x4,
                        },
                        wgpu::VertexAttribute {
                            offset: 16,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x4,
                        },
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            cache: None,
        });

        let vertices: [f32; 8] = [
            -1.0, -1.0,
             1.0, -1.0,
            -1.0,  1.0,
             1.0,  1.0,
        ];

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vertex-buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("index-buffer"),
            contents: bytemuck::cast_slice(&[0u16, 1, 2, 0, 2, 3]),
            usage: wgpu::BufferUsages::INDEX,
        });

        let initial_uniform = RectUniform {
            rect: [0.0, 0.0, 0.0, 0.0],
            color: [1.0, 1.0, 1.0, 1.0],
        };

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("uniform-buffer"),
            contents: bytemuck::cast_slice(&[initial_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        Ok(Self {
            surface,
            device,
            queue,
            config,
            size,
            pipeline,
            vertex_buffer,
            index_buffer,
            uniform_buffer,
        })
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn render(&mut self, layout: Option<&LayoutResult>) -> Result<(), String> {
        let frame = self
            .surface
            .get_current_texture()
            .map_err(|e| format!("Failed to acquire surface texture: {e}"))?;

        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("angnative-command-encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("angnative-render-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.1,
                            b: 0.1,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(
                self.index_buffer.slice(..),
                wgpu::IndexFormat::Uint16,
            );

            if let Some(layout) = layout {
                for (i, rect) in layout.nodes.iter().enumerate() {
                    let x = rect.x / self.size.width as f32 * 2.0 - 1.0;
                    let y = rect.y / self.size.height as f32 * 2.0 - 1.0;
                    let w = rect.width / self.size.width as f32 * 2.0;
                    let h = rect.height / self.size.height as f32 * 2.0;

                    let color = match rect.node_id {
                        1 => [0.2, 0.2, 0.25, 1.0],
                        2 => [0.1, 0.4, 0.7, 1.0],
                        3 => [0.1, 0.4, 0.7, 1.0],
                        4 => [0.15, 0.15, 0.2, 1.0],
                        5 => [0.2, 0.6, 0.9, 1.0],
                        6 => [0.2, 0.6, 0.9, 1.0],
                        7 => [0.2, 0.6, 0.9, 1.0],
                        8 => [0.0, 0.8, 0.4, 1.0],
                        _ => [0.3, 0.3, 0.35, 1.0],
                    };

                    let uniform = RectUniform {
                        rect: [x, y, w, h],
                        color,
                    };

                    self.queue.write_buffer(
                        &self.uniform_buffer,
                        0,
                        bytemuck::cast_slice(&[uniform]),
                    );

                    let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("rect-bind-group"),
                        layout: &self.pipeline.get_bind_group_layout(0),
                        entries: &[wgpu::BindGroupEntry {
                            binding: 0,
                            resource: self.uniform_buffer.as_entire_binding(),
                        }],
                    });

                    render_pass.set_bind_group(0, &bind_group, &[]);
                    render_pass.draw_indexed(0..6, 0, i as i32..i as i32 + 1);
                }
            } else {
                render_pass.draw_indexed(0..6, 0, 0..1);
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();

        Ok(())
    }
}
