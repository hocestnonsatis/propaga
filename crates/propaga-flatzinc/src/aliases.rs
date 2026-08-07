//! FlatZinc predicate aliases emitted by MiniZinc solver libraries.
//!
//! When compiling with a solver profile that declares native `fzn_*` predicates
//! (empty bodies), FlatZinc contains those names instead of stdlib builtins.
//! `array_int_lt` / `array_bool_lt` are common lex aliases.

/// Maps a solver-specific / stdlib-wrapper predicate name to Propaga's canonical
/// FlatZinc builtin name when argument order and shape match.
///
/// Returns `None` when `name` is already canonical or needs a dedicated remap.
#[must_use]
pub fn canonical_constraint_name(name: &str) -> Option<&'static str> {
    Some(match name {
        // All-different
        "fzn_all_different_int" | "fzn_all_different_set" => "all_different",

        // Lexicographic (including array_*_lt aliases seen in some toolchains)
        "fzn_lex_less_int" | "fzn_lex_less_bool" | "array_int_lt" | "array_bool_lt" => "lex_less",
        "fzn_lex_lesseq_int"
        | "fzn_lex_lesseq_bool"
        | "array_int_le"
        | "array_int_leq"
        | "array_bool_le"
        | "array_bool_leq" => "lex_lesseq",

        // Counting / cardinality globals
        "fzn_count_eq" => "count",
        "fzn_at_least_int" => "at_least",
        "fzn_at_most_int" => "at_most",
        "fzn_nvalue" => "nvalue",
        "fzn_among" => "among",
        "fzn_distribute" => "distribute",

        // Scheduling / packing / graph
        "fzn_circuit" => "circuit",
        "fzn_inverse" => "inverse",
        "fzn_diffn" | "fzn_diffn_nonstrict" => "diffn",
        "fzn_disjunctive" | "fzn_disjunctive_strict" => "disjunctive",
        "fzn_sort" => "sort",
        "fzn_table_int" | "fzn_table_bool" => "table",
        "fzn_regular" => "regular",
        "fzn_increasing_int" | "fzn_increasing_bool" => "increasing",
        "fzn_decreasing_int" | "fzn_decreasing_bool" => "decreasing",

        // Monotonicity / lex globals already named without fzn_ in some emits
        "array_int_maximum" => "array_int_maximum",
        "array_int_minimum" => "array_int_minimum",

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
        assert_eq!(canonical_constraint_name("array_int_lt"), Some("lex_less"));
        assert_eq!(
            canonical_constraint_name("fzn_lex_lesseq_int"),
            Some("lex_lesseq")
        );
        assert_eq!(canonical_constraint_name("fzn_count_eq"), Some("count"));
        assert_eq!(canonical_constraint_name("int_eq"), None);
        assert_eq!(resolve_constraint_name("fzn_circuit"), "circuit");
        assert_eq!(resolve_constraint_name("int_lt"), "int_lt");
    }
}
