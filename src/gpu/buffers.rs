use wgpu::{Buffer, BufferUsages, Device, Queue};
use crate::simulation::GpuCell;
use crate::config::{BASE_DT, WAVE_SPEED, DAMPING, LIGHT_SPEED, DEFAULT_VIEW_ZOOM, DEFAULT_RENDER_MODE};

/// Manages ping-pong storage buffers for the grid
pub struct GridBuffers {
    /// Buffer A - ping
    pub buffer_a: Buffer,
    /// Buffer B - pong
    pub buffer_b: Buffer,
    /// Uniform buffer for simulation parameters
    pub params_buffer: Buffer,
    /// Uniform buffer for render parameters
    pub render_params_buffer: Buffer,
    /// Which buffer is current input (true = A is input, false = B is input)
    read_from_a: bool,
    /// Grid dimensions
    pub width: u32,
    pub height: u32,
}

/// Simulation parameters passed to compute shader (64 bytes, aligned to 16)
/// Note: WGSL vec3<f32> has 16-byte alignment, so _padding must be 4 floats
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SimParams {
    // Grid info (16 bytes)
    pub grid_width: u32,
    pub grid_height: u32,
    pub frame_number: u32,
    pub randomness_factor: f32,

    // Wave equation parameters (16 bytes)
    pub base_dt: f32,
    pub wave_speed: f32,
    pub damping: f32,
    pub light_speed: f32,

    // Additional parameters (32 bytes) - mutation_probability + vec3 padding (16-byte aligned)
    pub mutation_probability: f32,
    pub _padding1: [f32; 3],  // Padding to align vec3 to 16 bytes
    pub _padding2: [f32; 4],  // Extra padding to match WGSL vec3 total size
}

/// Render parameters passed to render shader (48 bytes, aligned to 16)
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RenderParams {
    // Grid info (8 bytes + 8 padding = 16 bytes)
    pub grid_width: u32,
    pub grid_height: u32,
    pub render_mode: u32,      // 0 = Euclidean, 1 = Poincaré disk
    pub phase_visualization: u32, // 0 = off, 1 = hue shift by phase

    // View parameters (16 bytes)
    pub view_center_x: f32,
    pub view_center_y: f32,
    pub view_zoom: f32,
    pub time_viz_strength: f32,   // How much to visualize time dilation

    // Reserved (16 bytes)
    pub _padding: [f32; 4],
}

impl GridBuffers {
    /// Create new grid buffers and upload initial data
    pub fn new(device: &Device, queue: &Queue, width: u32, height: u32, initial_data: &[GpuCell]) -> Self {
        let cell_count = width * height;
        assert_eq!(
            initial_data.len(),
            cell_count as usize,
            "Initial data size mismatch"
        );

        let buffer_size = (cell_count as usize * std::mem::size_of::<GpuCell>()) as u64;

        let buffer_a = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("grid-buffer-a"),
            size: buffer_size,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let buffer_b = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("grid-buffer-b"),
            size: buffer_size,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sim-params-buffer"),
            size: std::mem::size_of::<SimParams>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let render_params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("render-params-buffer"),
            size: std::mem::size_of::<RenderParams>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Upload initial data to buffer A
        queue.write_buffer(&buffer_a, 0, bytemuck::cast_slice(initial_data));

        Self {
            buffer_a,
            buffer_b,
            params_buffer,
            render_params_buffer,
            read_from_a: true,
            width,
            height,
        }
    }

    /// Get (input_buffer, output_buffer) for current frame
    pub fn get_io_buffers(&self) -> (&Buffer, &Buffer) {
        if self.read_from_a {
            (&self.buffer_a, &self.buffer_b)
        } else {
            (&self.buffer_b, &self.buffer_a)
        }
    }

    /// Get current read buffer (for rendering after compute)
    pub fn get_render_buffer(&self) -> &Buffer {
        // After compute, output becomes the render source
        // Since we call swap() after compute, the "read" buffer is actually the output
        if self.read_from_a {
            &self.buffer_b
        } else {
            &self.buffer_a
        }
    }

    /// Swap buffers after compute pass
    pub fn swap(&mut self) {
        self.read_from_a = !self.read_from_a;
    }

    /// Update simulation parameters
    pub fn update_params(&self, queue: &Queue, frame_number: u32, randomness_factor: f32) {
        let params = SimParams {
            grid_width: self.width,
            grid_height: self.height,
            frame_number,
            randomness_factor,
            base_dt: BASE_DT,
            wave_speed: WAVE_SPEED,
            damping: DAMPING,
            light_speed: LIGHT_SPEED,
            mutation_probability: crate::config::MUTATION_PROBABILITY,
            _padding1: [0.0, 0.0, 0.0],
            _padding2: [0.0, 0.0, 0.0, 0.0],
        };
        queue.write_buffer(&self.params_buffer, 0, bytemuck::bytes_of(&params));
    }

    /// Update render parameters
    pub fn update_render_params(
        &self,
        queue: &Queue,
        render_mode: u32,
        phase_visualization: u32,
        view_center: (f32, f32),
        view_zoom: f32,
        time_viz_strength: f32,
    ) {
        let params = RenderParams {
            grid_width: self.width,
            grid_height: self.height,
            render_mode,
            phase_visualization,
            view_center_x: view_center.0,
            view_center_y: view_center.1,
            view_zoom,
            time_viz_strength,
            _padding: [0.0, 0.0, 0.0, 0.0],
        };
        queue.write_buffer(&self.render_params_buffer, 0, bytemuck::bytes_of(&params));
    }

    /// Update render params with defaults
    pub fn update_render_params_default(&self, queue: &Queue) {
        self.update_render_params(
            queue,
            DEFAULT_RENDER_MODE,
            0, // phase visualization off by default
            (self.width as f32 / 2.0, self.height as f32 / 2.0), // center of grid
            DEFAULT_VIEW_ZOOM,
            0.5, // moderate time visualization
        );
    }
}
