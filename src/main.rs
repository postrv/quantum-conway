//! Quantum Conway's Game of Life
//!
//! A GPU-accelerated implementation of Conway's Game of Life with complex/quantum states.
//! Each cell has a probability distribution over four states: +1, -1, +i, -i.
//! Cells can be entangled, causing their states to synchronize.

mod app;
mod config;
mod gpu;
mod simulation;

use winit::event_loop::{ControlFlow, EventLoop};

fn main() {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("Starting Quantum Conway's Game of Life");
    log::info!("Press ESC to exit");

    // Create event loop
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);

    // Run application
    let mut app = app::App::new();
    event_loop
        .run_app(&mut app)
        .expect("Failed to run application");
}
