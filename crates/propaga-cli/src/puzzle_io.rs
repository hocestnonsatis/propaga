use serde::Deserialize;
use std::io;
use std::path::Path;

/// Output format for solver results.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OutputFormat {
    #[default]
    Plain,
    Json,
}

impl OutputFormat {
    /// Parses a format name from CLI input.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "plain" | "text" => Some(Self::Plain),
            "json" => Some(Self::Json),
            _ => None,
        }
    }
}

/// Shared CLI options for solver commands.
#[derive(Clone, Copy, Debug)]
pub struct GlobalOptions {
    pub format: OutputFormat,
    pub stats: bool,
    pub all: bool,
    pub quiet: bool,
    pub visual: bool,
    pub learning: bool,
    pub restarts: propaga_search::RestartPolicy,
    pub value_ordering: propaga_search::ValueOrdering,
    pub variable_ordering: propaga_search::VariableOrdering,
    pub phase_saving: bool,
    pub solutions_limit: Option<usize>,
    pub time_limit: Option<std::time::Duration>,
    /// `true` when `--restarts` was explicitly passed on the command line.
    pub restarts_explicit: bool,
    /// `true` when `--var-ordering` was explicitly passed on the command line.
    pub variable_ordering_explicit: bool,
    /// `true` when `--value-ordering` was explicitly passed on the command line.
    pub value_ordering_explicit: bool,
    /// `true` when `--no-learning` was explicitly passed on the command line.
    pub _no_learning_explicit: bool,
    /// `true` when `--no-phase-saving` was explicitly passed on the command line.
    pub _no_phase_saving_explicit: bool,
}

impl Default for GlobalOptions {
    fn default() -> Self {
        Self {
            format: OutputFormat::Plain,
            stats: false,
            all: false,
            quiet: false,
            visual: false,
            learning: true,
            restarts: propaga_search::RestartPolicy::default(),
            value_ordering: propaga_search::ValueOrdering::default(),
            variable_ordering: propaga_search::VariableOrdering::default(),
            phase_saving: true,
            solutions_limit: None,
            time_limit: None,
            restarts_explicit: false,
            variable_ordering_explicit: false,
            value_ordering_explicit: false,
            _no_learning_explicit: false,
            _no_phase_saving_explicit: false,
        }
    }
}

impl GlobalOptions {
    /// Builds the search configuration for solver commands.
    #[must_use]
    pub fn search_config(&self) -> propaga_search::SearchConfig {
        propaga_search::SearchConfig {
            learning: self.learning,
            restart_policy: if self.all {
                propaga_search::RestartPolicy::None
            } else {
                self.restarts
            },
            value_ordering: self.value_ordering,
            variable_ordering: self.variable_ordering,
            phase_saving: self.phase_saving,
            time_limit: self.time_limit,
        }
    }

    /// Merges FlatZinc annotation search settings with CLI options.
    ///
    /// Annotation values are used as defaults; explicitly provided CLI flags override them.
    #[must_use]
    pub fn merge_flatzinc_search_config(
        &self,
        annotation: Option<propaga_flatzinc::AnnotationSearchConfig>,
    ) -> propaga_search::SearchConfig {
        let mut config = self.search_config();
        let Some(annotation) = annotation else {
            return config;
        };

        if !self.variable_ordering_explicit {
            config.variable_ordering = annotation.variable_ordering;
        }
        if !self.value_ordering_explicit {
            config.value_ordering = annotation.value_ordering;
        }
        if !self.restarts_explicit && !self.all {
            config.restart_policy = annotation.restart_policy;
        }
        config
    }

    /// Effective solution limit when enumerating with `--all`.
    #[must_use]
    pub fn effective_solutions_limit(&self) -> Option<usize> {
        if self.all { self.solutions_limit } else { None }
    }
}

/// Parses Sudoku puzzle text into 81 digits.
pub fn parse_sudoku_input(text: &str) -> io::Result<Vec<i32>> {
    if let Ok(json) = serde_json::from_str::<SudokuJson>(text) {
        return json.into_digits();
    }

    if text.trim_start().starts_with('{') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid sudoku JSON payload",
        ));
    }

    let digits: Vec<i32> = text
        .chars()
        .filter_map(|ch| ch.to_digit(10))
        .map(|digit| digit as i32)
        .collect();

    if digits.len() == 81 {
        return Ok(digits);
    }

    let grid_digits: Vec<i32> = text
        .split(|ch: char| ch.is_whitespace() || ch == '.' || ch == ',')
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.parse().ok())
        .collect();

    if grid_digits.len() == 81 {
        return Ok(grid_digits);
    }

    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        "expected 81 sudoku digits",
    ))
}

/// Loads one or more Sudoku puzzles from a file.
pub fn load_sudoku_file(path: &Path) -> io::Result<Vec<Vec<i32>>> {
    let content = std::fs::read_to_string(path)?;
    if let Ok(json) = serde_json::from_str::<SudokuBatchJson>(&content) {
        return json.into_puzzles();
    }

    let mut puzzles = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        puzzles.push(parse_sudoku_input(trimmed)?);
    }

    if puzzles.is_empty() {
        puzzles.push(parse_sudoku_input(&content)?);
    }

    Ok(puzzles)
}

#[derive(Debug, Deserialize)]
struct SudokuJson {
    grid: Option<Vec<Vec<i32>>>,
    puzzle: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SudokuBatchJson {
    puzzles: Option<Vec<String>>,
    grid: Option<Vec<Vec<i32>>>,
    puzzle: Option<String>,
}

impl SudokuJson {
    fn into_digits(self) -> io::Result<Vec<i32>> {
        if let Some(grid) = self.grid {
            return flatten_grid(grid);
        }
        if let Some(puzzle) = self.puzzle {
            return parse_sudoku_input(&puzzle);
        }
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "sudoku JSON must contain `grid` or `puzzle`",
        ))
    }
}

impl SudokuBatchJson {
    fn into_puzzles(self) -> io::Result<Vec<Vec<i32>>> {
        if let Some(puzzles) = self.puzzles {
            return puzzles
                .iter()
                .map(|puzzle| parse_sudoku_input(puzzle))
                .collect();
        }
        if let Some(grid) = self.grid {
            return Ok(vec![flatten_grid(grid)?]);
        }
        if let Some(puzzle) = self.puzzle {
            return Ok(vec![parse_sudoku_input(&puzzle)?]);
        }
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "batch JSON must contain `puzzles`, `grid`, or `puzzle`",
        ))
    }
}

fn flatten_grid(grid: Vec<Vec<i32>>) -> io::Result<Vec<i32>> {
    let digits: Vec<i32> = grid.into_iter().flatten().collect();
    if digits.len() == 81 {
        Ok(digits)
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "sudoku grid must contain 81 cells",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_compact_string() {
        let digits = parse_sudoku_input("1".repeat(81).as_str()).unwrap();
        assert_eq!(digits.len(), 81);
    }

    #[test]
    fn parses_dot_separated_grid() {
        let text = (0..81)
            .map(|index| char::from(b'0' + (index % 10) as u8))
            .collect::<String>();
        let spaced = text
            .chars()
            .enumerate()
            .map(|(index, ch)| {
                if index % 9 == 8 {
                    format!("{ch}\n")
                } else {
                    format!("{ch}.")
                }
            })
            .collect::<String>();
        let digits = parse_sudoku_input(&spaced).unwrap();
        assert_eq!(digits.len(), 81);
    }
}
