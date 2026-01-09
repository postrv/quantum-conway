use std::sync::Arc;
use wgpu::{Device, Instance, Queue, Surface, SurfaceConfiguration};
use winit::window::Window;

/// GPU context holding wgpu instance, device, queue, and surface
pub struct GpuContext {
    pub surface: Surface<'static>,
    pub device: Device,
    pub queue: Queue,
    pub config: SurfaceConfiguration,
}

impl GpuContext {
    /// Create a new GPU context for the given window
    pub async fn new(window: Arc<Window>) -> Self {
        let instance = Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance
            .create_surface(window.clone())
            .expect("Failed to create surface");

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to find suitable GPU adapter");

        log::info!("Using GPU: {}", adapter.get_info().name);

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("quantum-conway-device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await
            .expect("Failed to create device");

        let size = window.inner_size();
        let config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .expect("Surface not supported by adapter");

        surface.configure(&device, &config);

        Self {
            surface,
            device,
            queue,
            config,
        }
    }

    /// Handle window resize
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    /// Get current surface texture format
    pub fn format(&self) -> wgpu::TextureFormat {
        self.config.format
    }
}
