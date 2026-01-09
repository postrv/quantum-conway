use wgpu::{
    BindGroup, BindGroupLayout, Buffer, Device, RenderPipeline as WgpuRenderPipeline,
    TextureFormat, TextureView,
};

/// Render pipeline for visualizing the grid
/// Note: RenderParams is now defined in buffers.rs and managed by GridBuffers
pub struct RenderPipeline {
    pipeline: WgpuRenderPipeline,
    bind_group_layout: BindGroupLayout,
}

impl RenderPipeline {
    /// Create a new render pipeline
    pub fn new(device: &Device, format: TextureFormat, _grid_width: u32, _grid_height: u32) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("render-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/render.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("render-bind-group-layout"),
            entries: &[
                // Cells storage buffer (read-only)
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Render parameters (uniform)
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("render-pipeline-layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("render-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            pipeline,
            bind_group_layout,
        }
    }

    /// Create a bind group for the given cell buffer and params buffer
    pub fn create_bind_group(
        &self,
        device: &Device,
        cells_buffer: &Buffer,
        render_params_buffer: &Buffer,
    ) -> BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("render-bind-group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: cells_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: render_params_buffer.as_entire_binding(),
                },
            ],
        })
    }

    /// Draw the grid to the given texture view
    pub fn draw(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &TextureView,
        bind_group: &BindGroup,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("render-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.05,  // Near black
                        g: 0.05,
                        b: 0.08,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        pass.draw(0..3, 0..1); // Draw fullscreen triangle
    }
}
