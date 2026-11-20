/**
 * renderer.rs — wgpu GPU renderer for AngNative desktop.
 *
 * Renders the native view tree computed by the Rust host using GPU-accelerated
 * drawing. Each na-* element becomes an instance with its own layout rectangle
 * and color, so distinct nodes render at distinct positions with distinct
 * colors (all in a single draw call).
 *
 * No WebView, no Electron — pure GPU-accelerated native desktop rendering.
 */

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use wgpu::util::DeviceExt;
use winit::window::Window;

use angnative_host_layout::LayoutResult;

/**
 * RectUniform — per-instance data for one view node, consumed as a vertex
 * buffer with `Instance` step mode. `rect` is in NDC (x, y, w, h), and `color`
 * is RGBA.
 */
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
struct RectUniform {
    rect: [f32; 4],
    color: [f32; 4],
}

unsafe impl bytemuck::Pod for RectUniform {}
unsafe impl bytemuck::Zeroable for RectUniform {}

const SHADER_SRC: &str = r#"
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @location(0) rect: vec4<f32>,
    @location(1) color: vec4<f32>,
) -> VertexOutput {
    // Derive the corner from the vertex index arithmetically. Indexing a
    // const array with a dynamically-provided vertex_index is rejected by
    // naga/wgpu ("expression may only be indexed by a constant").
    let x = f32(vertex_index & 1u) * 2.0 - 1.0;
    let y = f32((vertex_index >> 1u) & 1u) * 2.0 - 1.0;
    let pos = vec2<f32>(x, y) * rect.zw + rect.xy;

    var output: VertexOutput;
    output.position = vec4<f32>(pos, 0.0, 1.0);
    output.color = color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}
"#;

/// Number of rectangles the instance buffer holds initially. The buffer is
/// grown automatically when a frame contains more nodes than this.
const INITIAL_INSTANCE_CAPACITY: usize = 256;

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
    index_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    instance_capacity: usize,
    /// Demo interaction state: currently pressed button and counter value.
    pressed: Option<u64>,
    counter: i32,
}

impl DesktopRenderer {
    /// Create a new renderer for the given window.
    pub async fn new(window: &Window) -> Result<Self, String> {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        // Create the surface from the window's raw handles so it is 'static
        // rather than borrowing the window for its lifetime. SAFETY: the raw
        // handles stay valid while the winit window is alive; the owning
        // AngNativeDesktop drops its renderer before the window.
        let raw_window_handle = window
            .window_handle()
            .map_err(|e| format!("Failed to get raw window handle: {e}"))?
            .as_raw();
        let raw_display_handle = window
            .display_handle()
            .map_err(|e| format!("Failed to get raw display handle: {e}"))?
            .as_raw();
        let surface = unsafe {
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                raw_display_handle,
                raw_window_handle,
            })
        }
        .map_err(|e| e.to_string())?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| "Failed to request a wgpu adapter for this surface".to_string())?;

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
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("angnative-shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER_SRC.into()),
        });

        // Rectangles are instanced via a vertex buffer, so no bind groups are
        // needed — each instance carries its own rectangle and color.
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("angnative-pipeline-layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("angnative-render-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                compilation_options: Default::default(),
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
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("index-buffer"),
            contents: bytemuck::cast_slice(&[0u16, 1, 2, 0, 2, 3]),
            usage: wgpu::BufferUsages::INDEX,
        });

        let instance_buffer = Self::create_instance_buffer(&device, INITIAL_INSTANCE_CAPACITY);

        Ok(Self {
            surface,
            device,
            queue,
            config,
            size,
            pipeline,
            index_buffer,
            instance_buffer,
            instance_capacity: INITIAL_INSTANCE_CAPACITY,
            pressed: None,
            counter: 0,
        })
    }

    fn create_instance_buffer(device: &wgpu::Device, capacity: usize) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("rect-instance-buffer"),
            size: (capacity * std::mem::size_of::<RectUniform>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
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

    /// Update the lightweight interaction state used for visual feedback.
    pub fn set_state(&mut self, counter: i32, pressed: Option<u64>) {
        self.counter = counter;
        self.pressed = pressed;
    }

    pub fn render(&mut self, layout: Option<&LayoutResult>) -> Result<(), String> {
        // Build one RectUniform per node — each instance gets its own rect and
        // color, so nodes no longer share a single uniform that is overwritten
        // inside the render pass.
        let instances: Vec<RectUniform> = match layout {
            Some(layout) => layout
                .nodes
                .iter()
                .map(|rect| RectUniform {
                    rect: rect_to_ndc(
                        rect.x,
                        rect.y,
                        rect.width,
                        rect.height,
                        self.size.width,
                        self.size.height,
                    ),
                    color: if self.pressed == Some(rect.node_id) {
                        [0.95, 0.75, 0.2, 1.0] // pressed button highlight
                    } else {
                        color_for_node(rect.node_id)
                    },
                })
                .collect(),
            None => vec![RectUniform {
                rect: [-0.5, -0.5, 1.0, 1.0],
                color: [0.3, 0.3, 0.35, 1.0],
            }],
        };

        // Counter readout: one green dot per click (up to 10), drawn in a row
        // at the bottom of the window. Text rendering is not wired up yet.
        let mut instances = instances;
        for i in 0..self.counter.clamp(0, 10) {
            instances.push(RectUniform {
                rect: [-0.92 + i as f32 * 0.11, -0.88, 0.07, 0.07],
                color: [0.1, 0.85, 0.45, 1.0],
            });
        }

        // Grow the instance buffer if a frame has more nodes than it can hold.
        if instances.len() > self.instance_capacity {
            let mut capacity = self.instance_capacity.max(1);
            while capacity < instances.len() {
                capacity *= 2;
            }
            self.instance_buffer = Self::create_instance_buffer(&self.device, capacity);
            self.instance_capacity = capacity;
        }
        self.queue
            .write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&instances));

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
            render_pass.set_vertex_buffer(0, self.instance_buffer.slice(..));
            render_pass.set_index_buffer(
                self.index_buffer.slice(..),
                wgpu::IndexFormat::Uint16,
            );
            render_pass.draw_indexed(0..6, 0, 0..instances.len() as u32);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();

        Ok(())
    }
}

/// Convert a layout rectangle in device-independent pixels to NDC.
/// Zero-sized surfaces are clamped to one pixel to avoid divide-by-zero.
fn rect_to_ndc(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    surface_width: u32,
    surface_height: u32,
) -> [f32; 4] {
    let w = surface_width.max(1) as f32;
    let h = surface_height.max(1) as f32;
    [
        x / w * 2.0 - 1.0,
        y / h * 2.0 - 1.0,
        width / w * 2.0,
        height / h * 2.0,
    ]
}

/// Deterministic, distinct color per demo node id so each rectangle is
/// visually distinguishable.
fn color_for_node(node_id: u64) -> [f32; 4] {
    match node_id {
        1 => [0.2, 0.2, 0.25, 1.0],
        2 | 3 => [0.1, 0.4, 0.7, 1.0],
        4 => [0.15, 0.15, 0.2, 1.0],
        5 | 6 | 7 => [0.2, 0.6, 0.9, 1.0],
        8 => [0.0, 0.8, 0.4, 1.0],
        _ => [0.3, 0.3, 0.35, 1.0],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_to_ndc_maps_layout_space_to_clip_space() {
        let ndc = rect_to_ndc(100.0, 50.0, 200.0, 100.0, 800, 600);
        assert_eq!(ndc[0], -0.75);
        assert!((ndc[1] - (-0.833_333_3)).abs() < 1e-5);
        assert_eq!(ndc[2], 0.5);
        assert!((ndc[3] - 0.333_333_3).abs() < 1e-5);
    }

    #[test]
    fn rect_to_ndc_handles_zero_sized_surface() {
        let ndc = rect_to_ndc(0.0, 0.0, 10.0, 10.0, 0, 0);
        assert!(ndc.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn node_colors_are_distinct_across_groups() {
        assert_ne!(color_for_node(1), color_for_node(2));
        assert_ne!(color_for_node(4), color_for_node(5));
        assert_ne!(color_for_node(8), color_for_node(99));
        // Unknown ids share a fallback color, which is deterministic.
        assert_eq!(color_for_node(99), color_for_node(100));
    }

    #[test]
    fn rect_uniform_is_32_bytes() {
        assert_eq!(std::mem::size_of::<RectUniform>(), 32);
    }
}
