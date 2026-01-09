use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use crate::config::{
    GRID_HEIGHT, GRID_WIDTH, RANDOMNESS_FACTOR,
    VIEW_PAN_SPEED, VIEW_ZOOM_SPEED, DEFAULT_VIEW_ZOOM,
};
use crate::gpu::{ComputePipeline, GpuContext, GridBuffers, RenderPipeline};
use crate::simulation::Grid;

/// View state for Poincaré disk navigation
struct ViewState {
    center_x: f32,
    center_y: f32,
    zoom: f32,
    render_mode: u32,       // 0 = Euclidean, 1 = Poincaré
    phase_visualization: u32, // 0 = off, 1 = on
    time_viz_strength: f32,
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            center_x: GRID_WIDTH as f32 / 2.0,
            center_y: GRID_HEIGHT as f32 / 2.0,
            zoom: DEFAULT_VIEW_ZOOM,
            render_mode: 0,
            phase_visualization: 0,
            time_viz_strength: 0.5,
        }
    }
}

/// Application state
pub struct App {
    window: Option<Arc<Window>>,
    gpu: Option<GpuContext>,
    grid_buffers: Option<GridBuffers>,
    compute_pipeline: Option<ComputePipeline>,
    render_pipeline: Option<RenderPipeline>,
    frame_number: u32,
    fps_counter: FpsCounter,
    view: ViewState,
}

impl App {
    pub fn new() -> Self {
        Self {
            window: None,
            gpu: None,
            grid_buffers: None,
            compute_pipeline: None,
            render_pipeline: None,
            frame_number: 0,
            fps_counter: FpsCounter::new(),
            view: ViewState::default(),
        }
    }

    fn render(&mut self) {
        let gpu = self.gpu.as_ref().unwrap();
        let buffers = self.grid_buffers.as_mut().unwrap();
        let compute = self.compute_pipeline.as_ref().unwrap();
        let render = self.render_pipeline.as_ref().unwrap();

        // Update simulation parameters
        buffers.update_params(&gpu.queue, self.frame_number, RANDOMNESS_FACTOR);

        // Update render parameters with current view state
        buffers.update_render_params(
            &gpu.queue,
            self.view.render_mode,
            self.view.phase_visualization,
            (self.view.center_x, self.view.center_y),
            self.view.zoom,
            self.view.time_viz_strength,
        );

        // Get surface texture
        let output = match gpu.surface.get_current_texture() {
            Ok(texture) => texture,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                // Reconfigure surface
                gpu.surface.configure(&gpu.device, &gpu.config);
                return;
            }
            Err(e) => {
                log::error!("Surface error: {:?}", e);
                return;
            }
        };

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame-encoder"),
            });

        // 1. Run compute shader (evolution step)
        let (input_buf, output_buf) = buffers.get_io_buffers();
        let compute_bind_group =
            compute.create_bind_group(&gpu.device, input_buf, output_buf, &buffers.params_buffer);
        compute.dispatch(&mut encoder, &compute_bind_group, GRID_WIDTH, GRID_HEIGHT);

        // 2. Swap buffers (output becomes input for next frame)
        buffers.swap();

        // 3. Render the new state
        let render_bind_group = render.create_bind_group(
            &gpu.device,
            buffers.get_render_buffer(),
            &buffers.render_params_buffer,
        );
        render.draw(&mut encoder, &view, &render_bind_group);

        // Submit work
        gpu.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        // Update counters
        self.frame_number = self.frame_number.wrapping_add(1);

        // Update and display FPS
        if let Some(fps) = self.fps_counter.tick() {
            if let Some(window) = &self.window {
                let mode_str = if self.view.render_mode == 1 { "Poincare" } else { "Euclidean" };
                let phase_str = if self.view.phase_visualization == 1 { " [Phase]" } else { "" };
                window.set_title(&format!(
                    "Hyperbolic Wave Sim - {:.0} FPS - {} {}",
                    fps, mode_str, phase_str
                ));
            }
        }
    }

    fn handle_key(&mut self, key_code: KeyCode) {
        match key_code {
            // Toggle render mode (Euclidean <-> Poincaré)
            KeyCode::Space => {
                self.view.render_mode = 1 - self.view.render_mode;
                let mode = if self.view.render_mode == 1 { "Poincare disk" } else { "Euclidean" };
                log::info!("Switched to {} mode", mode);
            }

            // Toggle phase visualization
            KeyCode::KeyP => {
                self.view.phase_visualization = 1 - self.view.phase_visualization;
                log::info!("Phase visualization: {}", if self.view.phase_visualization == 1 { "ON" } else { "OFF" });
            }

            // Toggle time dilation visualization
            KeyCode::KeyT => {
                if self.view.time_viz_strength > 0.0 {
                    self.view.time_viz_strength = 0.0;
                } else {
                    self.view.time_viz_strength = 0.5;
                }
                log::info!("Time dilation visualization: {}", if self.view.time_viz_strength > 0.0 { "ON" } else { "OFF" });
            }

            // Pan view (WASD)
            KeyCode::KeyW | KeyCode::ArrowUp => {
                self.view.center_y -= VIEW_PAN_SPEED;
            }
            KeyCode::KeyS | KeyCode::ArrowDown => {
                self.view.center_y += VIEW_PAN_SPEED;
            }
            KeyCode::KeyA | KeyCode::ArrowLeft => {
                self.view.center_x -= VIEW_PAN_SPEED;
            }
            KeyCode::KeyD | KeyCode::ArrowRight => {
                self.view.center_x += VIEW_PAN_SPEED;
            }

            // Zoom (Q/E or +/-)
            KeyCode::KeyQ | KeyCode::Minus => {
                self.view.zoom *= VIEW_ZOOM_SPEED;
                log::info!("Zoom: {:.1}", self.view.zoom);
            }
            KeyCode::KeyE | KeyCode::Equal => {
                self.view.zoom /= VIEW_ZOOM_SPEED;
                log::info!("Zoom: {:.1}", self.view.zoom);
            }

            // Reset view
            KeyCode::KeyR => {
                self.view = ViewState::default();
                log::info!("View reset");
            }

            // Increase/decrease time visualization strength
            KeyCode::BracketLeft => {
                self.view.time_viz_strength = (self.view.time_viz_strength - 0.1).max(0.0);
                log::info!("Time viz strength: {:.1}", self.view.time_viz_strength);
            }
            KeyCode::BracketRight => {
                self.view.time_viz_strength = (self.view.time_viz_strength + 0.1).min(1.0);
                log::info!("Time viz strength: {:.1}", self.view.time_viz_strength);
            }

            _ => {}
        }

        // Wrap view center around grid boundaries
        self.view.center_x = self.view.center_x.rem_euclid(GRID_WIDTH as f32);
        self.view.center_y = self.view.center_y.rem_euclid(GRID_HEIGHT as f32);
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        log::info!("Initializing Hyperbolic Relativistic Wave Simulator...");
        log::info!("Grid size: {}x{}", GRID_WIDTH, GRID_HEIGHT);

        // Create window
        let window_attrs = Window::default_attributes()
            .with_title("Hyperbolic Wave Sim - Initializing...")
            .with_inner_size(winit::dpi::LogicalSize::new(1024, 1024));

        let window = Arc::new(
            event_loop
                .create_window(window_attrs)
                .expect("Failed to create window"),
        );

        // Initialize GPU
        log::info!("Creating GPU context...");
        let gpu = pollster::block_on(GpuContext::new(window.clone()));

        // Initialize grid with random state
        log::info!("Generating initial grid...");
        let grid = Grid::new_default();
        log::info!("Grid cells: {}", grid.cells.len());

        // Create buffers
        log::info!("Creating GPU buffers...");
        let grid_buffers =
            GridBuffers::new(&gpu.device, &gpu.queue, GRID_WIDTH, GRID_HEIGHT, &grid.cells);

        // Initialize render params with defaults
        grid_buffers.update_render_params_default(&gpu.queue);

        // Create pipelines
        log::info!("Creating compute pipeline...");
        let compute_pipeline = ComputePipeline::new(&gpu.device);

        log::info!("Creating render pipeline...");
        let render_pipeline =
            RenderPipeline::new(&gpu.device, gpu.format(), GRID_WIDTH, GRID_HEIGHT);

        log::info!("Initialization complete!");
        log::info!("Controls:");
        log::info!("  Space: Toggle Euclidean/Poincare mode");
        log::info!("  P: Toggle phase visualization");
        log::info!("  T: Toggle time dilation visualization");
        log::info!("  WASD/Arrows: Pan view");
        log::info!("  Q/E: Zoom out/in");
        log::info!("  R: Reset view");
        log::info!("  [/]: Adjust time viz strength");
        log::info!("  Escape: Quit");

        self.window = Some(window);
        self.gpu = Some(gpu);
        self.grid_buffers = Some(grid_buffers);
        self.compute_pipeline = Some(compute_pipeline);
        self.render_pipeline = Some(render_pipeline);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                log::info!("Close requested, exiting...");
                event_loop.exit();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state.is_pressed() {
                    if let PhysicalKey::Code(key_code) = event.physical_key {
                        if key_code == KeyCode::Escape {
                            log::info!("Escape pressed, exiting...");
                            event_loop.exit();
                        } else {
                            self.handle_key(key_code);
                        }
                    }
                }
            }
            WindowEvent::Resized(new_size) => {
                if let Some(gpu) = &mut self.gpu {
                    log::info!("Window resized to {}x{}", new_size.width, new_size.height);
                    gpu.resize(new_size);
                }
            }
            WindowEvent::RedrawRequested => {
                self.render();
                // Request another frame immediately
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }
}

/// Simple FPS counter
struct FpsCounter {
    last_update: Instant,
    frame_count: u32,
}

impl FpsCounter {
    fn new() -> Self {
        Self {
            last_update: Instant::now(),
            frame_count: 0,
        }
    }

    /// Tick the counter, returns Some(fps) every second
    fn tick(&mut self) -> Option<f64> {
        self.frame_count += 1;
        let elapsed = self.last_update.elapsed();

        if elapsed.as_secs_f64() >= 1.0 {
            let fps = self.frame_count as f64 / elapsed.as_secs_f64();
            self.frame_count = 0;
            self.last_update = Instant::now();
            Some(fps)
        } else {
            None
        }
    }
}
