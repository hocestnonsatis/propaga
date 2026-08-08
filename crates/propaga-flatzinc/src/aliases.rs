//! FlatZinc predicate aliases emitted by MiniZinc solver libraries.
//!
//! When compiling with a solver profile that declares native `fzn_*` predicates
//! (empty bodies), FlatZinc contains those names instead of stdlib builtins.
//! `array_int_lt` / `array_bool_lt` are common lex aliases.
//!
//! Dedicated remaps that change argument shape (`fzn_exactly_int`,
//! `fzn_global_cardinality` 3-arg, `fzn_cumulative`) stay in parse/compile,
//! not in this table.

/// Maps a solver-specific / stdlib-wrapper predicate name to Propaga's canonical
/// FlatZinc builtin name when argument order and shape match.
///
/// Returns `None` when `name` is already canonical or needs a dedicated remap.
#[must_use]
pub fn canonical_constraint_name(name: &str) -> Option<&'static str> {
    Some(match name {
        // All-different (underscore and compacted spellings from solver libs)
        "fzn_all_different_int"
        | "fzn_all_different_set"
        | "fzn_all_different_bool"
        | "fzn_alldifferent_int"
        | "fzn_alldifferent" => "all_different",

        // Lexicographic (including array_*_lt aliases seen in some toolchains)
        "fzn_lex_less_int" | "fzn_lex_less_bool" | "array_int_lt" | "array_bool_lt" => "lex_less",
        "fzn_lex_lesseq_int"
        | "fzn_lex_lesseq_bool"
        | "array_int_le"
        | "array_int_leq"
        | "array_bool_le"
        | "array_bool_leq" => "lex_lesseq",
        "fzn_lex_greater_int" | "fzn_lex_greater_bool" | "array_int_gt" | "array_bool_gt" => {
            "lex_greater"
        }
        "fzn_lex_greatereq_int"
        | "fzn_lex_greatereq_bool"
        | "array_int_ge"
        | "array_int_geq"
        | "array_bool_ge"
        | "array_bool_geq" => "lex_greatereq",

        // Counting / cardinality globals
        "fzn_count_eq" | "fzn_count" => "count",
        "fzn_at_least_int" | "fzn_at_least" => "at_least",
        "fzn_at_most_int" | "fzn_at_most" => "at_most",
        "fzn_nvalue" | "fzn_nvalue_int" => "nvalue",
        "fzn_among" | "fzn_among_int" => "among",
        "fzn_distribute" | "fzn_distribute_int" => "distribute",

        // Scheduling / packing / graph
        "fzn_circuit" | "fzn_circuit_int" => "circuit",
        "fzn_inverse" | "fzn_inverse_int" => "inverse",
        "fzn_diffn" | "fzn_diffn_nonstrict" => "diffn",
        "fzn_disjunctive" | "fzn_disjunctive_strict" => "disjunctive",
        "fzn_sort" | "fzn_sort_int" => "sort",
        "fzn_table_int" | "fzn_table_bool" | "fzn_table" => "table",
        "fzn_regular" | "fzn_regular_int" => "regular",
        "fzn_increasing_int" | "fzn_increasing_bool" | "fzn_increasing" => "increasing",
        "fzn_decreasing_int" | "fzn_decreasing_bool" | "fzn_decreasing" => "decreasing",

        // Monotonicity / lex globals already named without fzn_ in some emits
        "array_int_maximum" => "array_int_maximum",
        "array_int_minimum" => "array_int_minimum",
        "array_float_maximum" => "array_float_maximum",
        "array_float_minimum" => "array_float_minimum",

        _ => return None,
    })
}

/// Resolves `name` to the parse/dispatch key (canonical alias or original).
#[must_use]
pub fn resolve_constraint_name(name: &str) -> &str {
    canonical_constraint_name(name).unwrap_or(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_common_fzn_and_array_aliases() {
        assert_eq!(
            canonical_constraint_name("fzn_all_different_int"),
            Some("all_different")
        );
        assert_eq!(
            canonical_constraint_name("fzn_alldifferent_int"),
            Some("all_different")
        );
        assert_eq!(canonical_constraint_name("array_int_lt"), Some("lex_less"));
        assert_eq!(
            canonical_constraint_name("fzn_lex_lesseq_int"),
            Some("lex_lesseq")
        );
        assert_eq!(
            canonical_constraint_name("fzn_lex_greater_int"),
            Some("lex_greater")
        );
        assert_eq!(
            canonical_constraint_name("array_int_geq"),
            Some("lex_greatereq")
        );
        assert_eq!(canonical_constraint_name("fzn_count_eq"), Some("count"));
        assert_eq!(canonical_constraint_name("fzn_count"), Some("count"));
        assert_eq!(canonical_constraint_name("int_eq"), None);
        assert_eq!(resolve_constraint_name("fzn_circuit"), "circuit");
        assert_eq!(resolve_constraint_name("fzn_increasing"), "increasing");
        assert_eq!(resolve_constraint_name("int_lt"), "int_lt");
        assert_eq!(
            canonical_constraint_name("array_float_maximum"),
            Some("array_float_maximum")
        );
        // Dedicated remaps stay outside this table.
        assert_eq!(canonical_constraint_name("fzn_exactly_int"), None);
        assert_eq!(canonical_constraint_name("fzn_cumulative"), None);
    }
}
