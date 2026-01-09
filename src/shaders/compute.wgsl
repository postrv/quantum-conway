// Hyperbolic Relativistic Wave Simulator - Compute Shader
// Evolves the grid using wave equations, phase dynamics, and relativistic causality

// ============================================
// DATA STRUCTURES
// ============================================

struct Cell {
    amplitudes: vec4<f32>,      // Wave amplitude magnitudes [+1, -1, +i, -i]
    phases: vec4<f32>,          // Phase angles [0, 2*PI) on unit circle
    velocities: vec4<f32>,      // d(amplitude)/dt for wave equation
    local_time: f32,            // Proper time (relativistic)
    time_dilation: f32,         // Evolution rate based on entropy
    entangled_partner: u32,     // Packed (x << 16) | y, or 0xFFFFFFFF for none
    rng_state: u32,
}

struct SimParams {
    grid_width: u32,
    grid_height: u32,
    frame_number: u32,
    randomness_factor: f32,

    base_dt: f32,
    wave_speed: f32,
    damping: f32,
    light_speed: f32,

    mutation_probability: f32,
    _padding: vec3<f32>,
}

@group(0) @binding(0) var<storage, read> cells_in: array<Cell>;
@group(0) @binding(1) var<storage, read_write> cells_out: array<Cell>;
@group(0) @binding(2) var<uniform> params: SimParams;

// ============================================
// CONSTANTS
// ============================================

const NO_ENTANGLEMENT: u32 = 0xFFFFFFFFu;
const TWO_PI: f32 = 6.28318530718;
const PI: f32 = 3.14159265359;

// ============================================
// UTILITY FUNCTIONS
// ============================================

// PCG hash function for high-quality random numbers
fn pcg_hash(input: u32) -> u32 {
    var state = input * 747796405u + 2891336453u;
    var word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

// Generate random f32 in [0, 1)
fn rand_f32(state: ptr<function, u32>) -> f32 {
    *state = pcg_hash(*state);
    return f32(*state) / 4294967296.0;
}

// Wrap angle to [0, 2*PI)
fn wrap_angle(angle: f32) -> f32 {
    var a = angle;
    while (a < 0.0) { a += TWO_PI; }
    while (a >= TWO_PI) { a -= TWO_PI; }
    return a;
}

// Check if cell has entanglement
fn has_entanglement(encoded: u32) -> bool {
    return encoded != NO_ENTANGLEMENT;
}

// Get linear index from packed partner coordinates
fn get_partner_index(encoded: u32, grid_width: u32) -> u32 {
    let x = encoded >> 16u;
    let y = encoded & 0xFFFFu;
    return y * grid_width + x;
}

// Wrap coordinate for toroidal topology
fn wrap_coord(coord: i32, size: u32) -> u32 {
    return u32((coord + i32(size)) % i32(size));
}

// ============================================
// ENTROPY AND TIME DILATION
// ============================================

// Compute Shannon entropy of amplitude distribution (as probabilities)
fn compute_entropy(amplitudes: vec4<f32>) -> f32 {
    // Convert amplitudes to probabilities (p = a^2)
    let probs = amplitudes * amplitudes;
    let total = probs.x + probs.y + probs.z + probs.w;

    if (total < 0.0001) {
        return 2.0; // Maximum entropy for degenerate case
    }

    let norm_probs = probs / total;
    var entropy = 0.0;
    let epsilon = 0.0001;

    // H = -sum(p * log2(p))
    if (norm_probs.x > epsilon) { entropy -= norm_probs.x * log2(norm_probs.x); }
    if (norm_probs.y > epsilon) { entropy -= norm_probs.y * log2(norm_probs.y); }
    if (norm_probs.z > epsilon) { entropy -= norm_probs.z * log2(norm_probs.z); }
    if (norm_probs.w > epsilon) { entropy -= norm_probs.w * log2(norm_probs.w); }

    return entropy; // Range: 0 (certain) to 2 (uniform)
}

// Map entropy to time dilation factor
fn entropy_to_time_dilation(entropy: f32) -> f32 {
    let max_entropy = 2.0; // log2(4) = 2
    let normalized = clamp(entropy / max_entropy, 0.0, 1.0);

    // High entropy -> slow time (0.1), Low entropy -> fast time (1.0)
    return 1.0 - normalized * 0.9;
}

// ============================================
// RELATIVISTIC CAUSALITY
// ============================================

// Check if a neighbor is in our past light cone
fn is_in_past_light_cone(
    my_time: f32,
    neighbor_time: f32,
    spatial_distance: f32,
    light_speed: f32
) -> bool {
    let time_diff = my_time - neighbor_time;

    // Neighbor must not be in our future
    if (time_diff < -0.01) {
        return false;
    }

    // Light must have had time to travel the distance
    let light_travel_distance = max(time_diff, 0.0) * light_speed;
    return spatial_distance <= light_travel_distance + 0.5; // Small buffer for numerical stability
}

// ============================================
// WAVE EQUATION DYNAMICS
// ============================================

// Compute discrete Laplacian for one amplitude channel
fn compute_laplacian_channel(
    center_amp: f32,
    neighbor_amps: array<f32, 8>,
    neighbor_weights: array<f32, 8>,
    neighbor_count: u32
) -> f32 {
    var laplacian = 0.0;
    var total_weight = 0.0;

    for (var i = 0u; i < neighbor_count; i++) {
        laplacian += (neighbor_amps[i] - center_amp) * neighbor_weights[i];
        total_weight += neighbor_weights[i];
    }

    if (total_weight > 0.0) {
        laplacian /= total_weight;
    }

    return laplacian * 8.0; // Scale to match expected Laplacian magnitude
}

// Evolve wave equation for one channel: d^2A/dt^2 = c^2 * laplacian - damping * dA/dt
fn evolve_wave_channel(
    amplitude: f32,
    velocity: f32,
    laplacian: f32,
    wave_speed: f32,
    damping: f32,
    dt: f32
) -> vec2<f32> {
    let c2 = wave_speed * wave_speed;
    let acceleration = c2 * laplacian - damping * velocity;

    // Semi-implicit Euler for stability
    let new_velocity = velocity + acceleration * dt;
    let new_amplitude = amplitude + new_velocity * dt;

    // Clamp amplitude to prevent explosion, but allow some oscillation
    let clamped_amp = clamp(new_amplitude, 0.0, 2.0);

    return vec2<f32>(clamped_amp, new_velocity);
}

// ============================================
// PHASE DYNAMICS
// ============================================

// Compute local "energy" for phase rotation rate
fn compute_phase_energy(amplitudes: vec4<f32>, laplacian_magnitude: f32) -> f32 {
    let max_amp = max(max(amplitudes.x, amplitudes.y), max(amplitudes.z, amplitudes.w));
    return max_amp * 0.3 + laplacian_magnitude * 0.2;
}

// Evolve phases - each state rotates at different rates for visual interest
fn evolve_phases(phases: vec4<f32>, energy: f32, dt: f32) -> vec4<f32> {
    return vec4<f32>(
        wrap_angle(phases.x + energy * dt * 1.0),
        wrap_angle(phases.y - energy * dt * 0.8),  // Counter-rotate
        wrap_angle(phases.z + energy * dt * 1.3),  // Faster
        wrap_angle(phases.w - energy * dt * 0.6)   // Slower counter
    );
}

// ============================================
// MAIN COMPUTE FUNCTION
// ============================================

@compute @workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = global_id.x;
    let y = global_id.y;

    // Bounds check
    if (x >= params.grid_width || y >= params.grid_height) {
        return;
    }

    let idx = y * params.grid_width + x;
    let cell = cells_in[idx];

    // Initialize RNG for this cell this frame
    var rng = pcg_hash(x + pcg_hash(y + pcg_hash(params.frame_number + cell.rng_state)));

    // ============================================
    // COMPUTE EFFECTIVE TIME STEP (Multi-scale time)
    // ============================================
    let entropy = compute_entropy(cell.amplitudes);
    let time_dilation = entropy_to_time_dilation(entropy);
    let effective_dt = params.base_dt * time_dilation;

    // Update local time
    let new_local_time = cell.local_time + effective_dt;

    // ============================================
    // GATHER NEIGHBORS WITH CAUSALITY CHECK
    // ============================================
    var neighbor_amps: array<array<f32, 8>, 4>;  // [channel][neighbor]
    var neighbor_phases: array<array<f32, 8>, 4>;
    var neighbor_weights: array<f32, 8>;
    var neighbor_count = 0u;

    var total_neighbor_amp = vec4<f32>(0.0);

    for (var dy: i32 = -1; dy <= 1; dy++) {
        for (var dx: i32 = -1; dx <= 1; dx++) {
            if (dx == 0 && dy == 0) {
                continue;
            }

            let nx = wrap_coord(i32(x) + dx, params.grid_width);
            let ny = wrap_coord(i32(y) + dy, params.grid_height);
            let neighbor_idx = ny * params.grid_width + nx;
            let neighbor = cells_in[neighbor_idx];

            let distance = sqrt(f32(dx * dx + dy * dy));

            // Relativistic causality check
            if (!is_in_past_light_cone(new_local_time, neighbor.local_time, distance, params.light_speed)) {
                continue; // Skip neighbors outside our light cone
            }

            // Weight: diagonal neighbors weighted less, closer time = more weight
            var weight = select(1.0, 0.707, dx != 0 && dy != 0);
            let time_diff = abs(new_local_time - neighbor.local_time);
            weight *= 1.0 / (1.0 + time_diff * 0.5);

            neighbor_weights[neighbor_count] = weight;

            // Store neighbor amplitudes and phases per channel
            neighbor_amps[0][neighbor_count] = neighbor.amplitudes.x;
            neighbor_amps[1][neighbor_count] = neighbor.amplitudes.y;
            neighbor_amps[2][neighbor_count] = neighbor.amplitudes.z;
            neighbor_amps[3][neighbor_count] = neighbor.amplitudes.w;

            neighbor_phases[0][neighbor_count] = neighbor.phases.x;
            neighbor_phases[1][neighbor_count] = neighbor.phases.y;
            neighbor_phases[2][neighbor_count] = neighbor.phases.z;
            neighbor_phases[3][neighbor_count] = neighbor.phases.w;

            total_neighbor_amp += neighbor.amplitudes * weight;

            neighbor_count++;
        }
    }

    // ============================================
    // WAVE EQUATION EVOLUTION
    // ============================================
    var new_amplitudes = cell.amplitudes;
    var new_velocities = cell.velocities;

    // Compute Laplacian and evolve each channel
    var total_laplacian = 0.0;

    if (neighbor_count > 0u) {
        for (var ch = 0u; ch < 4u; ch++) {
            let center_amp = cell.amplitudes[ch];

            // Build neighbor array for this channel
            var ch_neighbors: array<f32, 8>;
            for (var i = 0u; i < 8u; i++) {
                ch_neighbors[i] = neighbor_amps[ch][i];
            }

            let laplacian = compute_laplacian_channel(
                center_amp,
                ch_neighbors,
                neighbor_weights,
                neighbor_count
            );

            total_laplacian += abs(laplacian);

            let result = evolve_wave_channel(
                center_amp,
                cell.velocities[ch],
                laplacian,
                params.wave_speed,
                params.damping,
                effective_dt
            );

            new_amplitudes[ch] = result.x;
            new_velocities[ch] = result.y;
        }
    }

    // ============================================
    // PHASE EVOLUTION
    // ============================================
    let phase_energy = compute_phase_energy(cell.amplitudes, total_laplacian / 4.0);
    var new_phases = evolve_phases(cell.phases, phase_energy, effective_dt);

    // Phase interference with neighbors (subtle effect)
    if (neighbor_count > 0u) {
        let phase_coupling = 0.05 * effective_dt;
        for (var ch = 0u; ch < 4u; ch++) {
            var phase_influence = 0.0;
            var total_weight = 0.0;
            for (var i = 0u; i < neighbor_count; i++) {
                let phase_diff = neighbor_phases[ch][i] - new_phases[ch];
                phase_influence += sin(phase_diff) * neighbor_weights[i];
                total_weight += neighbor_weights[i];
            }
            if (total_weight > 0.0) {
                new_phases[ch] = wrap_angle(new_phases[ch] + phase_coupling * phase_influence / total_weight);
            }
        }
    }

    // ============================================
    // ENTANGLEMENT (Non-local, ignores causality!)
    // ============================================
    if (has_entanglement(cell.entangled_partner)) {
        let partner_idx = get_partner_index(cell.entangled_partner, params.grid_width);
        let partner = cells_in[partner_idx];

        // Phase synchronization - "spooky action at a distance"
        let entanglement_strength = 0.02;
        for (var i = 0u; i < 4u; i++) {
            if (rand_f32(&rng) < params.randomness_factor * 2.0) {
                let phase_diff = partner.phases[i] - new_phases[i];
                new_phases[i] = wrap_angle(new_phases[i] + entanglement_strength * sin(phase_diff));

                // Slight amplitude correlation too
                new_amplitudes[i] = new_amplitudes[i] * 0.98 + partner.amplitudes[i] * 0.02;
            }
        }
    }

    // ============================================
    // SPONTANEOUS MUTATION (Creates new wave sources)
    // ============================================
    if (rand_f32(&rng) < params.mutation_probability) {
        let flip_to = u32(rand_f32(&rng) * 4.0) % 4u;
        let mutation_amp = 0.9 + rand_f32(&rng) * 0.2;

        // Create a strong pulse in one channel
        new_amplitudes = vec4<f32>(0.2, 0.2, 0.2, 0.2);
        new_amplitudes[flip_to] = mutation_amp;

        // Give it a random phase and outward velocity
        new_phases[flip_to] = rand_f32(&rng) * TWO_PI;
        new_velocities = vec4<f32>(0.0);
        new_velocities[flip_to] = 0.3; // Outward pulse
    }

    // ============================================
    // ADD QUANTUM NOISE
    // ============================================
    for (var i = 0u; i < 4u; i++) {
        new_amplitudes[i] += (rand_f32(&rng) - 0.5) * params.randomness_factor * 0.5;
        new_amplitudes[i] = max(new_amplitudes[i], 0.0);
    }

    // ============================================
    // NORMALIZE AMPLITUDES
    // ============================================
    // Normalize so sum of squared amplitudes (probabilities) = 1
    let sum_sq = new_amplitudes.x * new_amplitudes.x
               + new_amplitudes.y * new_amplitudes.y
               + new_amplitudes.z * new_amplitudes.z
               + new_amplitudes.w * new_amplitudes.w;

    if (sum_sq > 0.0001) {
        let scale = 1.0 / sqrt(sum_sq);
        new_amplitudes = new_amplitudes * scale;
    } else {
        // Reset to uniform if degenerate
        new_amplitudes = vec4<f32>(0.5, 0.5, 0.5, 0.5);
    }

    // ============================================
    // WRITE OUTPUT
    // ============================================
    var out_cell: Cell;
    out_cell.amplitudes = new_amplitudes;
    out_cell.phases = new_phases;
    out_cell.velocities = new_velocities;
    out_cell.local_time = new_local_time;
    out_cell.time_dilation = time_dilation;
    out_cell.entangled_partner = cell.entangled_partner;
    out_cell.rng_state = rng;

    cells_out[idx] = out_cell;
}
