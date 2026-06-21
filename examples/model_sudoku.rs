//! Solve a small Sudoku puzzle using the high-level [`Model`] API.

use propaga_domains::IntervalDomain;
use propaga_model::Model;

const PUZZLE: &str = "\
534678912\
126349578\
789125346\
213400000\
000000000\
000000000\
000000000\
000000000\
000000000";

fn main() {
    let mut model = Model::new();
    let mut cells = Vec::with_capacity(81);

    for ch in PUZZLE.chars() {
        let digit = ch.to_digit(10).expect("puzzle must be digits") as i32;
        let var = if digit == 0 {
            model.int_var_domain(IntervalDomain::new(1, 9))
        } else {
            model.int_var_fixed(digit)
        };
        cells.push(var);
    }

    for row in 0..9 {
        let vars: Vec<_> = (0..9).map(|col| cells[row * 9 + col]).collect();
        model.all_different(vars);
    }

    for col in 0..9 {
        let vars: Vec<_> = (0..9).map(|row| cells[row * 9 + col]).collect();
        model.all_different(vars);
    }

    for box_row in 0..3 {
        for box_col in 0..3 {
            let mut vars = Vec::with_capacity(9);
            for row in 0..3 {
                for col in 0..3 {
                    let index = (box_row * 3 + row) * 9 + (box_col * 3 + col);
                    vars.push(cells[index]);
                }
            }
            model.all_different(vars);
        }
    }

    let solution = model
        .solve_subset(cells.clone())
        .expect("sudoku puzzle should be solvable");

    let mut values = vec![0; 81];
    for (var, value) in &solution {
        if let Some(index) = cells.iter().position(|cell| cell == var) {
            values[index] = *value;
        }
    }

    for row in 0..9 {
        for col in 0..9 {
            print!("{}", values[row * 9 + col]);
        }
        println!();
    }
}
