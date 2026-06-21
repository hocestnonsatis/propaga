use propaga_core::{ChangeReason, Explanation, Nogood, NogoodLiteral, VariableId};
use std::collections::{HashMap, HashSet};

/// Analyzes propagation conflicts and extracts first-UIP nogoods.
pub struct ConflictAnalyzer;

impl ConflictAnalyzer {
    /// Builds a first-UIP nogood from a conflict explanation.
    #[must_use]
    pub fn analyze(explanation: &Explanation, conflict_var: VariableId) -> Nogood {
        let branch_order: Vec<NogoodLiteral> = explanation.branch_literals().collect();
        let levels = build_level_map(&branch_order);
        let literals = backward_nogood(explanation, conflict_var);
        let trimmed = apply_first_uip(literals, &levels);
        Nogood::new(trimmed)
    }

    /// Returns the trail level to backjump to for `nogood` given branch order.
    #[must_use]
    pub fn backjump_level(nogood: &Nogood, branch_order: &[NogoodLiteral]) -> usize {
        let levels = build_level_map(branch_order);
        let mut decision_levels: Vec<usize> = nogood
            .literals()
            .iter()
            .filter_map(|literal| levels.get(&(literal.variable, literal.value)).copied())
            .collect();
        decision_levels.sort_unstable();
        decision_levels.dedup();
        match decision_levels.len() {
            0 => 0,
            1 => decision_levels[0],
            _ => decision_levels[decision_levels.len() - 2],
        }
    }
}

fn build_level_map(branch_order: &[NogoodLiteral]) -> HashMap<(VariableId, i32), usize> {
    branch_order
        .iter()
        .enumerate()
        .map(|(level, literal)| ((literal.variable, literal.value), level))
        .collect()
}

fn backward_nogood(
    explanation: &Explanation,
    conflict_var: VariableId,
) -> HashSet<(VariableId, i32)> {
    if let Some(literals) = explanation.propagator_conflict_literals() {
        return literals
            .into_iter()
            .map(|literal| (literal.variable, literal.value))
            .collect();
    }

    let entries = explanation.entries();
    let mut to_explain = HashSet::from([conflict_var]);
    let mut nogood = HashSet::new();

    for i in (0..entries.len()).rev() {
        match &entries[i] {
            ChangeReason::Branch { variable, value } => {
                if to_explain.remove(variable) {
                    nogood.insert((*variable, *value));
                }
            }
            ChangeReason::Propagator { variable, .. } => {
                let _ = to_explain.remove(variable);
            }
            ChangeReason::PropagatorConflict { .. } => {}
        }
    }

    if nogood.is_empty() {
        for literal in explanation.branch_literals() {
            nogood.insert((literal.variable, literal.value));
        }
    }

    nogood
}

fn apply_first_uip(
    literals: HashSet<(VariableId, i32)>,
    levels: &HashMap<(VariableId, i32), usize>,
) -> Vec<NogoodLiteral> {
    if literals.is_empty() {
        return Vec::new();
    }

    let max_level = literals
        .iter()
        .map(|key| levels.get(key).copied().unwrap_or(0))
        .max()
        .unwrap_or(0);

    let mut by_level: HashMap<usize, Vec<NogoodLiteral>> = HashMap::new();
    for (variable, value) in literals {
        let level = levels.get(&(variable, value)).copied().unwrap_or(0);
        by_level
            .entry(level)
            .or_default()
            .push(NogoodLiteral { variable, value });
    }

    let mut result = Vec::new();
    for (level, mut level_literals) in by_level {
        if level < max_level {
            result.append(&mut level_literals);
        } else if level == max_level {
            level_literals.sort_by_key(|literal| (literal.variable.key(), literal.value));
            if let Some(uip) = level_literals.into_iter().next() {
                result.push(uip);
            }
        }
    }

    result.sort_by_key(|literal| {
        levels
            .get(&(literal.variable, literal.value))
            .copied()
            .unwrap_or(0)
    });
    result
}

/// Stores learned nogoods and supports pruning checks.
#[derive(Clone, Debug, Default)]
pub struct NogoodStore {
    nogoods: Vec<Nogood>,
}

impl NogoodStore {
    /// Creates an empty nogood store.
    #[must_use]
    pub fn new() -> Self {
        Self {
            nogoods: Vec::new(),
        }
    }

    /// Returns the number of learned nogoods.
    #[must_use]
    pub fn len(&self) -> usize {
        self.nogoods.len()
    }

    /// Returns `true` when no nogoods have been learned.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nogoods.is_empty()
    }

    /// Returns all learned nogoods.
    #[must_use]
    pub fn nogoods(&self) -> &[Nogood] {
        &self.nogoods
    }

    /// Records a nogood if it is not already known.
    pub fn learn(&mut self, nogood: Nogood) -> bool {
        if nogood.literals().is_empty() {
            return false;
        }
        if self
            .nogoods
            .iter()
            .any(|existing| same_literals(existing, &nogood))
        {
            return false;
        }
        self.nogoods.push(nogood);
        true
    }

    /// Returns `true` when `assignment` satisfies any learned nogood.
    #[must_use]
    pub fn is_violated(&self, assignment: &[(VariableId, i32)]) -> bool {
        self.nogoods
            .iter()
            .any(|nogood| nogood.is_satisfied_by(assignment))
    }

    /// Returns `true` when assigning `(var, value)` would satisfy a nogood.
    #[must_use]
    pub fn would_violate(
        &self,
        assignment: &[(VariableId, i32)],
        var: VariableId,
        value: i32,
    ) -> bool {
        self.nogoods
            .iter()
            .any(|nogood| nogood.would_be_satisfied_by(assignment, var, value))
    }

    /// Returns the most recently learned nogood, if any.
    #[must_use]
    pub fn last(&self) -> Option<&Nogood> {
        self.nogoods.last()
    }
}

fn same_literals(left: &Nogood, right: &Nogood) -> bool {
    let mut a: Vec<_> = left
        .literals()
        .iter()
        .map(|l| (l.variable, l.value))
        .collect();
    let mut b: Vec<_> = right
        .literals()
        .iter()
        .map(|l| (l.variable, l.value))
        .collect();
    a.sort_unstable_by_key(|(var, _)| var.key());
    b.sort_unstable_by_key(|(var, _)| var.key());
    a == b
}

#[cfg(test)]
mod tests {
    use super::*;
    use propaga_core::Nogood;
    use propaga_domains::IntervalDomain;
    use propaga_engine::Engine;

    #[test]
    fn first_uip_from_real_conflict() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(1, 2));
        let b = engine.new_variable(IntervalDomain::new(1, 2));
        let c = engine.new_variable(IntervalDomain::new(1, 3));
        engine.add_propagator(Box::new(propaga_propagators::AllDifferentPropagator::new(
            vec![a, b, c],
        )));

        engine.trail_mark();
        engine.fix_variable(a, 1).unwrap();
        engine.fix_variable(b, 2).unwrap();
        let _ = engine.fix_variable(c, 1);

        let conflict = engine.last_conflict().expect("conflict");
        let nogood = ConflictAnalyzer::analyze(&conflict.explanation, conflict.variable);
        assert!(!nogood.literals().is_empty());

        let branch_order: Vec<_> = conflict.explanation.branch_literals().collect();
        let max_level = nogood
            .literals()
            .iter()
            .map(|literal| {
                branch_order
                    .iter()
                    .position(|branch| {
                        branch.variable == literal.variable && branch.value == literal.value
                    })
                    .unwrap_or(0)
            })
            .max()
            .unwrap_or(0);
        let top_level_count = nogood
            .literals()
            .iter()
            .filter(|literal| {
                branch_order
                    .iter()
                    .position(|branch| {
                        branch.variable == literal.variable && branch.value == literal.value
                    })
                    .unwrap_or(0)
                    == max_level
            })
            .count();
        assert_eq!(top_level_count, 1);
    }

    #[test]
    fn analyzes_two_branch_all_different_conflict() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(1, 2));
        let b = engine.new_variable(IntervalDomain::new(1, 2));
        engine.add_propagator(Box::new(propaga_propagators::AllDifferentPropagator::new(
            vec![a, b],
        )));
        engine.trail_mark();
        engine.fix_variable(a, 1).unwrap();
        let _ = engine.fix_variable(b, 1);
        let conflict = engine.last_conflict().expect("conflict");
        let nogood = ConflictAnalyzer::analyze(&conflict.explanation, conflict.variable);
        assert!(!nogood.literals().is_empty());
    }

    #[test]
    fn disjunctive_overlap_nogood_uses_fixed_pair_only() {
        let mut engine = Engine::new();
        let start_a = engine.new_variable(IntervalDomain::new(0, 10));
        let start_b = engine.new_variable(IntervalDomain::new(0, 10));
        engine.trail_mark();
        engine.fix_variable(start_a, 0).unwrap();
        engine.fix_variable(start_b, 0).unwrap();
        engine.add_propagator(Box::new(propaga_propagators::DisjunctivePropagator::new(
            vec![
                propaga_propagators::DisjunctiveTask {
                    start: start_a,
                    duration: 1,
                },
                propaga_propagators::DisjunctiveTask {
                    start: start_b,
                    duration: 1,
                },
            ],
        )));
        let _ = engine.propagate_all();

        let conflict = engine.last_conflict().expect("conflict");
        let nogood = ConflictAnalyzer::analyze(&conflict.explanation, conflict.variable);
        assert_eq!(nogood.literals().len(), 2);
    }

    #[test]
    fn cumulative_overload_nogood_uses_mandatory_tasks_only() {
        let mut engine = Engine::new();
        let start_a = engine.new_variable(IntervalDomain::new(0, 10));
        let end_a = engine.new_variable(IntervalDomain::new(1, 11));
        let start_b = engine.new_variable(IntervalDomain::new(0, 10));
        let end_b = engine.new_variable(IntervalDomain::new(1, 11));
        engine.add_propagator(Box::new(propaga_propagators::CumulativePropagator::new(
            vec![
                propaga_propagators::TaskSpec::new(start_a, 1, end_a),
                propaga_propagators::TaskSpec::new(start_b, 1, end_b),
            ],
            1,
        )));

        engine.trail_mark();
        engine.fix_variable(start_a, 0).unwrap();
        let _ = engine.fix_variable(start_b, 0);

        let conflict = engine.last_conflict().expect("conflict");
        let nogood = ConflictAnalyzer::analyze(&conflict.explanation, conflict.variable);
        assert_eq!(nogood.literals().len(), 2);
        assert!(
            !nogood
                .literals()
                .iter()
                .any(|literal| literal.variable != start_a && literal.variable != start_b)
        );
    }

    #[test]
    fn propagator_only_pruning_falls_back_to_branch_literals() {
        let mut engine = Engine::new();
        let a = engine.new_variable(IntervalDomain::new(1, 3));
        let b = engine.new_variable(IntervalDomain::new(1, 3));
        engine.add_propagator(Box::new(propaga_propagators::LessEqualPropagator::new(
            a, b,
        )));

        engine.trail_mark();
        engine.fix_variable(a, 3).unwrap();
        let _ = engine.fix_variable(b, 1);

        let conflict = engine.last_conflict().expect("conflict");
        let nogood = ConflictAnalyzer::analyze(&conflict.explanation, conflict.variable);
        assert!(!nogood.literals().is_empty());
        assert!(
            nogood
                .literals()
                .iter()
                .any(|literal| literal.variable == b && literal.value == 1)
        );
    }

    #[test]
    fn detects_nogood_violation() {
        let mut engine = Engine::new();
        let var = engine.new_variable(IntervalDomain::new(1, 5));
        let nogood = Nogood::new(vec![NogoodLiteral {
            variable: var,
            value: 3,
        }]);
        let mut store = NogoodStore::new();
        store.learn(nogood);
        assert!(store.is_violated(&[(var, 3)]));
        assert!(!store.is_violated(&[(var, 2)]));
    }
}
