# Quantum Conway's Game of Life

Emmett Shear asked:

> Has anyone made Conway's Game Of Life, but the values of the cells are in [-1, 1, -i, i] instead of [0, 1] and the evolution rules per time step are complex instead? Quantum Life.

This is my attempt at answering that question.

## What It Does

A GPU-accelerated simulation combining quantum mechanics, wave physics, and relativistic causality, rendered as a hyperbolic Poincaré disk that looks like a mesmerizing mirror ball.

### Features

- **4-State Quantum System**: Each cell has amplitudes and phases for four states (+1, -1, +i, -i)
- **Wave Equation Evolution**: Continuous wave propagation with configurable speed and damping
- **Relativistic Causality**: Cells only interact within their past light cone
- **Time Dilation**: High-entropy (uncertain) cells evolve slower than low-entropy (certain) ones
- **Quantum Entanglement**: Random cell pairs have non-local phase correlations
- **Hyperbolic Rendering**: Poincaré disk projection creates stunning curved-space visuals

## Building & Running

```bash
cargo run --release
```

Requires a GPU with Vulkan, Metal, or DX12 support.

## Controls

| Key | Action |
|-----|--------|
| `Space` | Toggle Euclidean / Poincaré disk mode |
| `P` | Toggle phase visualization (hue shift) |
| `T` | Toggle time dilation visualization |
| `WASD` / Arrows | Pan view |
| `Q` / `E` | Zoom out / in |
| `R` | Reset view |
| `[` / `]` | Adjust time visualization strength |
| `Escape` | Quit |

## Architecture

```
src/
├── main.rs              # Entry point, event loop
├── app.rs               # Application state, input handling
├── config.rs            # Simulation parameters
├── gpu/
│   ├── context.rs       # wgpu device/queue setup
│   ├── buffers.rs       # Ping-pong storage buffers
│   ├── compute.rs       # Compute pipeline dispatch
│   └── render.rs        # Fullscreen render pipeline
├── shaders/
│   ├── compute.wgsl     # Wave equation, entropy, causality
│   └── render.wgsl      # Poincaré projection, coloring
└── simulation/
    ├── cell.rs          # GpuCell struct (64 bytes)
    └── grid.rs          # Initial state generation
```

## The Physics

### Wave Equation
Each amplitude channel evolves according to a damped wave equation:

```
∂²A/∂t² = c²∇²A - γ(∂A/∂t)
```

### Time Dilation
Shannon entropy of the amplitude distribution determines local time flow:

```
H = -Σ p_i log₂(p_i)
time_dilation = 1 - 0.9 × (H / 2)
```

High entropy (uniform distribution) → slow time. Low entropy (one dominant state) → fast time.

### Causality
Neighbors only influence a cell if they're within its past light cone:

```
spatial_distance ≤ c × time_difference
```

## Future Enhancements

See `src/config.rs` for planned CA dynamics options:
- Collapse dynamics (quantum → classical snap)
- Voting/majority rules (B3/S23-style)
- Bistability with hysteresis

## License

MIT
