mod flatzinc;
mod n_queens;
mod output;
mod puzzle_io;
mod schedule;
mod sudoku;

use clap::{Parser, Subcommand};
use propaga_search::RestartPolicy;
use propaga_search::{ValueOrdering, VariableOrdering};
use puzzle_io::{GlobalOptions, OutputFormat};
use std::path::PathBuf;

/// Propaga constraint solver command-line interface.
#[derive(Parser)]
#[command(
    name = "propaga",
    version,
    about = "Propagator-based constraint solver"
)]
struct Cli {
    /// Output format: plain or json.
    #[arg(long, global = true, default_value = "plain")]
    format: String,

    /// Print search statistics.
    #[arg(long, global = true)]
    stats: bool,

    /// Enumerate all solutions.
    #[arg(long, global = true)]
    all: bool,

    /// Suppress decorative output.
    #[arg(long, global = true)]
    quiet: bool,

    /// Disable nogood learning during search.
    #[arg(long, global = true)]
    no_learning: bool,

    /// Restart policy: none, luby, or luby:N (e.g. luby:256).
    #[arg(long, global = true, default_value = "luby")]
    restarts: String,

    /// Value ordering during search: asc or lcv.
    #[arg(long, global = true, default_value = "asc")]
    value_ordering: String,

    /// Disable phase saving (reuse last assigned value as first branch candidate).
    #[arg(long, global = true)]
    no_phase_saving: bool,

    /// Variable ordering during search: mrv, dom, or dom-wdeg.
    #[arg(long, global = true, default_value = "mrv")]
    var_ordering: String,

    /// Maximum number of solutions to emit with `--all`.
    #[arg(long, global = true)]
    solutions: Option<usize>,

    /// Wall-clock time limit in seconds for search.
    #[arg(long, global = true, value_name = "SECS")]
    time_limit: Option<f64>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Solve a Sudoku puzzle.
    Sudoku {
        /// Puzzle as 81 digits with 0 for empty cells.
        #[arg(long)]
        puzzle: Option<String>,

        /// Path to a puzzle file containing 81 digits or one puzzle per line.
        #[arg(long)]
        file: Option<PathBuf>,

        /// Output format override for this command.
        #[arg(long)]
        format: Option<String>,
    },
    /// Solve the N-Queens problem.
    NQueens {
        /// Board size.
        #[arg(long, default_value_t = 8)]
        size: usize,

        /// Render an ASCII chessboard.
        #[arg(long)]
        visual: bool,

        /// Output format override for this command.
        #[arg(long)]
        format: Option<String>,
    },
    /// Solve a FlatZinc instance (subset).
    #[command(group(clap::ArgGroup::new("solve_input").required(true).multiple(false)))]
    Solve {
        /// Path to a `.fzn` FlatZinc file.
        #[arg(long, group = "solve_input")]
        file: Option<PathBuf>,

        /// Directory of `.fzn` files to solve in batch.
        #[arg(long, group = "solve_input")]
        dir: Option<PathBuf>,

        /// Output format override for this command.
        #[arg(long)]
        format: Option<String>,
    },
    /// Solve a cumulative scheduling instance from JSON.
    Schedule {
        /// Path to a JSON schedule specification.
        #[arg(long)]
        file: PathBuf,

        /// Output format override for this command.
        #[arg(long)]
        format: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();
    let global_format = OutputFormat::parse(&cli.format).unwrap_or_else(|| {
        eprintln!("unknown format `{}`, using plain", cli.format);
        OutputFormat::Plain
    });

    let restart_policy = RestartPolicy::parse(&cli.restarts).unwrap_or_else(|| {
        eprintln!("unknown restart policy `{}`, using luby", cli.restarts);
        RestartPolicy::default()
    });

    let value_ordering = ValueOrdering::parse(&cli.value_ordering).unwrap_or_else(|| {
        eprintln!(
            "unknown value ordering `{}`, using ascending",
            cli.value_ordering
        );
        ValueOrdering::default()
    });

    let variable_ordering = VariableOrdering::parse(&cli.var_ordering).unwrap_or_else(|| {
        eprintln!(
            "unknown variable ordering `{}`, using mrv",
            cli.var_ordering
        );
        VariableOrdering::default()
    });

    let base_options = GlobalOptions {
        stats: cli.stats,
        all: cli.all,
        quiet: cli.quiet,
        learning: !cli.no_learning,
        restarts: restart_policy,
        value_ordering,
        variable_ordering,
        phase_saving: !cli.no_phase_saving,
        solutions_limit: cli.solutions,
        time_limit: cli
            .time_limit
            .map(|secs| std::time::Duration::from_secs_f64(secs.max(0.0))),
        ..GlobalOptions::default()
    };

    let result = match cli.command {
        Commands::Sudoku {
            puzzle,
            file,
            format,
        } => {
            let options = GlobalOptions {
                format: format
                    .as_deref()
                    .and_then(OutputFormat::parse)
                    .unwrap_or(global_format),
                ..base_options
            };
            sudoku::run(puzzle, file.as_deref(), options)
        }
        Commands::NQueens {
            size,
            visual,
            format,
        } => {
            let options = GlobalOptions {
                format: format
                    .as_deref()
                    .and_then(OutputFormat::parse)
                    .unwrap_or(global_format),
                visual,
                ..base_options
            };
            n_queens::run(size, options)
        }
        Commands::Solve { file, dir, format } => {
            let options = GlobalOptions {
                format: format
                    .as_deref()
                    .and_then(OutputFormat::parse)
                    .unwrap_or(global_format),
                ..base_options
            };
            match (file, dir) {
                (Some(file), None) => flatzinc::run(&file, options),
                (None, Some(dir)) => flatzinc::run_dir(&dir, options),
                _ => Err("exactly one of --file or --dir is required".into()),
            }
        }
        Commands::Schedule { file, format } => {
            let options = GlobalOptions {
                format: format
                    .as_deref()
                    .and_then(OutputFormat::parse)
                    .unwrap_or(global_format),
                ..base_options
            };
            schedule::run(&file, options)
        }
    };

    if let Err(error) = result {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::puzzle_io::parse_sudoku_input;

    #[test]
    fn parses_json_puzzle_field() {
        let json = r#"{"puzzle":"000000000000000000000000000000000000000000000000000000000000000000000000000000000"}"#;
        let digits = parse_sudoku_input(json).unwrap();
        assert_eq!(digits.len(), 81);
    }
}
