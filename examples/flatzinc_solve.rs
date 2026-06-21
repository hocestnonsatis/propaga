//! Parse and solve a FlatZinc instance from a string.

use propaga_flatzinc::{compile, parse};

fn main() {
    let source = include_str!("../benchmarks/magic_square.fzn");
    let program = parse(source).expect("valid FlatZinc");
    let mut instance = compile(program).expect("supported constraints");
    let solve_vars = instance.solve_vars.clone();
    let names = instance.names.clone();
    let (solution, stats) = instance.model.solve_subset_with_stats(instance.solve_vars);

    match solution {
        Some(sol) => {
            println!("SAT (nodes: {})", stats.nodes);
            for var in solve_vars {
                if let Some(name) = names.get(&var) {
                    if let Some((_, value)) = sol.iter().find(|(v, _)| *v == var) {
                        println!("  {name} = {value}");
                    }
                }
            }
        }
        None => println!("UNSAT"),
    }
}
