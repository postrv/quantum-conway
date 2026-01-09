/// Grid dimensions (512x512 = 262K cells, 4x larger on screen)
pub const GRID_WIDTH: u32 = 512;
pub const GRID_HEIGHT: u32 = 512;

/// Compute shader workgroup size
pub const WORKGROUP_SIZE: u32 = 16;

/// Simulation parameters
pub const RANDOMNESS_FACTOR: f32 = 0.01;
pub const ENTANGLEMENT_PROBABILITY: f64 = 0.88;

/// No entanglement marker (all bits set)
pub const NO_ENTANGLEMENT: u32 = 0xFFFFFFFF;

// ============================================
// Wave Equation Parameters
// ============================================

/// Base time step for simulation (CFL condition: dt < dx/(√2 * c) ≈ 0.47 for c=1.5)
pub const BASE_DT: f32 = 0.1;

/// Wave propagation speed in cells per time unit
pub const WAVE_SPEED: f32 = 1.2;

/// Damping coefficient for wave equation (prevents runaway oscillation)
pub const DAMPING: f32 = 0.05;

/// Speed of light for relativistic causality (cells per time unit)
pub const LIGHT_SPEED: f32 = 1.5;

/// Mutation probability per frame (creates new wave sources)
pub const MUTATION_PROBABILITY: f32 = 0.002;

// ============================================
// Poincaré Disk Rendering
// ============================================

/// Default zoom level for Poincaré disk view (grid units visible)
pub const DEFAULT_VIEW_ZOOM: f32 = 256.0;

/// Render mode: 0 = Euclidean (flat grid), 1 = Poincaré disk (hyperbolic)
pub const DEFAULT_RENDER_MODE: u32 = 0;

/// View pan speed for keyboard navigation
pub const VIEW_PAN_SPEED: f32 = 10.0;

/// View zoom speed for keyboard navigation
pub const VIEW_ZOOM_SPEED: f32 = 1.1;

// ============================================
// FUTURE ENHANCEMENTS: Cellular Automata Dynamics
// ============================================
//
// The following options describe ways to restore more traditional CA-style
// discrete dynamics while preserving the beautiful wave-based visuals:
//
// ## Option 1: Collapse Dynamics (Quantum → Classical)
//
// Add periodic amplitude quantization where cells "snap" toward their dominant
// state, creating clearer domain boundaries like CA cells:
//
//   - Cells with a clear winner (one amplitude >> others) become more stable
//   - Creates discrete "alive in state X" regions with wave-like boundaries
//   - Implementation: Add `collapse_strength: f32` parameter (0.0 = pure waves,
//     1.0 = hard snap to dominant state each frame)
//   - In compute shader, after wave evolution:
//       let max_idx = argmax(amplitudes);
//       amplitudes = mix(amplitudes, one_hot(max_idx), collapse_strength);
//
// ## Option 2: Voting/Majority Rules
//
// Layer classic CA logic on top - neighbors vote on which state survives:
//
//   - If >N neighbors share your dominant state, you strengthen
//   - If <N neighbors share it, you weaken (underpopulation/overpopulation)
//   - Creates glider-like structures that ride the wave dynamics
//   - Implementation: Add `voting_strength: f32` and `survival_threshold: u32`
//   - Count neighbors with same dominant state, apply B3/S23-style rules
//
// ## Option 3: Bistability + Hysteresis
//
// Make cells prefer being "mostly one state" rather than mixed:
//
//   - Add energy penalty for being in superposition (high entropy)
//   - Once a cell commits to a state, it resists changing (hysteresis)
//   - Creates sharper, more persistent domains
//   - Implementation: Add `bistability_strength: f32`, `hysteresis: f32`
//   - Modify normalization to penalize uniform distributions
//
// ## Option 4: Parameter Tuning for CA-like Behavior
//
// Adjust existing parameters without new mechanics:
//
//   - Higher DAMPING (0.15-0.3) → less fluid, more stable regions
//   - Lower MUTATION_PROBABILITY → patterns persist longer
//   - Lower WAVE_SPEED → slower, more deliberate evolution
//   - Add structured initial seeds (gliders, oscillators) in grid.rs
//
// ## Recommended Approach
//
// Combine Option 1 + Option 2: Add collapse_strength and neighbor voting.
// This preserves quantum-ish wave propagation for beautiful transitions,
// while creating discrete CA-like domains and emergent structures.
//
// New parameters to add:
//   pub const COLLAPSE_STRENGTH: f32 = 0.1;      // 0.0-1.0, snap toward dominant
//   pub const VOTING_ENABLED: bool = true;
//   pub const SURVIVAL_MIN: u32 = 2;             // Min neighbors to survive
//   pub const SURVIVAL_MAX: u32 = 3;             // Max neighbors to survive
//   pub const BIRTH_COUNT: u32 = 3;              // Neighbors needed for birth
