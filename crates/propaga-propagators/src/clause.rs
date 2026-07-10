use propaga_core::{NogoodLiteral, PropagationContext, PropagationStatus, Propagator, VariableId};

/// Propagator for a learned clause (same semantics as a nogood).
#[derive(Clone)]
pub struct ClausePropagator {
    watched: Vec<VariableId>,
    literals: Vec<NogoodLiteral>,
}

impl ClausePropagator {
    /// Creates a propagator for a learned clause.
    #[must_use]
    pub fn new(literals: impl Into<Vec<NogoodLiteral>>) -> Self {
        let literals = literals.into();
        let mut watched = Vec::new();
        for literal in &literals {
            if !watched.contains(&literal.variable) {
                watched.push(literal.variable);
            }
        }
        Self { watched, literals }
    }
}

impl Propagator for ClausePropagator {
    fn watched_variables(&self) -> &[VariableId] {
        &self.watched
    }

    fn priority(&self) -> u32 {
        1
    }

    fn propagate(&mut self, ctx: &mut dyn PropagationContext) -> PropagationStatus {
        crate::nogood::propagate_nogood_literals(&self.literals, ctx)
    }
}
