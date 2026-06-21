use propaga_domains::IntervalDomain;
use propaga_engine::Engine;
use propaga_propagators::AllDifferentPropagator;
use propaga_search::DepthFirstSearch;

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
    let mut engine = Engine::new();
    let mut cells = Vec::with_capacity(81);

    for ch in PUZZLE.chars() {
        let digit = ch.to_digit(10).expect("puzzle must be digits") as i32;
        let domain = if digit == 0 {
            IntervalDomain::new(1, 9)
        } else {
            IntervalDomain::fix(digit)
        };
        cells.push(engine.new_variable(domain));
    }

    for row in 0..9 {
        let vars: Vec<_> = (0..9).map(|col| cells[row * 9 + col]).collect();
        engine.add_propagator(Box::new(AllDifferentPropagator::new(vars)));
    }

    for col in 0..9 {
        let vars: Vec<_> = (0..9).map(|row| cells[row * 9 + col]).collect();
        engine.add_propagator(Box::new(AllDifferentPropagator::new(vars)));
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
            engine.add_propagator(Box::new(AllDifferentPropagator::new(vars)));
        }
    }

    engine.propagate_all().expect("initial propagation");

    let mut search = DepthFirstSearch::new(cells.clone());
    let solution = search
        .solve(&mut engine)
        .expect("sudoku puzzle should be solvable");

    print_grid(&solution, &cells);
}

fn print_grid(solution: &[(propaga_core::VariableId, i32)], cells: &[propaga_core::VariableId]) {
    let mut values = vec![0; 81];
    for (var, value) in solution {
        if let Some(index) = cells.iter().position(|cell| cell == var) {
            values[index] = *value;
        }
    }

    println!("Sudoku solution:");
    for row in 0..9 {
        for col in 0..9 {
            print!("{}", values[row * 9 + col]);
            if col == 2 || col == 5 {
                print!(" ");
            }
        }
        println!();
        if row == 2 || row == 5 {
            println!();
        }
    }
}
