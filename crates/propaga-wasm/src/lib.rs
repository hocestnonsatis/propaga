//! WebAssembly bindings for small Propaga puzzles.

use propaga_core::VariableId;
use propaga_model::Model;
use propaga_search::SearchConfig;
use wasm_bindgen::prelude::*;

/// Solves a 9x9 Sudoku puzzle from an 81-digit string (`0` for empty cells).
#[wasm_bindgen]
pub fn solve_sudoku(puzzle: &str) -> String {
    let digits: Vec<i32> = puzzle
        .chars()
        .filter_map(|ch| ch.to_digit(10))
        .map(|digit| digit as i32)
        .collect();
    if digits.len() != 81 {
        return String::new();
    }

    let mut model = Model::new();
    model.set_search_config(SearchConfig::without_learning());
    let mut cells = Vec::with_capacity(81);
    for &digit in &digits {
        let var = if digit == 0 {
            model.int_var(1, 9)
        } else {
            model.int_var_fixed(digit)
        };
        cells.push(var);
    }
    post_sudoku_constraints(&mut model, &cells);
    let _ = model.propagate();
    let solution = model.solve_subset(cells.clone());
    solution
        .map(|values| {
            let mut grid = vec![0; 81];
            for (var, value) in values {
                if let Some(index) = cells.iter().position(|cell| *cell == var) {
                    grid[index] = value;
                }
            }
            grid.iter().map(|value| value.to_string()).collect()
        })
        .unwrap_or_default()
}

/// Solves N-Queens and returns column positions as comma-separated values.
#[wasm_bindgen]
pub fn solve_n_queens(size: u32) -> String {
    let size = size.clamp(1, 20) as usize;
    let mut model = Model::new();
    model.set_search_config(SearchConfig::without_learning());
    let n = size as i32;
    let queens: Vec<_> = (0..size).map(|_| model.int_var(0, n - 1)).collect();
    model.all_different(queens.clone());
    for i in 0..size {
        for j in (i + 1)..size {
            let offset = (j - i) as i32;
            model.not_equal_offset(queens[i], queens[j], offset);
            model.not_equal_offset(queens[i], queens[j], -offset);
        }
    }
    let _ = model.propagate();
    model
        .solve_subset(queens.clone())
        .map(|solution| {
            let mut columns = vec![0; size];
            for (var, value) in solution {
                if let Some(index) = queens.iter().position(|queen| *queen == var) {
                    columns[index] = value;
                }
            }
            columns
                .iter()
                .map(|column| column.to_string())
                .collect::<Vec<_>>()
                .join(",")
        })
        .unwrap_or_default()
}

fn post_sudoku_constraints(model: &mut Model, cells: &[VariableId]) {
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
                    vars.push(cells[(box_row * 3 + row) * 9 + (box_col * 3 + col)]);
                }
            }
            model.all_different(vars);
        }
    }
}
