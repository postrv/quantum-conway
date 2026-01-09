// Hyperbolic Relativistic Wave Simulator - Render Shader
// Visualizes the grid with Poincaré disk projection and phase/time visualization

// ============================================
// DATA STRUCTURES
// ============================================

struct Cell {
    amplitudes: vec4<f32>,      // Wave amplitude magnitudes [+1, -1, +i, -i]
    phases: vec4<f32>,          // Phase angles [0, 2*PI) on unit circle
    velocities: vec4<f32>,      // d(amplitude)/dt for wave equation
    local_time: f32,            // Proper time (relativistic)
    time_dilation: f32,         // Evolution rate based on entropy
    entangled_partner: u32,
    rng_state: u32,
}

struct RenderParams {
    grid_width: u32,
    grid_height: u32,
    render_mode: u32,           // 0 = Euclidean, 1 = Poincaré disk
    phase_visualization: u32,   // 0 = off, 1 = hue shift by phase

    view_center_x: f32,
    view_center_y: f32,
    view_zoom: f32,
    time_viz_strength: f32,

    _padding: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@group(0) @binding(0) var<storage, read> cells: array<Cell>;
@group(0) @binding(1) var<uniform> params: RenderParams;

// ============================================
// CONSTANTS
// ============================================

// Deep ocean color palette
const COLOR_ONE = vec3<f32>(0.0, 0.45, 0.55);          // Deep teal (+1)
const COLOR_MINUS_ONE = vec3<f32>(0.65, 0.25, 0.20);   // Dark coral (-1)
const COLOR_PLUS_I = vec3<f32>(0.35, 0.20, 0.50);      // Deep purple (+i)
const COLOR_MINUS_I = vec3<f32>(0.55, 0.50, 0.15);     // Dark gold (-i)
const BG_COLOR = vec3<f32>(0.05, 0.05, 0.08);          // Near black

const TWO_PI: f32 = 6.28318530718;
const PI: f32 = 3.14159265359;

// ============================================
// POINCARÉ DISK TRANSFORMS
// ============================================

// Hyperbolic arctanh for Poincaré disk
fn atanh_approx(x: f32) -> f32 {
    // atanh(x) = 0.5 * ln((1+x)/(1-x))
    let clamped = clamp(x, -0.999, 0.999);
    return 0.5 * log((1.0 + clamped) / (1.0 - clamped));
}

// Transform from Poincaré disk coordinates to grid coordinates
fn poincare_to_grid(disk_pos: vec2<f32>, zoom: f32, center: vec2<f32>) -> vec2<f32> {
    let r = length(disk_pos);

    if (r >= 0.999) {
        return vec2<f32>(-1.0, -1.0); // Outside disk boundary
    }

    if (r < 0.001) {
        return center; // At center, return view center
    }

    // Hyperbolic distance grows logarithmically near edge
    let hyperbolic_r = 2.0 * atanh_approx(r);

    // Scale factor: how much to expand this radius
    let scale = hyperbolic_r / r;

    // Direction from disk center
    let direction = disk_pos / r;

    // Grid coordinates (centered on view_center)
    return center + direction * scale * zoom;
}

// ============================================
// COLOR MANIPULATION
// ============================================

// Rotate hue by angle (simplified HSV rotation in RGB space)
fn rotate_hue(color: vec3<f32>, angle: f32) -> vec3<f32> {
    let k = vec3<f32>(0.57735, 0.57735, 0.57735); // 1/sqrt(3)
    let cos_a = cos(angle);
    let sin_a = sin(angle);

    return color * cos_a + cross(k, color) * sin_a + k * dot(k, color) * (1.0 - cos_a);
}

// Apply saturation boost based on time dilation
fn apply_time_dilation_color(color: vec3<f32>, time_dilation: f32, strength: f32) -> vec3<f32> {
    // Slow regions (low dilation) get more saturated and slightly brighter
    let slowness = 1.0 - time_dilation; // 0 = fast, 0.9 = slow

    // Increase saturation for slow regions
    let luminance = dot(color, vec3<f32>(0.299, 0.587, 0.114));
    let saturation_boost = 1.0 + slowness * strength;
    var saturated = mix(vec3<f32>(luminance), color, saturation_boost);

    // Add slight glow to slow regions
    saturated = saturated + vec3<f32>(0.1, 0.05, 0.15) * slowness * strength;

    return saturated;
}

// ============================================
// VERTEX SHADER
// ============================================

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // Fullscreen triangle - more efficient than quad
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0)
    );

    var uvs = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(2.0, 1.0),
        vec2<f32>(0.0, -1.0)
    );

    var output: VertexOutput;
    output.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    output.uv = uvs[vertex_index];
    return output;
}

// ============================================
// FRAGMENT SHADER
// ============================================

@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let clamped_uv = clamp(uv, vec2<f32>(0.0), vec2<f32>(1.0));

    var grid_pos: vec2<f32>;
    var is_disk_edge = false;

    // ============================================
    // COORDINATE TRANSFORM (Euclidean or Poincaré)
    // ============================================
    if (params.render_mode == 1u) {
        // Poincaré disk mode
        let disk_pos = (clamped_uv - 0.5) * 2.0; // Map [0,1] -> [-1,1]
        let r = length(disk_pos);

        // Draw disk boundary
        if (r > 0.995) {
            return vec4<f32>(0.3, 0.3, 0.4, 1.0); // Disk edge
        }
        if (r > 0.98) {
            is_disk_edge = true;
        }

        // Transform to grid space
        let view_center = vec2<f32>(params.view_center_x, params.view_center_y);
        grid_pos = poincare_to_grid(disk_pos, params.view_zoom, view_center);

        // Check if outside valid range
        if (grid_pos.x < 0.0 || grid_pos.y < 0.0) {
            return vec4<f32>(BG_COLOR * 0.5, 1.0);
        }
    } else {
        // Standard Euclidean mode
        grid_pos = clamped_uv * vec2<f32>(f32(params.grid_width), f32(params.grid_height));
    }

    // Apply toroidal wrapping
    let grid_x = u32(grid_pos.x) % params.grid_width;
    let grid_y = u32(grid_pos.y) % params.grid_height;

    // Cell gap detection
    let cell_local = fract(grid_pos);
    let gap_size = 0.08;
    let is_gap = cell_local.x < gap_size || cell_local.x > (1.0 - gap_size) ||
                 cell_local.y < gap_size || cell_local.y > (1.0 - gap_size);

    // Bounds check
    if (grid_x >= params.grid_width || grid_y >= params.grid_height) {
        return vec4<f32>(BG_COLOR, 1.0);
    }

    let idx = grid_y * params.grid_width + grid_x;
    let cell = cells[idx];
    let amps = cell.amplitudes;

    // ============================================
    // COMPUTE PROBABILITIES FROM AMPLITUDES
    // ============================================
    let probs = amps * amps; // Probability = amplitude^2
    let total_prob = probs.x + probs.y + probs.z + probs.w;
    var norm_probs = probs;
    if (total_prob > 0.0001) {
        norm_probs = probs / total_prob;
    }

    // ============================================
    // BASE COLOR FROM PROBABILITY BLEND
    // ============================================
    var color = COLOR_ONE * norm_probs.x
              + COLOR_MINUS_ONE * norm_probs.y
              + COLOR_PLUS_I * norm_probs.z
              + COLOR_MINUS_I * norm_probs.w;

    // ============================================
    // PHASE VISUALIZATION (optional hue rotation)
    // ============================================
    if (params.phase_visualization == 1u) {
        // Find dominant state and use its phase for hue rotation
        let max_amp = max(max(amps.x, amps.y), max(amps.z, amps.w));
        var dominant_phase = 0.0;

        if (amps.x == max_amp) { dominant_phase = cell.phases.x; }
        else if (amps.y == max_amp) { dominant_phase = cell.phases.y; }
        else if (amps.z == max_amp) { dominant_phase = cell.phases.z; }
        else { dominant_phase = cell.phases.w; }

        // Rotate hue by phase (subtle effect scaled by certainty)
        let certainty = (max_amp - 0.5) * 2.0; // -1 to 1, higher = more certain
        let rotation_strength = max(certainty, 0.0) * 0.5;
        color = rotate_hue(color, dominant_phase * rotation_strength);
    }

    // ============================================
    // CERTAINTY-BASED BRIGHTNESS
    // ============================================
    let max_prob = max(max(norm_probs.x, norm_probs.y), max(norm_probs.z, norm_probs.w));
    let certainty = (max_prob - 0.25) / 0.75; // 0 when uniform, 1 when certain

    let brightness = 0.3 + certainty * 0.7;
    color = color * brightness;

    // ============================================
    // TIME DILATION VISUALIZATION
    // ============================================
    if (params.time_viz_strength > 0.0) {
        color = apply_time_dilation_color(color, cell.time_dilation, params.time_viz_strength);
    }

    // ============================================
    // VELOCITY GLOW (show wave motion)
    // ============================================
    let velocity_mag = length(cell.velocities);
    if (velocity_mag > 0.1) {
        // Add subtle glow where waves are moving fast
        let glow = min(velocity_mag * 0.5, 0.3);
        color = color + vec3<f32>(glow * 0.3, glow * 0.4, glow * 0.5);
    }

    // ============================================
    // APPLY CELL GAPS
    // ============================================
    if (is_gap) {
        color = color * 0.3;
    }

    // Poincaré disk edge fading
    if (is_disk_edge) {
        color = mix(color, BG_COLOR, 0.5);
    }

    // ============================================
    // DEAD CELL FADING
    // ============================================
    if (max_prob < 0.30) {
        let dead_factor = (0.30 - max_prob) / 0.05;
        color = mix(color, BG_COLOR, clamp(dead_factor, 0.0, 1.0));
    }

    return vec4<f32>(color, 1.0);
}
