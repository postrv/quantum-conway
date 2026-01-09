use crate::config::NO_ENTANGLEMENT;

/// GPU-compatible cell structure for hyperbolic relativistic wave simulation.
///
/// Layout: 64 bytes total, aligned for efficient GPU access.
/// - amplitudes: [f32; 4] = 16 bytes - Wave amplitude magnitudes for [+1, -1, +i, -i]
/// - phases: [f32; 4] = 16 bytes - Phase angles [0, 2π) on unit circle
/// - velocities: [f32; 4] = 16 bytes - d(amplitude)/dt for wave equation
/// - local_time: f32 = 4 bytes - Proper time (relativistic)
/// - time_dilation: f32 = 4 bytes - Evolution rate [0.1, 1.0] based on entropy
/// - entangled_partner: u32 = 4 bytes - Packed (x,y) or NO_ENTANGLEMENT
/// - rng_state: u32 = 4 bytes - PCG state for GPU random
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuCell {
    /// Wave amplitude magnitudes for states: [+1, -1, +i, -i]
    /// Probability = amplitude^2, so sqrt(probability) = amplitude
    pub amplitudes: [f32; 4],

    /// Phase angles [0, 2π) for each state on the unit circle
    /// Full wavefunction: ψ_i = amplitude_i * e^(i * phase_i)
    pub phases: [f32; 4],

    /// Velocity field: d(amplitude)/dt for wave equation propagation
    pub velocities: [f32; 4],

    /// Local proper time (for relativistic causality)
    pub local_time: f32,

    /// Time dilation factor [0.1, 1.0] based on local entropy
    /// Low entropy (certain) = fast evolution, High entropy = slow
    pub time_dilation: f32,

    /// Entangled partner encoded as single u32:
    /// - High 16 bits: partner_x (0-65535)
    /// - Low 16 bits: partner_y (0-65535)
    /// - Value 0xFFFFFFFF means no entanglement
    pub entangled_partner: u32,

    /// Per-cell PRNG state (updated each frame by GPU)
    pub rng_state: u32,
}

impl GpuCell {
    /// Create a new cell with given amplitudes (derived from probabilities) and optional entanglement
    pub fn new(probabilities: [f32; 4], partner: Option<(u32, u32)>, rng_seed: u32) -> Self {
        // Convert probabilities to amplitudes (amplitude = sqrt(probability))
        let amplitudes = [
            probabilities[0].sqrt(),
            probabilities[1].sqrt(),
            probabilities[2].sqrt(),
            probabilities[3].sqrt(),
        ];

        Self {
            amplitudes,
            phases: [0.0, 0.0, 0.0, 0.0],     // Start with zero phase
            velocities: [0.0, 0.0, 0.0, 0.0], // Start at rest
            local_time: 0.0,                   // Start at t=0
            time_dilation: 1.0,                // Normal time flow initially
            entangled_partner: encode_partner(partner),
            rng_state: rng_seed,
        }
    }

    /// Create a cell with explicit amplitudes and phases
    pub fn new_with_phases(
        amplitudes: [f32; 4],
        phases: [f32; 4],
        partner: Option<(u32, u32)>,
        rng_seed: u32,
    ) -> Self {
        Self {
            amplitudes,
            phases,
            velocities: [0.0, 0.0, 0.0, 0.0],
            local_time: 0.0,
            time_dilation: 1.0,
            entangled_partner: encode_partner(partner),
            rng_state: rng_seed,
        }
    }
}

/// Encode (x, y) coordinates into a single u32
/// Returns NO_ENTANGLEMENT for None
pub fn encode_partner(partner: Option<(u32, u32)>) -> u32 {
    match partner {
        Some((x, y)) => (x << 16) | (y & 0xFFFF),
        None => NO_ENTANGLEMENT,
    }
}

/// Decode partner from u32
#[allow(dead_code)]
pub fn decode_partner(encoded: u32) -> Option<(u32, u32)> {
    if encoded == NO_ENTANGLEMENT {
        None
    } else {
        Some((encoded >> 16, encoded & 0xFFFF))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cell_size() {
        assert_eq!(std::mem::size_of::<GpuCell>(), 64);
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let coords = (500, 300);
        let encoded = encode_partner(Some(coords));
        let decoded = decode_partner(encoded);
        assert_eq!(decoded, Some(coords));
    }

    #[test]
    fn test_no_entanglement() {
        let encoded = encode_partner(None);
        assert_eq!(encoded, NO_ENTANGLEMENT);
        assert_eq!(decode_partner(encoded), None);
    }

    #[test]
    fn test_large_coords() {
        // Note: (65535, 65535) encodes to 0xFFFFFFFF which is NO_ENTANGLEMENT
        // So we test with coords that don't collide
        let coords = (65534, 65535);
        let encoded = encode_partner(Some(coords));
        let decoded = decode_partner(encoded);
        assert_eq!(decoded, Some(coords));

        let coords2 = (65535, 65534);
        let encoded2 = encode_partner(Some(coords2));
        let decoded2 = decode_partner(encoded2);
        assert_eq!(decoded2, Some(coords2));
    }
}
