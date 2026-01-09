use rand::Rng;
use crate::config::{ENTANGLEMENT_PROBABILITY, GRID_WIDTH, GRID_HEIGHT};
use crate::simulation::cell::GpuCell;

/// Grid of cells for initialization
pub struct Grid {
    pub cells: Vec<GpuCell>,
}

impl Grid {
    /// Create a new grid with random initial states
    pub fn new(width: u32, height: u32) -> Self {
        let mut rng = rand::thread_rng();
        let cell_count = (width * height) as usize;
        let mut cells = Vec::with_capacity(cell_count);

        for y in 0..height {
            for x in 0..width {
                // Generate polarized probabilities - cells start "alive" with dominant states
                let mut probs = [0.0f32; 4];

                // Pick a dominant state randomly
                let dominant = rng.gen_range(0..4);

                // Give dominant state 50-80% probability
                probs[dominant] = rng.gen_range(0.5..0.8);

                // Distribute remaining probability among others
                let remaining = 1.0 - probs[dominant];
                for i in 0..4 {
                    if i != dominant {
                        probs[i] = remaining / 3.0 + rng.gen::<f32>() * 0.05;
                    }
                }

                // Normalize to sum to 1.0
                let sum: f32 = probs.iter().sum();
                for p in &mut probs {
                    *p /= sum;
                }

                // Randomly assign entangled partner
                let partner = if rng.gen::<f64>() < ENTANGLEMENT_PROBABILITY {
                    Some((
                        rng.gen_range(0..width),
                        rng.gen_range(0..height),
                    ))
                } else {
                    None
                };

                // Generate initial RNG seed from position
                let rng_seed = pcg_hash(x.wrapping_add(pcg_hash(y)));

                cells.push(GpuCell::new(probs, partner, rng_seed));
            }
        }

        Self { cells }
    }

    /// Create a grid with default dimensions
    pub fn new_default() -> Self {
        Self::new(GRID_WIDTH, GRID_HEIGHT)
    }
}

/// PCG hash function for generating deterministic seeds
fn pcg_hash(input: u32) -> u32 {
    let state = input.wrapping_mul(747796405).wrapping_add(2891336453);
    let word = ((state >> ((state >> 28).wrapping_add(4))) ^ state).wrapping_mul(277803737);
    (word >> 22) ^ word
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_creation() {
        let grid = Grid::new(100, 100);
        assert_eq!(grid.cells.len(), 10000);
    }

    #[test]
    fn test_amplitudes_normalized() {
        let grid = Grid::new(10, 10);
        for cell in &grid.cells {
            // Sum of squared amplitudes should equal 1.0 (probability normalization)
            let sum_sq: f32 = cell.amplitudes.iter().map(|a| a * a).sum();
            assert!((sum_sq - 1.0).abs() < 0.01, "Sum of squared amplitudes should be ~1.0, got {}", sum_sq);
        }
    }

    #[test]
    fn test_cell_fields_initialized() {
        let grid = Grid::new(10, 10);
        for cell in &grid.cells {
            // Phases should be initialized to 0
            for phase in &cell.phases {
                assert_eq!(*phase, 0.0, "Phases should be initialized to 0");
            }
            // Velocities should be initialized to 0
            for vel in &cell.velocities {
                assert_eq!(*vel, 0.0, "Velocities should be initialized to 0");
            }
            // Local time should be 0
            assert_eq!(cell.local_time, 0.0, "Local time should be 0");
            // Time dilation should be 1.0
            assert_eq!(cell.time_dilation, 1.0, "Time dilation should be 1.0");
        }
    }
}
