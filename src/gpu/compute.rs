use wgpu::{BindGroup, BindGroupLayout, Buffer, ComputePipeline as WgpuComputePipeline, Device};
use crate::config::WORKGROUP_SIZE;

/// Compute pipeline for grid evolution
pub struct ComputePipeline {
    pipeline: WgpuComputePipeline,
    bind_group_layout: BindGroupLayout,
}

impl ComputePipeline {
    /// Create a new compute pipeline
    pub fn new(device: &Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("compute-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/compute.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("compute-bind-group-layout"),
            entries: &[
                // Input cells (read-only storage)
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Output cells (read-write storage)
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Simulation parameters (uniform)
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
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
            label: Some("compute-pipeline-layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("evolution-compute-pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        Self {
            pipeline,
            bind_group_layout,
        }
    }

    /// Create a bind group for the given buffers
    pub fn create_bind_group(
        &self,
        device: &Device,
        input_buffer: &Buffer,
        output_buffer: &Buffer,
        params_buffer: &Buffer,
    ) -> BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("compute-bind-group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        })
    }

    /// Dispatch the compute shader
    pub fn dispatch(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        bind_group: &BindGroup,
        grid_width: u32,
        grid_height: u32,
    ) {
        let workgroups_x = (grid_width + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
        let workgroups_y = (grid_height + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;

        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("evolution-compute-pass"),
            timestamp_writes: None,
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
    }
}
