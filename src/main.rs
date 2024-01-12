extern crate piston_window;
extern crate rand;
extern crate rayon;

use rand::Rng;
use std::sync::Arc;

use piston_window::{clear, rectangle, PistonWindow, WindowSettings};
use rayon::prelude::*;
use std::time::{Duration, Instant};
#[derive(Clone, Debug, Copy)]
enum BasicState {
    One,
    MinusOne,
    ComplexI,
    ComplexMinusI,
}

#[derive(Clone, Debug, Copy)]
struct CellState {
    state_probabilities: [f64; 4], // Probabilities for each basic state
    entangled_partner: Option<(usize, usize)>, // Optional entangled partner coordinates
}

impl Grid {
    fn calculate_state_distribution(&self) -> StateDistribution {
        let mut distribution = StateDistribution {
            one: 0,
            minus_one: 0,
            complex: 0,
        };

        for row in &self.cells {
            for cell in row {
                let max_index = cell
                    .state_probabilities
                    .iter()
                    .enumerate()
                    .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                    .map(|(index, _)| index);

                match max_index {
                    Some(0) => distribution.one += 1,
                    Some(1) => distribution.minus_one += 1,
                    Some(2) | Some(3) => distribution.complex += 1,
                    _ => {}
                }
            }
        }

        distribution
    }

    fn calculate_new_state(grid: &Vec<Vec<CellState>>, current_state: &CellState, neighbors: &Vec<CellState>) -> CellState {
        let mut new_state = current_state.clone();
        let mut rng = rand::thread_rng();

        // Reduce the randomness factor
        let randomness_factor = 0.01; // Smaller randomness factor

        // Entanglement Logic - enhanced for more structured behavior
        if let Some((partner_x, partner_y)) = current_state.entangled_partner {
            let partner_state = &grid[partner_x][partner_y];

            // Example: Synchronize states if certain conditions are met
            for i in 0..4 {
                if rng.gen::<f64>() < randomness_factor {
                    new_state.state_probabilities[i] = (new_state.state_probabilities[i] + partner_state.state_probabilities[i]) / 2.0;
                }
            }
        }

        // Calculate the weighted influence of neighbors
        let mut neighbor_influence = [0.0; 4];
        for neighbor in neighbors {
            for (i, &prob) in neighbor.state_probabilities.iter().enumerate() {
                neighbor_influence[i] += prob;
            }
        }

        // Normalize the influence
        let total_influence: f64 = neighbor_influence.iter().sum();
        if total_influence > 0.0 {
            for influence in &mut neighbor_influence {
                *influence /= total_influence;
            }
        }

        // Update state probabilities based on neighbor influence and some randomness
        for (i, prob) in new_state.state_probabilities.iter_mut().enumerate() {
            *prob = (*prob + neighbor_influence[i]) / 2.0;
            *prob += rng.gen::<f64>() * randomness_factor; // Reduced randomness
        }

        // Ensure probabilities sum to 1
        let total_prob: f64 = new_state.state_probabilities.iter().sum();
        for prob in &mut new_state.state_probabilities {
            *prob /= total_prob;
        }

        new_state
    }
}

struct Grid {
    cells: Vec<Vec<CellState>>,
    width: usize,
    height: usize,
}

// A struct to hold counts of different types of cell states for the entire grid
struct StateDistribution {
    one: usize,
    minus_one: usize,
    complex: usize,
}

impl Grid {
    fn new(width: usize, height: usize) -> Grid {
        let mut rng = rand::thread_rng();
        let cells = (0..height)
            .map(|_| {
                (0..width)
                    .map(|_| {
                        // Random probabilities for each state
                        let mut probs = [0.0; 4];
                        for p in &mut probs {
                            *p = rng.gen::<f64>();
                        }
                        let sum: f64 = probs.iter().sum();
                        for p in &mut probs {
                            *p /= sum; // Normalize probabilities to sum to 1
                        }

                        // Randomly assign entangled partners (for simplicity, could be improved)
                        let entangled_partner = if rng.gen::<f64>() < 0.88 { // 30% chance of entanglement
                            Some((rng.gen_range(0..width), rng.gen_range(0..height)))
                        } else {
                            None
                        };

                        CellState {
                            state_probabilities: probs,
                            entangled_partner,
                        }
                    })
                    .collect()
            })
            .collect();

        Grid {
            cells,
            width,
            height,
        }
    }

    fn update(&mut self) {
        let width = self.width;
        let height = self.height;
        let cells_arc = Arc::new(self.cells.clone());

        self.cells.par_iter_mut().enumerate().for_each(|(i, row)| {
            for j in 0..width {
                let neighbors = Grid::get_neighbors(&cells_arc, i, j, width, height);
                row[j] = Grid::calculate_new_state(&cells_arc, &cells_arc[i][j], &neighbors);
            }
        });
    }

    fn get_neighbors(
        grid: &Vec<Vec<CellState>>,
        row: usize,
        col: usize,
        width: usize,
        height: usize,
    ) -> Vec<CellState> {
        let mut neighbors = Vec::new();

        for i_offset in -1..=1 {
            for j_offset in -1..=1 {
                if i_offset == 0 && j_offset == 0 {
                    continue; // Skip the cell itself
                }

                let neighbor_row = (row as isize + i_offset).rem_euclid(height as isize) as usize;
                let neighbor_col = (col as isize + j_offset).rem_euclid(width as isize) as usize;

                neighbors.push(grid[neighbor_row][neighbor_col]);
            }
        }

        neighbors
    }

    fn count_neighbors(&self, row: usize, col: usize) -> NeighborCount {
        let mut count = NeighborCount {
            one_or_i: 0,
            minus_i: 0,
            i: 0,
        };

        for i_offset in -1..=1 {
            for j_offset in -1..=1 {
                if i_offset == 0 && j_offset == 0 {
                    continue;
                }

                let neighbor_row =
                    (row as isize + i_offset).rem_euclid(self.height as isize) as usize;
                let neighbor_col =
                    (col as isize + j_offset).rem_euclid(self.width as isize) as usize;

                let cell = self.cells[neighbor_row][neighbor_col];
                let max_index = cell
                    .state_probabilities
                    .iter()
                    .enumerate()
                    .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                    .map(|(index, _)| index);

                match max_index {
                    Some(0) => count.one_or_i += 1,
                    Some(2) => count.i += 1,       // ComplexI
                    Some(3) => count.minus_i += 1, // ComplexMinusI
                    _ => {}
                }
            }
        }

        count
    }

    fn sum_neighbors_complex(&self, row: usize, col: usize) -> (f64, f64) {
        let mut sum_real = 0.0;
        let mut sum_imaginary = 0.0;

        for i_offset in -1..=1 {
            for j_offset in -1..=1 {
                if i_offset == 0 && j_offset == 0 {
                    continue;
                }

                let neighbor_row =
                    (row as isize + i_offset).rem_euclid(self.height as isize) as usize;
                let neighbor_col =
                    (col as isize + j_offset).rem_euclid(self.width as isize) as usize;

                let cell = self.cells[neighbor_row][neighbor_col];
                sum_real += cell.state_probabilities[0] - cell.state_probabilities[1]; // One - MinusOne
                sum_imaginary += cell.state_probabilities[2] - cell.state_probabilities[3];
                // ComplexI - ComplexMinusI
            }
        }

        (sum_real, sum_imaginary)
    }
}
// A struct to hold counts of different types of neighbors
struct NeighborCount {
    one_or_i: usize,
    minus_i: usize,
    i: usize,
}

fn main() {
    let grid_width = 100;
    let grid_height = 100;
    let cell_size = 7; // Size of each cell in pixels

    let mut grid = Grid::new(grid_width, grid_height);
    let mut window: PistonWindow = WindowSettings::new(
        "Quantum Life",
        [
            (grid_width * cell_size) as u32,
            (grid_height * cell_size) as u32,
        ],
    )
    .exit_on_esc(true)
    .build()
    .unwrap_or_else(|e| panic!("Failed to build PistonWindow: {}", e));

    let mut last_update = Instant::now();
    let update_interval = Duration::from_millis(300); // ~ 3 times a second

    while let Some(e) = window.next() {
        if last_update.elapsed() >= update_interval {
            grid.update(); // Update the grid
            last_update = Instant::now();
        }

        window.draw_2d(&e, |c, g, _| {
            clear([1.0; 4], g); // Clear the screen
            for i in 0..grid_height {
                for j in 0..grid_width {
                    let color = {
                        let state = &grid.cells[i][j];
                        let max_prob_state = state
                            .state_probabilities
                            .iter()
                            .enumerate()
                            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                            .map(|(index, _)| index);

                        match max_prob_state {
                            Some(0) => [0.5, 0.5, 0.8, 1.0], // Color for state 'One'
                            Some(1) => [0.8, 0.5, 0.5, 1.0], // Color for state 'MinusOne'
                            Some(2) => [0.5, 0.8, 0.5, 1.0], // Color for state 'ComplexI'
                            Some(3) => [0.8, 0.8, 0.5, 1.0], // Color for state 'ComplexMinusI'
                            _ => [0.5, 0.5, 0.5, 1.0],       // Default or error color
                        }
                    };
                    let square = rectangle::square(
                        (j * cell_size) as f64,
                        (i * cell_size) as f64,
                        cell_size as f64,
                    );
                    rectangle(color, square, c.transform, g); // Draw the rectangle
                }
            }
        });
    }
}
