use crate::error::FlatZincError;

/// Parsed FlatZinc program (subset).
#[derive(Debug, Clone, PartialEq)]
pub struct FlatZincProgram {
    /// Parameter declarations.
    pub params: Vec<ParamDecl>,
    /// Variable declarations.
    pub variables: Vec<VarDecl>,
    /// Posted constraints.
    pub constraints: Vec<Constraint>,
    /// User-defined predicate declarations.
    pub predicates: Vec<PredicateDecl>,
    /// Output directives for solution formatting.
    pub outputs: Vec<OutputDirective>,
    /// Solve directive with optional search annotations.
    pub solve: SolveDirective,
}

/// A FlatZinc parameter declaration.
#[derive(Debug, Clone, PartialEq)]
pub enum ParamDecl {
    /// Scalar integer parameter.
    Int {
        /// Parameter name.
        name: String,
        /// Parameter value.
        value: i32,
    },
    /// Fixed integer array parameter.
    IntArray {
        /// Array name.
        name: String,
        /// Values in index order.
        values: Vec<i32>,
    },
    /// Scalar boolean parameter (0 or 1).
    Bool {
        /// Parameter name.
        name: String,
        /// Parameter value.
        value: i32,
    },
    /// Scalar float parameter.
    Float {
        /// Parameter name.
        name: String,
        /// Parameter value.
        value: f64,
    },
    /// Fixed set parameter.
    Set {
        /// Parameter name.
        name: String,
        /// Contained values.
        values: Vec<i32>,
    },
}

/// A FlatZinc variable declaration.
#[derive(Debug, Clone, PartialEq)]
pub enum VarDecl {
    /// Scalar integer variable with inclusive bounds.
    IntVar {
        /// Variable name.
        name: String,
        /// Lower bound.
        low: i32,
        /// Upper bound.
        high: i32,
    },
    /// Array of integer variables.
    Array {
        /// Array name.
        name: String,
        /// Inclusive lower index.
        index_low: i32,
        /// Inclusive upper index.
        index_high: i32,
        /// Domain lower bound.
        low: i32,
        /// Domain upper bound.
        high: i32,
    },
    /// Scalar boolean variable (modeled as 0..1 integer).
    BoolVar {
        /// Variable name.
        name: String,
        /// Fixed value when declared with `=`.
        fixed: Option<i32>,
    },
    /// Array of boolean variables (modeled as 0..1 integers).
    BoolArray {
        /// Array name.
        name: String,
        /// Inclusive lower index.
        index_low: i32,
        /// Inclusive upper index.
        index_high: i32,
    },
    /// Scalar set variable over an integer universe.
    SetVar {
        /// Variable name.
        name: String,
        /// Universe lower bound.
        low: i32,
        /// Universe upper bound.
        high: i32,
    },
    /// Scalar float variable with inclusive bounds.
    FloatVar {
        /// Variable name.
        name: String,
        /// Domain lower bound.
        low: f64,
        /// Domain upper bound.
        high: f64,
    },
}

/// A FlatZinc constraint call.
#[derive(Debug, Clone, PartialEq)]
pub enum Constraint {
    /// `all_different(...)`
    AllDifferent(Vec<Expr>),
    /// `int_eq(a, b)`
    IntEq(Expr, Expr),
    /// `int_lin_eq(coeffs, vars, rhs)`
    IntLinEq {
        /// Coefficients.
        coeffs: Vec<i32>,
        /// Variables or expressions.
        vars: Vec<Expr>,
        /// Right-hand side.
        rhs: i32,
    },
    /// `int_lin_le(coeffs, vars, rhs)`
    IntLinLe {
        /// Coefficients.
        coeffs: Vec<i32>,
        /// Variables or expressions.
        vars: Vec<Expr>,
        /// Right-hand side.
        rhs: i32,
    },
    /// `int_lin_ge(coeffs, vars, rhs)`
    IntLinGe {
        /// Coefficients.
        coeffs: Vec<i32>,
        /// Variables or expressions.
        vars: Vec<Expr>,
        /// Right-hand side.
        rhs: i32,
    },
    /// `int_lin_le_reif(coeffs, vars, rhs, reif)`
    IntLinLeReif {
        /// Coefficients.
        coeffs: Vec<i32>,
        /// Variables or expressions.
        vars: Vec<Expr>,
        /// Right-hand side.
        rhs: i32,
        /// Reification variable.
        reif: Expr,
    },
    /// `int_lin_ge_reif(coeffs, vars, rhs, reif)`
    IntLinGeReif {
        /// Coefficients.
        coeffs: Vec<i32>,
        /// Variables or expressions.
        vars: Vec<Expr>,
        /// Right-hand side.
        rhs: i32,
        /// Reification variable.
        reif: Expr,
    },
    /// `int_lin_eq_reif(coeffs, vars, rhs, reif)`
    IntLinEqReif {
        /// Coefficients.
        coeffs: Vec<i32>,
        /// Variables or expressions.
        vars: Vec<Expr>,
        /// Right-hand side.
        rhs: i32,
        /// Reification variable.
        reif: Expr,
    },
    /// `int_lin_ne_reif(coeffs, vars, rhs, reif)`
    IntLinNeReif {
        /// Coefficients.
        coeffs: Vec<i32>,
        /// Variables or expressions.
        vars: Vec<Expr>,
        /// Right-hand side.
        rhs: i32,
        /// Reification variable.
        reif: Expr,
    },
    /// `int_ne(a, b)`
    IntNe(Expr, Expr),
    /// `int_le(a, b)`
    IntLe(Expr, Expr),
    /// `int_lt(a, b)`
    IntLt(Expr, Expr),
    /// `int_ge(a, b)`
    IntGe(Expr, Expr),
    /// `int_gt(a, b)`
    IntGt(Expr, Expr),
    /// `int_eq_reif(a, b, reif)`
    IntEqReif(Expr, Expr, Expr),
    /// `int_ne_reif(a, b, reif)`
    IntNeReif(Expr, Expr, Expr),
    /// `int_le_reif(a, b, reif)`
    IntLeReif(Expr, Expr, Expr),
    /// `int_lt_reif(a, b, reif)`
    IntLtReif(Expr, Expr, Expr),
    /// `int_ge_reif(a, b, reif)`
    IntGeReif(Expr, Expr, Expr),
    /// `int_gt_reif(a, b, reif)`
    IntGtReif(Expr, Expr, Expr),
    /// `element(array, index, value)`
    Element {
        /// Array expression.
        array: Expr,
        /// Index expression.
        index: Expr,
        /// Value expression.
        value: Expr,
    },
    /// `cumulative(starts, durations, ends, capacity)` or with heights
    /// `cumulative(starts, durations, ends, heights, capacity)`
    Cumulative {
        /// Start variables.
        starts: Expr,
        /// Duration list or parameter name.
        durations: DurationSpec,
        /// End variables.
        ends: Expr,
        /// Optional height/demand list or parameter name.
        heights: Option<DurationSpec>,
        /// Resource capacity.
        capacity: i32,
    },
    /// `disjunctive(starts, durations)`
    Disjunctive {
        /// Start variables.
        starts: Expr,
        /// Duration list or parameter name.
        durations: DurationSpec,
    },
    /// `global_cardinality(cover, vars)` or `global_cardinality(vars, cover, lbound, ubound)`
    GlobalCardinality {
        /// Decision variables.
        vars: Expr,
        /// Covered values.
        cover: Expr,
        /// Optional per-value lower bounds (parallel to cover).
        lbound: Option<Expr>,
        /// Optional per-value upper bounds (parallel to cover).
        ubound: Option<Expr>,
    },
    /// `table(vars, {tuples})`
    Table {
        /// Variables in the constraint.
        vars: Expr,
        /// Flattened tuple values from `{a, b, c, d, ...}`.
        tuples: Vec<i32>,
    },
    /// `bool_eq(a, b)`
    BoolEq(Expr, Expr),
    /// `bool2int(b, i)`
    Bool2Int(Expr, Expr),
    /// `circuit(successors)`
    Circuit(Expr),
    /// `inverse(forward, backward)`
    Inverse {
        /// Forward array.
        forward: Expr,
        /// Backward array.
        backward: Expr,
    },
    /// `diffn(xs, ys, widths, heights)`
    Diffn {
        /// X coordinates.
        xs: Expr,
        /// Y coordinates.
        ys: Expr,
        /// Widths (inline ints or param array).
        widths: DurationSpec,
        /// Heights (inline ints or param array).
        heights: DurationSpec,
    },
    /// `count(xs, value, total)`
    Count(Expr, Expr, Expr),
    /// `among(n, xs, values)`
    Among(Expr, Expr, Expr),
    /// `at_least(n, xs, value)`
    AtLeast(Expr, Expr, Expr),
    /// `at_most(n, xs, value)`
    AtMost(Expr, Expr, Expr),
    /// `distribute(card, value, base)`
    Distribute(Expr, Expr, Expr),
    /// `nvalue(n, xs)`
    Nvalue(Expr, Expr),
    /// `lex_less(x, y)`
    LexLess(Expr, Expr),
    /// `lex_lesseq(x, y)`
    LexLesseq(Expr, Expr),
    /// `lex_greater(x, y)`
    LexGreater(Expr, Expr),
    /// `lex_greatereq(x, y)`
    LexGreatereq(Expr, Expr),
    /// `increasing(x)`
    Increasing(Expr),
    /// `decreasing(x)`
    Decreasing(Expr),
    /// `sort(x, y)`
    Sort(Expr, Expr),
    /// `float_dom(x, ranges)`
    FloatDom(Expr, Vec<f64>),
    /// `float_in(x, lo, hi)`
    FloatIn(Expr, f64, f64),
    /// `array_float_element(array, index, value)`
    ArrayFloatElement(Expr, Expr, Expr),
    /// `array_var_float_element(array, index, value)`
    ArrayVarFloatElement(Expr, Expr, Expr),
    /// `array_float_maximum(xs, m)`
    ArrayFloatMaximum(Expr, Expr),
    /// `array_float_minimum(xs, m)`
    ArrayFloatMinimum(Expr, Expr),
    /// User-defined predicate call.
    PredicateCall {
        /// Predicate name.
        name: String,
        /// Call arguments.
        args: Vec<Expr>,
    },
    /// `regular(vars, q, s, d, start, accepting)`
    Regular {
        /// Sequence variables.
        vars: Vec<Expr>,
        /// Alphabet size.
        num_symbols: i32,
        /// Number of states.
        num_states: i32,
        /// Transition matrix parameter name.
        transitions: String,
        /// Start state.
        start: i32,
        /// Accepting state(s).
        accepting: Vec<i32>,
    },
    /// `set_card(set, card)` — `card` may be an integer literal or variable
    SetCard(Expr, Expr),
    /// `set_subset(subset, superset)`
    SetSubset(Expr, Expr),
    /// `set_eq(left, right)`
    SetEq(Expr, Expr),
    /// `set_in(value, set)`
    SetIn(Expr, Expr),
    /// `set_superset(superset, subset)`
    SetSuperset(Expr, Expr),
    /// `set_le(left, right)`
    SetLe(Expr, Expr),
    /// `set_ne(left, right)`
    SetNe(Expr, Expr),
    /// `set_lt(left, right)`
    SetLt(Expr, Expr),
    /// `set_diff(left, right, result)`
    SetDiff(Expr, Expr, Expr),
    /// `set_symdiff(left, right, result)`
    SetSymdiff(Expr, Expr, Expr),
    /// `set_eq_reif(left, right, reif)`
    SetEqReif(Expr, Expr, Expr),
    /// `set_ne_reif(left, right, reif)`
    SetNeReif(Expr, Expr, Expr),
    /// `set_in_reif(value, set, reif)`
    SetInReif(Expr, Expr, Expr),
    /// `set_subset_reif(subset, superset, reif)`
    SetSubsetReif(Expr, Expr, Expr),
    /// `set_superset_reif(superset, subset, reif)`
    SetSupersetReif(Expr, Expr, Expr),
    /// `set_le_reif(left, right, reif)`
    SetLeReif(Expr, Expr, Expr),
    /// `set_lt_reif(left, right, reif)`
    SetLtReif(Expr, Expr, Expr),
    /// `array_var_set_element` / `array_var_set_element_nonshifted`
    /// (`one_based` is true for the shifted/standard form).
    ArrayVarSetElement {
        array: Expr,
        index: Expr,
        value: Expr,
        one_based: bool,
    },
    /// `float_le(left, right)`
    FloatLe(Expr, Expr),
    /// `float_eq(left, right)`
    FloatEq(Expr, Expr),
    /// `set_union(x, y, r)`
    SetUnion(Expr, Expr, Expr),
    /// `set_intersect(x, y, r)`
    SetIntersect(Expr, Expr, Expr),
    /// `float_times(a, b, c)`
    FloatTimes(Expr, Expr, Expr),
    /// `float_plus(a, b, c)`
    FloatPlus(Expr, Expr, Expr),
    /// `float_abs(a, b)`
    FloatAbs(Expr, Expr),
    /// `float_div(a, b, c)`
    FloatDiv(Expr, Expr, Expr),
    /// `float_lt(a, b)`
    FloatLt(Expr, Expr),
    /// `float_ne(a, b)`
    FloatNe(Expr, Expr),
    /// `float_max(a, b, c)`
    FloatMax(Expr, Expr, Expr),
    /// `float_min(a, b, c)`
    FloatMin(Expr, Expr, Expr),
    /// `int2float(i, f)`
    Int2Float(Expr, Expr),
    /// `float_sqrt(a, b)`
    FloatSqrt(Expr, Expr),
    /// `float_sin(a, b)`
    FloatSin(Expr, Expr),
    /// `float_cos(a, b)`
    FloatCos(Expr, Expr),
    /// `float_ln(a, b)`
    FloatLn(Expr, Expr),
    /// `float_log2(a, b)`
    FloatLog2(Expr, Expr),
    /// `float_exp(a, b)`
    FloatExp(Expr, Expr),
    /// `float_ceil(a, b)`
    FloatCeil(Expr, Expr),
    /// `float_floor(a, b)`
    FloatFloor(Expr, Expr),
    /// `float_round(a, b)`
    FloatRound(Expr, Expr),
    /// `float_lin_eq(coeffs, vars, rhs)`
    FloatLinEq {
        coeffs: Vec<f64>,
        vars: Vec<Expr>,
        rhs: f64,
    },
    /// `float_lin_ne(coeffs, vars, rhs)`
    FloatLinNe {
        coeffs: Vec<f64>,
        vars: Vec<Expr>,
        rhs: f64,
    },
    /// `float_lin_le(coeffs, vars, rhs)`
    FloatLinLe {
        coeffs: Vec<f64>,
        vars: Vec<Expr>,
        rhs: f64,
    },
    /// `float_lin_ge(coeffs, vars, rhs)`
    FloatLinGe {
        coeffs: Vec<f64>,
        vars: Vec<Expr>,
        rhs: f64,
    },
    /// `float_lin_le_reif(coeffs, vars, rhs, reif)`
    FloatLinLeReif {
        coeffs: Vec<f64>,
        vars: Vec<Expr>,
        rhs: f64,
        reif: Expr,
    },
    /// `float_lin_ge_reif(coeffs, vars, rhs, reif)`
    FloatLinGeReif {
        coeffs: Vec<f64>,
        vars: Vec<Expr>,
        rhs: f64,
        reif: Expr,
    },
    /// `float_lin_eq_reif(coeffs, vars, rhs, reif)`
    FloatLinEqReif {
        coeffs: Vec<f64>,
        vars: Vec<Expr>,
        rhs: f64,
        reif: Expr,
    },
    /// `float_eq_reif(a, b, reif)`
    FloatEqReif(Expr, Expr, Expr),
    /// `float_ne_reif(a, b, reif)`
    FloatNeReif(Expr, Expr, Expr),
    /// `float_le_reif(a, b, reif)`
    FloatLeReif(Expr, Expr, Expr),
    /// `float_lt_reif(a, b, reif)`
    FloatLtReif(Expr, Expr, Expr),
    /// `int_abs(a, b)` — b = |a|
    IntAbs(Expr, Expr),
    /// `int_times(a, b, c)`
    IntTimes(Expr, Expr, Expr),
    /// `int_div(a, b, c)`
    IntDiv(Expr, Expr, Expr),
    /// `int_mod(a, b, c)`
    IntMod(Expr, Expr, Expr),
    /// `bool_not(a, b)`
    BoolNot(Expr, Expr),
    /// `bool_and(a, b, c)`
    BoolAnd(Expr, Expr, Expr),
    /// `bool_or(a, b, c)`
    BoolOr(Expr, Expr, Expr),
    /// `bool_xor(a, b, c)`
    BoolXor(Expr, Expr, Expr),
    /// `bool_clause(literals)`
    BoolClause(Expr),
    /// `bool_clause_reif(literals, reif)`
    BoolClauseReif(Expr, Expr),
    /// `bool_eq_reif(a, b, reif)`
    BoolEqReif(Expr, Expr, Expr),
    /// `bool_le(a, b)`
    BoolLe(Expr, Expr),
    /// `bool_le_reif(a, b, reif)`
    BoolLeReif(Expr, Expr, Expr),
    /// `bool_lt(a, b)`
    BoolLt(Expr, Expr),
    /// `bool_lt_reif(a, b, reif)`
    BoolLtReif(Expr, Expr, Expr),
    /// `bool_lin_eq(coeffs, vars, rhs)`
    BoolLinEq {
        coeffs: Vec<i32>,
        vars: Vec<Expr>,
        rhs: i32,
    },
    /// `bool_lin_le(coeffs, vars, rhs)`
    BoolLinLe {
        coeffs: Vec<i32>,
        vars: Vec<Expr>,
        rhs: i32,
    },
    /// `array_bool_and(xs, c)`
    ArrayBoolAnd(Expr, Expr),
    /// `array_bool_xor(xs, c)`
    ArrayBoolXor(Expr, Expr),
    /// `array_bool_element(array, index, value)`
    ArrayBoolElement(Expr, Expr, Expr),
    /// `array_var_bool_element(array, index, value)`
    ArrayVarBoolElement(Expr, Expr, Expr),
    /// `int_min(a, b, c)`
    IntMin(Expr, Expr, Expr),
    /// `int_max(a, b, c)`
    IntMax(Expr, Expr, Expr),
    /// `int_pow(base, exp, result)`
    IntPow(Expr, Expr, Expr),
    /// `int_pow_fixed(base, exp, result)`
    IntPowFixed(Expr, i32, Expr),
    /// `array_int_element(array, index, value)`
    ArrayIntElement(Expr, Expr, Expr),
    /// `array_var_int_element(array, index, value)`
    ArrayVarIntElement(Expr, Expr, Expr),
    /// `array_int_maximum(xs, m)`
    ArrayIntMaximum(Expr, Expr),
    /// `array_int_minimum(xs, m)`
    ArrayIntMinimum(Expr, Expr),
    /// `int_plus(a, b, c)`
    IntPlus(Expr, Expr, Expr),
    /// `int_lin_ne(coeffs, vars, rhs)`
    IntLinNe {
        /// Coefficients.
        coeffs: Vec<i32>,
        /// Variables or expressions.
        vars: Vec<Expr>,
        /// Right-hand side.
        rhs: i32,
    },
    /// `automaton(vars, symbols, states, transitions, start, accepting)`
    Automaton {
        /// Sequence variables.
        vars: Vec<Expr>,
        /// Alphabet size.
        num_symbols: i32,
        /// Number of states.
        num_states: i32,
        /// Transition matrix parameter name.
        transitions: String,
        /// Start state.
        start: i32,
        /// Accepting states.
        accepting: Vec<i32>,
    },
}

/// A parsed user-defined predicate with one or more constraint bodies.
#[derive(Debug, Clone, PartialEq)]
pub struct PredicateDecl {
    /// Predicate name.
    pub name: String,
    /// Formal parameter names in order.
    pub params: Vec<String>,
    /// Inlined constraint bodies.
    pub body: Vec<Constraint>,
}

/// Duration array in a cumulative constraint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DurationSpec {
    /// Inline integer list.
    Inline(Vec<i32>),
    /// Name of an `array of int` parameter.
    Name(String),
}

/// FlatZinc expression subset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    /// Identifier or indexed array access.
    Name(String),
    /// Integer literal.
    Int(i32),
    /// Indexed access `name[i]`.
    Index {
        /// Array name.
        name: String,
        /// Index expression.
        index: Box<Expr>,
    },
    /// Inline list `[a, b, c]`.
    List(Vec<Expr>),
}

/// A parsed FlatZinc output directive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputDirective {
    /// Segments to render when printing a solution.
    pub segments: Vec<OutputSegment>,
}

/// One segment of formatted output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputSegment {
    /// Literal text.
    Text(String),
    /// Variable reference by name (scalar or indexed).
    Variable(String),
}

/// Solve directive with optional FlatZinc search annotations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SolveDirective {
    /// Parsed search annotations before the goal.
    pub annotations: SearchAnnotations,
    /// Optimization or satisfaction goal.
    pub goal: SolveGoal,
}

/// Search annotations attached to a `solve` directive.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SearchAnnotations {
    /// `int_search(...)` annotation, when present.
    pub int_search: Option<IntSearchAnnotation>,
    /// `bool_search(...)` annotation, when present.
    pub bool_search: Option<IntSearchAnnotation>,
    /// `float_search(...)` annotation, when present.
    pub float_search: Option<IntSearchAnnotation>,
    /// `set_search(...)` annotation, when present.
    pub set_search: Option<IntSearchAnnotation>,
    /// `seq_search([...])` ordered list of nested typed searches.
    pub seq_search: Option<Vec<IntSearchAnnotation>>,
    /// `restart_*` annotation, when present.
    pub restart: Option<RestartAnnotation>,
    /// `pareto([...])` annotation listing objective variables.
    pub pareto: Option<Vec<Expr>>,
}

/// Parsed `int_search(vars, var_choice, value_choice, complete)` annotation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntSearchAnnotation {
    /// Decision variables in search order.
    pub vars: Vec<Expr>,
    /// FlatZinc variable selection method (e.g. `first_fail`).
    pub var_choice: String,
    /// FlatZinc value selection method (e.g. `indomain_min`).
    pub value_choice: String,
    /// Whether search is complete (`complete` vs `incomplete`).
    pub complete: bool,
}

/// Parsed restart annotation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestartAnnotation {
    /// Restart policy kind.
    pub kind: RestartKind,
}

/// Supported FlatZinc restart policies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestartKind {
    /// `restart_constant(scale)`.
    Constant {
        /// Fixed node limit before each restart.
        scale: u64,
    },
    /// `restart_geometric(base, scale)`.
    Geometric {
        /// Geometric multiplier, kept textual to avoid lossy AST equality.
        base: String,
        /// Initial node limit multiplier.
        scale: u64,
    },
    /// `restart_luby(base)` or `restart_luby(base, scale)`.
    Luby {
        /// Luby base multiplier.
        base: u64,
    },
    /// `restart_none`.
    None,
    /// `restart_linear(scale)`.
    Linear {
        /// Node limit multiplier per restart.
        scale: u64,
    },
    /// `restart_on_solution()`.
    OnSolution,
}

/// Solve directive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SolveGoal {
    /// `solve satisfy`
    Satisfy,
    /// `solve minimize expr` or `solve minimize x, y`
    Minimize(Vec<Expr>),
    /// `solve maximize expr` or `solve maximize x, y`
    Maximize(Vec<Expr>),
}

/// Parses a FlatZinc subset program from source text.
pub fn parse(source: &str) -> Result<FlatZincProgram, FlatZincError> {
    let stripped = strip_comments(source);
    let tokens = tokenize(&stripped)?;
    Parser::new(tokens).parse_program()
}

fn strip_comments(source: &str) -> String {
    source
        .lines()
        .map(|line| {
            if let Some(idx) = line.find('%') {
                &line[..idx]
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Ident(String),
    Int(i32),
    Float(String),
    String(String),
    Symbol(String),
}

fn tokenize(source: &str) -> Result<Vec<Token>, FlatZincError> {
    let mut tokens = Vec::new();
    let mut chars = source.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            ' ' | '\t' | '\n' | '\r' => {}
            'a'..='z' | 'A'..='Z' | '_' => {
                let mut ident = String::from(ch);
                while matches!(chars.peek(), Some('a'..='z' | 'A'..='Z' | '0'..='9' | '_')) {
                    ident.push(chars.next().expect("peeked"));
                }
                tokens.push(Token::Ident(ident));
            }
            '0'..='9' | '-' => {
                let mut number = String::from(ch);
                while matches!(chars.peek(), Some('0'..='9')) {
                    number.push(chars.next().expect("peeked"));
                }
                let mut is_float = false;
                if matches!(chars.peek(), Some('.')) {
                    let mut lookahead = chars.clone();
                    lookahead.next();
                    if matches!(lookahead.peek(), Some('0'..='9')) {
                        is_float = true;
                        number.push(chars.next().expect("peeked"));
                        while matches!(chars.peek(), Some('0'..='9')) {
                            number.push(chars.next().expect("peeked"));
                        }
                    }
                }
                if is_float {
                    tokens.push(Token::Float(number));
                    continue;
                }
                let value = number
                    .parse::<i32>()
                    .map_err(|_| FlatZincError::InvalidInteger(number))?;
                tokens.push(Token::Int(value));
            }
            '"' => {
                let mut text = String::new();
                for next in chars.by_ref() {
                    if next == '"' {
                        break;
                    }
                    text.push(next);
                }
                tokens.push(Token::String(text));
            }
            '.' if matches!(chars.peek(), Some('.')) => {
                chars.next();
                tokens.push(Token::Symbol("..".to_string()));
            }
            ':' if matches!(chars.peek(), Some(':')) => {
                chars.next();
                tokens.push(Token::Symbol("::".to_string()));
            }
            other => tokens.push(Token::Symbol(other.to_string())),
        }
    }

    Ok(tokens)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn parse_program(&mut self) -> Result<FlatZincProgram, FlatZincError> {
        let mut params = Vec::new();
        let mut variables = Vec::new();
        let mut constraints = Vec::new();
        let mut predicates = Vec::new();
        let mut outputs = Vec::new();
        let mut solve = None;

        while !self.is_eof() {
            if self.peek_is_ident("var") {
                variables.push(self.parse_var_decl()?);
            } else if self.peek_is_ident("array") {
                if self.peek_is_int_array_param() {
                    params.push(self.parse_int_array_param()?);
                } else {
                    variables.push(self.parse_array_decl()?);
                }
            } else if self.peek_is_ident("int") {
                params.push(self.parse_param_decl()?);
            } else if self.peek_is_ident("bool") {
                params.push(self.parse_bool_param()?);
            } else if self.peek_is_ident("float") {
                params.push(self.parse_float_param()?);
            } else if self.peek_is_ident("set") {
                params.push(self.parse_set_param()?);
            } else if self.peek_is_ident("constraint") {
                constraints.push(self.parse_constraint()?);
            } else if self.peek_is_ident("solve") {
                if solve.is_some() {
                    return Err(FlatZincError::Unsupported(
                        "multiple solve directives".to_string(),
                    ));
                }
                solve = Some(self.parse_solve()?);
            } else if self.peek_is_ident("output") {
                outputs.push(self.parse_output()?);
            } else if self.peek_is_ident("predicate") {
                predicates.push(self.parse_predicate_decl()?);
            } else if self.peek_is_ident("function")
                || self.peek_is_ident("test")
                || self.peek_is_ident("annotation")
            {
                self.skip_until_semicolon_or_eof();
            } else {
                let found = match self.peek() {
                    Some(Token::Ident(name)) => name.clone(),
                    Some(other) => format!("{other:?}"),
                    None => return Err(FlatZincError::UnexpectedEof),
                };
                return Err(FlatZincError::Unsupported(format!(
                    "unsupported top-level statement starting with `{found}`"
                )));
            }
            self.consume_optional_semicolon();
        }

        let solve = solve.ok_or(FlatZincError::MissingSolve)?;
        Ok(FlatZincProgram {
            params,
            variables,
            constraints,
            predicates,
            outputs,
            solve,
        })
    }

    fn parse_param_decl(&mut self) -> Result<ParamDecl, FlatZincError> {
        self.expect_ident("int")?;
        self.expect_symbol(":")?;
        let name = self.expect_ident_token()?;
        self.expect_symbol("=")?;
        let value = self.expect_int()?;
        Ok(ParamDecl::Int { name, value })
    }

    fn peek_is_int_array_param(&self) -> bool {
        if !self.peek_is_ident("array") {
            return false;
        }
        let mut pos = self.pos + 1;
        while pos < self.tokens.len() {
            match &self.tokens[pos] {
                Token::Ident(name) if name == "of" => {
                    return matches!(
                        self.tokens.get(pos + 1),
                        Some(Token::Ident(name)) if name == "int"
                    );
                }
                Token::Symbol(symbol) if symbol == ";" => return false,
                _ => pos += 1,
            }
        }
        false
    }

    fn parse_int_array_param(&mut self) -> Result<ParamDecl, FlatZincError> {
        self.expect_ident("array")?;
        self.expect_symbol("[")?;
        self.expect_int()?;
        self.expect_symbol("..")?;
        self.expect_int()?;
        self.expect_symbol("]")?;
        self.expect_ident("of")?;
        self.expect_ident("int")?;
        self.expect_symbol(":")?;
        let name = self.expect_ident_token()?;
        self.expect_symbol("=")?;
        self.expect_symbol("[")?;
        let values = self.parse_int_list()?;
        self.expect_symbol("]")?;
        Ok(ParamDecl::IntArray { name, values })
    }

    fn parse_bool_param(&mut self) -> Result<ParamDecl, FlatZincError> {
        self.expect_ident("bool")?;
        self.expect_symbol(":")?;
        let name = self.expect_ident_token()?;
        self.expect_symbol("=")?;
        let value = if self.peek_is_ident("true") {
            self.expect_ident("true")?;
            1
        } else {
            self.expect_ident("false")?;
            0
        };
        Ok(ParamDecl::Bool { name, value })
    }

    fn parse_float_param(&mut self) -> Result<ParamDecl, FlatZincError> {
        self.expect_ident("float")?;
        self.expect_symbol(":")?;
        let name = self.expect_ident_token()?;
        self.expect_symbol("=")?;
        let value = self
            .expect_float_text()?
            .parse::<f64>()
            .map_err(|_| FlatZincError::Unsupported("invalid float literal".into()))?;
        Ok(ParamDecl::Float { name, value })
    }

    fn parse_set_param(&mut self) -> Result<ParamDecl, FlatZincError> {
        self.expect_ident("set")?;
        self.expect_ident("of")?;
        let _ = self.parse_domain()?;
        self.expect_symbol(":")?;
        let name = self.expect_ident_token()?;
        self.expect_symbol("=")?;
        let values = self.parse_tuple_set()?;
        Ok(ParamDecl::Set { name, values })
    }

    fn skip_until_semicolon_or_eof(&mut self) {
        while !self.is_eof() && !self.peek_is_symbol(";") {
            self.pos += 1;
        }
        self.consume_optional_semicolon();
    }

    fn parse_var_decl(&mut self) -> Result<VarDecl, FlatZincError> {
        self.expect_ident("var")?;
        if self.peek_is_ident("array") {
            return self.parse_array_decl_body();
        }
        if self.peek_is_ident("int") {
            self.expect_ident("int")?;
            self.expect_symbol(":")?;
            let name = self.expect_ident_token()?;
            let (low, high) = if self.peek_is_symbol("=") {
                self.expect_symbol("=")?;
                let value = self.expect_int()?;
                (value, value)
            } else {
                self.parse_domain()?
            };
            return Ok(VarDecl::IntVar { name, low, high });
        }
        if self.peek_is_ident("bool") {
            self.expect_ident("bool")?;
            self.expect_symbol(":")?;
            let name = self.expect_ident_token()?;
            let fixed = if self.peek_is_symbol("=") {
                self.expect_symbol("=")?;
                Some(self.expect_int()?)
            } else {
                None
            };
            return Ok(VarDecl::BoolVar { name, fixed });
        }
        if self.peek_is_ident("set") {
            self.expect_ident("set")?;
            self.expect_ident("of")?;
            let (low, high) = self.parse_domain()?;
            self.expect_symbol(":")?;
            let name = self.expect_ident_token()?;
            return Ok(VarDecl::SetVar { name, low, high });
        }
        if self.peek_is_ident("float") {
            self.expect_ident("float")?;
            self.expect_symbol(":")?;
            let name = self.expect_ident_token()?;
            let (low, high) = if self.peek_is_symbol("=") {
                self.expect_symbol("=")?;
                self.parse_float_domain()?
            } else {
                (f64::NEG_INFINITY, f64::INFINITY)
            };
            return Ok(VarDecl::FloatVar { name, low, high });
        }
        if matches!(self.peek(), Some(Token::Float(_))) {
            let (low, high) = self.parse_float_domain()?;
            self.expect_symbol(":")?;
            let name = self.expect_ident_token()?;
            return Ok(VarDecl::FloatVar { name, low, high });
        }
        let (low, high) = self.parse_domain()?;
        self.expect_symbol(":")?;
        let name = self.expect_ident_token()?;
        Ok(VarDecl::IntVar { name, low, high })
    }

    fn parse_array_decl(&mut self) -> Result<VarDecl, FlatZincError> {
        self.parse_array_decl_body()
    }

    fn parse_array_decl_body(&mut self) -> Result<VarDecl, FlatZincError> {
        self.expect_ident("array")?;
        self.expect_symbol("[")?;
        let index_low = self.expect_int()?;
        self.expect_symbol("..")?;
        let index_high = self.expect_int()?;
        self.expect_symbol("]")?;
        self.expect_ident("of")?;
        self.expect_ident("var")?;
        if self.peek_is_ident("bool") {
            self.expect_ident("bool")?;
            self.expect_symbol(":")?;
            let name = self.expect_ident_token()?;
            return Ok(VarDecl::BoolArray {
                name,
                index_low,
                index_high,
            });
        }
        let (low, high) = self.parse_domain()?;
        self.expect_symbol(":")?;
        let name = self.expect_ident_token()?;
        Ok(VarDecl::Array {
            name,
            index_low,
            index_high,
            low,
            high,
        })
    }

    fn parse_domain(&mut self) -> Result<(i32, i32), FlatZincError> {
        if self.peek_is_ident("int") {
            self.expect_ident("int")?;
            return Ok((i32::MIN / 4, i32::MAX / 4));
        }
        let low = self.expect_int()?;
        self.expect_symbol("..")?;
        let high = self.expect_int()?;
        Ok((low, high))
    }

    fn parse_float_domain(&mut self) -> Result<(f64, f64), FlatZincError> {
        let low = self
            .expect_float_text()?
            .parse::<f64>()
            .map_err(|_| FlatZincError::Unsupported("invalid float literal".into()))?;
        self.expect_symbol("..")?;
        let high = self
            .expect_float_text()?
            .parse::<f64>()
            .map_err(|_| FlatZincError::Unsupported("invalid float literal".into()))?;
        Ok((low, high))
    }

    fn parse_constraint(&mut self) -> Result<Constraint, FlatZincError> {
        self.expect_ident("constraint")?;
        let name = self.expect_ident_token()?;
        self.expect_symbol("(")?;
        let constraint = self.parse_constraint_by_name(&name)?;
        self.expect_symbol(")")?;
        Ok(constraint)
    }

    fn parse_constraint_by_name(&mut self, name: &str) -> Result<Constraint, FlatZincError> {
        let constraint = match name {
            "all_different" => {
                let expr = self.parse_expr()?;
                let args = match expr {
                    Expr::List(items) => items,
                    other => vec![other],
                };
                Constraint::AllDifferent(args)
            }
            "int_eq" => {
                let left = self.parse_expr()?;
                self.expect_symbol(",")?;
                let right = self.parse_expr()?;
                Constraint::IntEq(left, right)
            }
            "int_lin_eq" => {
                self.expect_symbol("[")?;
                let coeffs = self.parse_int_list()?;
                self.expect_symbol("]")?;
                self.expect_symbol(",")?;
                self.expect_symbol("[")?;
                let vars = self.parse_expr_list()?;
                self.expect_symbol("]")?;
                self.expect_symbol(",")?;
                let rhs = self.expect_int()?;
                Constraint::IntLinEq { coeffs, vars, rhs }
            }
            "int_lin_le" => {
                self.expect_symbol("[")?;
                let coeffs = self.parse_int_list()?;
                self.expect_symbol("]")?;
                self.expect_symbol(",")?;
                self.expect_symbol("[")?;
                let vars = self.parse_expr_list()?;
                self.expect_symbol("]")?;
                self.expect_symbol(",")?;
                let rhs = self.expect_int()?;
                Constraint::IntLinLe { coeffs, vars, rhs }
            }
            "int_lin_ge" => {
                self.expect_symbol("[")?;
                let coeffs = self.parse_int_list()?;
                self.expect_symbol("]")?;
                self.expect_symbol(",")?;
                self.expect_symbol("[")?;
                let vars = self.parse_expr_list()?;
                self.expect_symbol("]")?;
                self.expect_symbol(",")?;
                let rhs = self.expect_int()?;
                Constraint::IntLinGe { coeffs, vars, rhs }
            }
            "int_lin_le_reif" => {
                self.expect_symbol("[")?;
                let coeffs = self.parse_int_list()?;
                self.expect_symbol("]")?;
                self.expect_symbol(",")?;
                self.expect_symbol("[")?;
                let vars = self.parse_expr_list()?;
                self.expect_symbol("]")?;
                self.expect_symbol(",")?;
                let rhs = self.expect_int()?;
                self.expect_symbol(",")?;
                let reif = self.parse_expr()?;
                Constraint::IntLinLeReif {
                    coeffs,
                    vars,
                    rhs,
                    reif,
                }
            }
            "int_lin_ge_reif" => {
                self.expect_symbol("[")?;
                let coeffs = self.parse_int_list()?;
                self.expect_symbol("]")?;
                self.expect_symbol(",")?;
                self.expect_symbol("[")?;
                let vars = self.parse_expr_list()?;
                self.expect_symbol("]")?;
                self.expect_symbol(",")?;
                let rhs = self.expect_int()?;
                self.expect_symbol(",")?;
                let reif = self.parse_expr()?;
                Constraint::IntLinGeReif {
                    coeffs,
                    vars,
                    rhs,
                    reif,
                }
            }
            "int_lin_eq_reif" => {
                self.expect_symbol("[")?;
                let coeffs = self.parse_int_list()?;
                self.expect_symbol("]")?;
                self.expect_symbol(",")?;
                self.expect_symbol("[")?;
                let vars = self.parse_expr_list()?;
                self.expect_symbol("]")?;
                self.expect_symbol(",")?;
                let rhs = self.expect_int()?;
                self.expect_symbol(",")?;
                let reif = self.parse_expr()?;
                Constraint::IntLinEqReif {
                    coeffs,
                    vars,
                    rhs,
                    reif,
                }
            }
            "int_lin_ne_reif" => {
                self.expect_symbol("[")?;
                let coeffs = self.parse_int_list()?;
                self.expect_symbol("]")?;
                self.expect_symbol(",")?;
                self.expect_symbol("[")?;
                let vars = self.parse_expr_list()?;
                self.expect_symbol("]")?;
                self.expect_symbol(",")?;
                let rhs = self.expect_int()?;
                self.expect_symbol(",")?;
                let reif = self.parse_expr()?;
                Constraint::IntLinNeReif {
                    coeffs,
                    vars,
                    rhs,
                    reif,
                }
            }
            "int_ne" => {
                let left = self.parse_expr()?;
                self.expect_symbol(",")?;
                let right = self.parse_expr()?;
                Constraint::IntNe(left, right)
            }
            "int_le" => {
                let left = self.parse_expr()?;
                self.expect_symbol(",")?;
                let right = self.parse_expr()?;
                Constraint::IntLe(left, right)
            }
            "int_lt" => {
                let left = self.parse_expr()?;
                self.expect_symbol(",")?;
                let right = self.parse_expr()?;
                Constraint::IntLt(left, right)
            }
            "int_ge" => {
                let left = self.parse_expr()?;
                self.expect_symbol(",")?;
                let right = self.parse_expr()?;
                Constraint::IntGe(left, right)
            }
            "int_gt" => {
                let left = self.parse_expr()?;
                self.expect_symbol(",")?;
                let right = self.parse_expr()?;
                Constraint::IntGt(left, right)
            }
            "int_eq_reif" => {
                let left = self.parse_expr()?;
                self.expect_symbol(",")?;
                let right = self.parse_expr()?;
                self.expect_symbol(",")?;
                let reif = self.parse_expr()?;
                Constraint::IntEqReif(left, right, reif)
            }
            "int_ne_reif" => {
                let left = self.parse_expr()?;
                self.expect_symbol(",")?;
                let right = self.parse_expr()?;
                self.expect_symbol(",")?;
                let reif = self.parse_expr()?;
                Constraint::IntNeReif(left, right, reif)
            }
            "int_le_reif" => {
                let left = self.parse_expr()?;
                self.expect_symbol(",")?;
                let right = self.parse_expr()?;
                self.expect_symbol(",")?;
                let reif = self.parse_expr()?;
                Constraint::IntLeReif(left, right, reif)
            }
            "int_lt_reif" => {
                let left = self.parse_expr()?;
                self.expect_symbol(",")?;
                let right = self.parse_expr()?;
                self.expect_symbol(",")?;
                let reif = self.parse_expr()?;
                Constraint::IntLtReif(left, right, reif)
            }
            "int_ge_reif" => {
                let left = self.parse_expr()?;
                self.expect_symbol(",")?;
                let right = self.parse_expr()?;
                self.expect_symbol(",")?;
                let reif = self.parse_expr()?;
                Constraint::IntGeReif(left, right, reif)
            }
            "int_gt_reif" => {
                let left = self.parse_expr()?;
                self.expect_symbol(",")?;
                let right = self.parse_expr()?;
                self.expect_symbol(",")?;
                let reif = self.parse_expr()?;
                Constraint::IntGtReif(left, right, reif)
            }
            "element" => {
                let array = self.parse_expr()?;
                self.expect_symbol(",")?;
                let index = self.parse_expr()?;
                self.expect_symbol(",")?;
                let value = self.parse_expr()?;
                Constraint::Element {
                    array,
                    index,
                    value,
                }
            }
            "cumulative" => {
                let starts = self.parse_expr()?;
                self.expect_symbol(",")?;
                let durations = self.parse_duration_spec()?;
                self.expect_symbol(",")?;
                let ends = self.parse_expr()?;
                self.expect_symbol(",")?;
                let (heights, capacity) =
                    if self.peek_is_symbol("[") || matches!(self.peek(), Some(Token::Ident(_))) {
                        let heights = self.parse_duration_spec()?;
                        self.expect_symbol(",")?;
                        let capacity = self.expect_int()?;
                        (Some(heights), capacity)
                    } else {
                        let capacity = self.expect_int()?;
                        (None, capacity)
                    };
                Constraint::Cumulative {
                    starts,
                    durations,
                    ends,
                    heights,
                    capacity,
                }
            }
            "disjunctive" => {
                let starts = self.parse_expr()?;
                self.expect_symbol(",")?;
                let durations = self.parse_duration_spec()?;
                Constraint::Disjunctive { starts, durations }
            }
            "count" => {
                let xs = self.parse_expr()?;
                self.expect_symbol(",")?;
                let value = self.parse_expr()?;
                self.expect_symbol(",")?;
                let total = self.parse_expr()?;
                Constraint::Count(xs, value, total)
            }
            "among" => {
                let n = self.parse_expr()?;
                self.expect_symbol(",")?;
                let xs = self.parse_expr()?;
                self.expect_symbol(",")?;
                let values = self.parse_expr()?;
                Constraint::Among(n, xs, values)
            }
            "at_least" => {
                let n = self.parse_expr()?;
                self.expect_symbol(",")?;
                let xs = self.parse_expr()?;
                self.expect_symbol(",")?;
                let value = self.parse_expr()?;
                Constraint::AtLeast(n, xs, value)
            }
            "at_most" => {
                let n = self.parse_expr()?;
                self.expect_symbol(",")?;
                let xs = self.parse_expr()?;
                self.expect_symbol(",")?;
                let value = self.parse_expr()?;
                Constraint::AtMost(n, xs, value)
            }
            "distribute" => {
                let card = self.parse_expr()?;
                self.expect_symbol(",")?;
                let value = self.parse_expr()?;
                self.expect_symbol(",")?;
                let base = self.parse_expr()?;
                Constraint::Distribute(card, value, base)
            }
            "nvalue" => {
                let n = self.parse_expr()?;
                self.expect_symbol(",")?;
                let xs = self.parse_expr()?;
                Constraint::Nvalue(n, xs)
            }
            "lex_less" => {
                let left = self.parse_expr()?;
                self.expect_symbol(",")?;
                let right = self.parse_expr()?;
                Constraint::LexLess(left, right)
            }
            "lex_lesseq" => {
                let left = self.parse_expr()?;
                self.expect_symbol(",")?;
                let right = self.parse_expr()?;
                Constraint::LexLesseq(left, right)
            }
            "lex_greater" => {
                let left = self.parse_expr()?;
                self.expect_symbol(",")?;
                let right = self.parse_expr()?;
                Constraint::LexGreater(left, right)
            }
            "lex_greatereq" => {
                let left = self.parse_expr()?;
                self.expect_symbol(",")?;
                let right = self.parse_expr()?;
                Constraint::LexGreatereq(left, right)
            }
            "increasing" => {
                let xs = self.parse_expr()?;
                Constraint::Increasing(xs)
            }
            "decreasing" => {
                let xs = self.parse_expr()?;
                Constraint::Decreasing(xs)
            }
            "sort" => {
                let x = self.parse_expr()?;
                self.expect_symbol(",")?;
                let y = self.parse_expr()?;
                Constraint::Sort(x, y)
            }
            "float_dom" => {
                let x = self.parse_expr()?;
                self.expect_symbol(",")?;
                self.expect_symbol("[")?;
                let ranges = self.parse_float_list()?;
                self.expect_symbol("]")?;
                Constraint::FloatDom(x, ranges)
            }
            "float_in" => {
                let x = self.parse_expr()?;
                self.expect_symbol(",")?;
                let lo = self.expect_float()?;
                self.expect_symbol(",")?;
                let hi = self.expect_float()?;
                Constraint::FloatIn(x, lo, hi)
            }
            "array_float_element" => {
                let array = self.parse_expr()?;
                self.expect_symbol(",")?;
                let index = self.parse_expr()?;
                self.expect_symbol(",")?;
                let value = self.parse_expr()?;
                Constraint::ArrayFloatElement(array, index, value)
            }
            "array_var_float_element" => {
                let array = self.parse_expr()?;
                self.expect_symbol(",")?;
                let index = self.parse_expr()?;
                self.expect_symbol(",")?;
                let value = self.parse_expr()?;
                Constraint::ArrayVarFloatElement(array, index, value)
            }
            "array_float_maximum" => {
                let xs = self.parse_expr()?;
                self.expect_symbol(",")?;
                let m = self.parse_expr()?;
                Constraint::ArrayFloatMaximum(xs, m)
            }
            "array_float_minimum" => {
                let xs = self.parse_expr()?;
                self.expect_symbol(",")?;
                let m = self.parse_expr()?;
                Constraint::ArrayFloatMinimum(xs, m)
            }
            "global_cardinality" => {
                let first = self.parse_expr()?;
                self.expect_symbol(",")?;
                let second = self.parse_expr()?;
                if self.peek_is_symbol(")") {
                    Constraint::GlobalCardinality {
                        cover: first,
                        vars: second,
                        lbound: None,
                        ubound: None,
                    }
                } else {
                    self.expect_symbol(",")?;
                    let lbound = self.parse_expr()?;
                    self.expect_symbol(",")?;
                    let ubound = self.parse_expr()?;
                    Constraint::GlobalCardinality {
                        vars: first,
                        cover: second,
                        lbound: Some(lbound),
                        ubound: Some(ubound),
                    }
                }
            }
            "table" => {
                let vars = self.parse_expr()?;
                self.expect_symbol(",")?;
                let tuples = self.parse_tuple_set()?;
                Constraint::Table { vars, tuples }
            }
            "bool_eq" => {
                let left = self.parse_expr()?;
                self.expect_symbol(",")?;
                let right = self.parse_expr()?;
                Constraint::BoolEq(left, right)
            }
            "bool2int" => {
                let bool_var = self.parse_expr()?;
                self.expect_symbol(",")?;
                let int_var = self.parse_expr()?;
                Constraint::Bool2Int(bool_var, int_var)
            }
            "circuit" => {
                let successors = self.parse_expr()?;
                Constraint::Circuit(successors)
            }
            "inverse" => {
                let forward = self.parse_expr()?;
                self.expect_symbol(",")?;
                let backward = self.parse_expr()?;
                Constraint::Inverse { forward, backward }
            }
            "diffn" => {
                let xs = self.parse_expr()?;
                self.expect_symbol(",")?;
                let ys = self.parse_expr()?;
                self.expect_symbol(",")?;
                let widths = self.parse_duration_spec()?;
                self.expect_symbol(",")?;
                let heights = self.parse_duration_spec()?;
                Constraint::Diffn {
                    xs,
                    ys,
                    widths,
                    heights,
                }
            }
            "regular" => {
                let vars_expr = self.parse_expr()?;
                let vars = match vars_expr {
                    Expr::List(items) => items,
                    other => vec![other],
                };
                self.expect_symbol(",")?;
                let num_symbols = self.expect_int()?;
                self.expect_symbol(",")?;
                let num_states = self.expect_int()?;
                self.expect_symbol(",")?;
                let transitions = self.expect_ident_token()?;
                self.expect_symbol(",")?;
                let start = self.expect_int()?;
                self.expect_symbol(",")?;
                let accepting = self.expect_int()?;
                Constraint::Regular {
                    vars,
                    num_symbols,
                    num_states,
                    transitions,
                    start,
                    accepting: vec![accepting],
                }
            }
            "set_card" => {
                let set = self.parse_expr()?;
                self.expect_symbol(",")?;
                let card = self.parse_expr()?;
                Constraint::SetCard(set, card)
            }
            "set_subset" => {
                let subset = self.parse_expr()?;
                self.expect_symbol(",")?;
                let superset = self.parse_expr()?;
                Constraint::SetSubset(subset, superset)
            }
            "set_eq" => {
                let left = self.parse_expr()?;
                self.expect_symbol(",")?;
                let right = self.parse_expr()?;
                Constraint::SetEq(left, right)
            }
            "set_in" => {
                let value = self.parse_expr()?;
                self.expect_symbol(",")?;
                let set = self.parse_expr()?;
                Constraint::SetIn(value, set)
            }
            "set_superset" => {
                let superset = self.parse_expr()?;
                self.expect_symbol(",")?;
                let subset = self.parse_expr()?;
                Constraint::SetSuperset(superset, subset)
            }
            "set_le" => {
                let left = self.parse_expr()?;
                self.expect_symbol(",")?;
                let right = self.parse_expr()?;
                Constraint::SetLe(left, right)
            }
            "set_ne" => {
                let left = self.parse_expr()?;
                self.expect_symbol(",")?;
                let right = self.parse_expr()?;
                Constraint::SetNe(left, right)
            }
            "set_lt" => {
                let left = self.parse_expr()?;
                self.expect_symbol(",")?;
                let right = self.parse_expr()?;
                Constraint::SetLt(left, right)
            }
            "set_diff" => {
                let left = self.parse_expr()?;
                self.expect_symbol(",")?;
                let right = self.parse_expr()?;
                self.expect_symbol(",")?;
                let result = self.parse_expr()?;
                Constraint::SetDiff(left, right, result)
            }
            "set_symdiff" => {
                let left = self.parse_expr()?;
                self.expect_symbol(",")?;
                let right = self.parse_expr()?;
                self.expect_symbol(",")?;
                let result = self.parse_expr()?;
                Constraint::SetSymdiff(left, right, result)
            }
            "set_eq_reif" => {
                let left = self.parse_expr()?;
                self.expect_symbol(",")?;
                let right = self.parse_expr()?;
                self.expect_symbol(",")?;
                let reif = self.parse_expr()?;
                Constraint::SetEqReif(left, right, reif)
            }
            "set_ne_reif" => {
                let left = self.parse_expr()?;
                self.expect_symbol(",")?;
                let right = self.parse_expr()?;
                self.expect_symbol(",")?;
                let reif = self.parse_expr()?;
                Constraint::SetNeReif(left, right, reif)
            }
            "set_in_reif" => {
                let value = self.parse_expr()?;
                self.expect_symbol(",")?;
                let set = self.parse_expr()?;
                self.expect_symbol(",")?;
                let reif = self.parse_expr()?;
                Constraint::SetInReif(value, set, reif)
            }
            "set_subset_reif" => {
                let subset = self.parse_expr()?;
                self.expect_symbol(",")?;
                let superset = self.parse_expr()?;
                self.expect_symbol(",")?;
                let reif = self.parse_expr()?;
                Constraint::SetSubsetReif(subset, superset, reif)
            }
            "set_superset_reif" => {
                let superset = self.parse_expr()?;
                self.expect_symbol(",")?;
                let subset = self.parse_expr()?;
                self.expect_symbol(",")?;
                let reif = self.parse_expr()?;
                Constraint::SetSupersetReif(superset, subset, reif)
            }
            "set_le_reif" => {
                let left = self.parse_expr()?;
                self.expect_symbol(",")?;
                let right = self.parse_expr()?;
                self.expect_symbol(",")?;
                let reif = self.parse_expr()?;
                Constraint::SetLeReif(left, right, reif)
            }
            "set_lt_reif" => {
                let left = self.parse_expr()?;
                self.expect_symbol(",")?;
                let right = self.parse_expr()?;
                self.expect_symbol(",")?;
                let reif = self.parse_expr()?;
                Constraint::SetLtReif(left, right, reif)
            }
            "array_var_set_element" => {
                let array = self.parse_expr()?;
                self.expect_symbol(",")?;
                let index = self.parse_expr()?;
                self.expect_symbol(",")?;
                let value = self.parse_expr()?;
                Constraint::ArrayVarSetElement {
                    array,
                    index,
                    value,
                    one_based: true,
                }
            }
            "array_var_set_element_nonshifted" => {
                let array = self.parse_expr()?;
                self.expect_symbol(",")?;
                let index = self.parse_expr()?;
                self.expect_symbol(",")?;
                let value = self.parse_expr()?;
                Constraint::ArrayVarSetElement {
                    array,
                    index,
                    value,
                    one_based: false,
                }
            }
            "float_le" => {
                let left = self.parse_expr()?;
                self.expect_symbol(",")?;
                let right = self.parse_expr()?;
                Constraint::FloatLe(left, right)
            }
            "float_eq" => {
                let left = self.parse_expr()?;
                self.expect_symbol(",")?;
                let right = self.parse_expr()?;
                Constraint::FloatEq(left, right)
            }
            "set_union" => {
                let left = self.parse_expr()?;
                self.expect_symbol(",")?;
                let right = self.parse_expr()?;
                self.expect_symbol(",")?;
                let result = self.parse_expr()?;
                Constraint::SetUnion(left, right, result)
            }
            "set_intersect" => {
                let left = self.parse_expr()?;
                self.expect_symbol(",")?;
                let right = self.parse_expr()?;
                self.expect_symbol(",")?;
                let result = self.parse_expr()?;
                Constraint::SetIntersect(left, right, result)
            }
            "float_times" => {
                let a = self.parse_expr()?;
                self.expect_symbol(",")?;
                let b = self.parse_expr()?;
                self.expect_symbol(",")?;
                let c = self.parse_expr()?;
                Constraint::FloatTimes(a, b, c)
            }
            "float_plus" => {
                let a = self.parse_expr()?;
                self.expect_symbol(",")?;
                let b = self.parse_expr()?;
                self.expect_symbol(",")?;
                let c = self.parse_expr()?;
                Constraint::FloatPlus(a, b, c)
            }
            "float_abs" => {
                let a = self.parse_expr()?;
                self.expect_symbol(",")?;
                let b = self.parse_expr()?;
                Constraint::FloatAbs(a, b)
            }
            "float_div" => {
                let a = self.parse_expr()?;
                self.expect_symbol(",")?;
                let b = self.parse_expr()?;
                self.expect_symbol(",")?;
                let c = self.parse_expr()?;
                Constraint::FloatDiv(a, b, c)
            }
            "float_lt" => {
                let a = self.parse_expr()?;
                self.expect_symbol(",")?;
                let b = self.parse_expr()?;
                Constraint::FloatLt(a, b)
            }
            "float_ne" => {
                let a = self.parse_expr()?;
                self.expect_symbol(",")?;
                let b = self.parse_expr()?;
                Constraint::FloatNe(a, b)
            }
            "float_max" => {
                let a = self.parse_expr()?;
                self.expect_symbol(",")?;
                let b = self.parse_expr()?;
                self.expect_symbol(",")?;
                let c = self.parse_expr()?;
                Constraint::FloatMax(a, b, c)
            }
            "float_min" => {
                let a = self.parse_expr()?;
                self.expect_symbol(",")?;
                let b = self.parse_expr()?;
                self.expect_symbol(",")?;
                let c = self.parse_expr()?;
                Constraint::FloatMin(a, b, c)
            }
            "int2float" => {
                let int_var = self.parse_expr()?;
                self.expect_symbol(",")?;
                let float_var = self.parse_expr()?;
                Constraint::Int2Float(int_var, float_var)
            }
            "float_sqrt" => {
                let a = self.parse_expr()?;
                self.expect_symbol(",")?;
                let b = self.parse_expr()?;
                Constraint::FloatSqrt(a, b)
            }
            "float_sin" => {
                let a = self.parse_expr()?;
                self.expect_symbol(",")?;
                let b = self.parse_expr()?;
                Constraint::FloatSin(a, b)
            }
            "float_cos" => {
                let a = self.parse_expr()?;
                self.expect_symbol(",")?;
                let b = self.parse_expr()?;
                Constraint::FloatCos(a, b)
            }
            "float_ln" => {
                let a = self.parse_expr()?;
                self.expect_symbol(",")?;
                let b = self.parse_expr()?;
                Constraint::FloatLn(a, b)
            }
            "float_log2" => {
                let a = self.parse_expr()?;
                self.expect_symbol(",")?;
                let b = self.parse_expr()?;
                Constraint::FloatLog2(a, b)
            }
            "float_exp" => {
                let a = self.parse_expr()?;
                self.expect_symbol(",")?;
                let b = self.parse_expr()?;
                Constraint::FloatExp(a, b)
            }
            "float_ceil" => {
                let a = self.parse_expr()?;
                self.expect_symbol(",")?;
                let b = self.parse_expr()?;
                Constraint::FloatCeil(a, b)
            }
            "float_floor" => {
                let a = self.parse_expr()?;
                self.expect_symbol(",")?;
                let b = self.parse_expr()?;
                Constraint::FloatFloor(a, b)
            }
            "float_round" => {
                let a = self.parse_expr()?;
                self.expect_symbol(",")?;
                let b = self.parse_expr()?;
                Constraint::FloatRound(a, b)
            }
            "float_lin_eq" => {
                self.expect_symbol("[")?;
                let coeffs = self.parse_float_list()?;
                self.expect_symbol("]")?;
                self.expect_symbol(",")?;
                self.expect_symbol("[")?;
                let vars = self.parse_expr_list()?;
                self.expect_symbol("]")?;
                self.expect_symbol(",")?;
                let rhs = self.expect_float()?;
                Constraint::FloatLinEq { coeffs, vars, rhs }
            }
            "float_lin_ne" => {
                self.expect_symbol("[")?;
                let coeffs = self.parse_float_list()?;
                self.expect_symbol("]")?;
                self.expect_symbol(",")?;
                self.expect_symbol("[")?;
                let vars = self.parse_expr_list()?;
                self.expect_symbol("]")?;
                self.expect_symbol(",")?;
                let rhs = self.expect_float()?;
                Constraint::FloatLinNe { coeffs, vars, rhs }
            }
            "float_lin_le" => {
                self.expect_symbol("[")?;
                let coeffs = self.parse_float_list()?;
                self.expect_symbol("]")?;
                self.expect_symbol(",")?;
                self.expect_symbol("[")?;
                let vars = self.parse_expr_list()?;
                self.expect_symbol("]")?;
                self.expect_symbol(",")?;
                let rhs = self.expect_float()?;
                Constraint::FloatLinLe { coeffs, vars, rhs }
            }
            "float_lin_ge" => {
                self.expect_symbol("[")?;
                let coeffs = self.parse_float_list()?;
                self.expect_symbol("]")?;
                self.expect_symbol(",")?;
                self.expect_symbol("[")?;
                let vars = self.parse_expr_list()?;
                self.expect_symbol("]")?;
                self.expect_symbol(",")?;
                let rhs = self.expect_float()?;
                Constraint::FloatLinGe { coeffs, vars, rhs }
            }
            "float_lin_le_reif" => {
                self.expect_symbol("[")?;
                let coeffs = self.parse_float_list()?;
                self.expect_symbol("]")?;
                self.expect_symbol(",")?;
                self.expect_symbol("[")?;
                let vars = self.parse_expr_list()?;
                self.expect_symbol("]")?;
                self.expect_symbol(",")?;
                let rhs = self.expect_float()?;
                self.expect_symbol(",")?;
                let reif = self.parse_expr()?;
                Constraint::FloatLinLeReif {
                    coeffs,
                    vars,
                    rhs,
                    reif,
                }
            }
            "float_lin_ge_reif" => {
                self.expect_symbol("[")?;
                let coeffs = self.parse_float_list()?;
                self.expect_symbol("]")?;
                self.expect_symbol(",")?;
                self.expect_symbol("[")?;
                let vars = self.parse_expr_list()?;
                self.expect_symbol("]")?;
                self.expect_symbol(",")?;
                let rhs = self.expect_float()?;
                self.expect_symbol(",")?;
                let reif = self.parse_expr()?;
                Constraint::FloatLinGeReif {
                    coeffs,
                    vars,
                    rhs,
                    reif,
                }
            }
            "float_lin_eq_reif" => {
                self.expect_symbol("[")?;
                let coeffs = self.parse_float_list()?;
                self.expect_symbol("]")?;
                self.expect_symbol(",")?;
                self.expect_symbol("[")?;
                let vars = self.parse_expr_list()?;
                self.expect_symbol("]")?;
                self.expect_symbol(",")?;
                let rhs = self.expect_float()?;
                self.expect_symbol(",")?;
                let reif = self.parse_expr()?;
                Constraint::FloatLinEqReif {
                    coeffs,
                    vars,
                    rhs,
                    reif,
                }
            }
            "float_eq_reif" => {
                let a = self.parse_expr()?;
                self.expect_symbol(",")?;
                let b = self.parse_expr()?;
                self.expect_symbol(",")?;
                let reif = self.parse_expr()?;
                Constraint::FloatEqReif(a, b, reif)
            }
            "float_ne_reif" => {
                let a = self.parse_expr()?;
                self.expect_symbol(",")?;
                let b = self.parse_expr()?;
                self.expect_symbol(",")?;
                let reif = self.parse_expr()?;
                Constraint::FloatNeReif(a, b, reif)
            }
            "float_le_reif" => {
                let a = self.parse_expr()?;
                self.expect_symbol(",")?;
                let b = self.parse_expr()?;
                self.expect_symbol(",")?;
                let reif = self.parse_expr()?;
                Constraint::FloatLeReif(a, b, reif)
            }
            "float_lt_reif" => {
                let a = self.parse_expr()?;
                self.expect_symbol(",")?;
                let b = self.parse_expr()?;
                self.expect_symbol(",")?;
                let reif = self.parse_expr()?;
                Constraint::FloatLtReif(a, b, reif)
            }
            "int_abs" => {
                let a = self.parse_expr()?;
                self.expect_symbol(",")?;
                let b = self.parse_expr()?;
                Constraint::IntAbs(a, b)
            }
            "int_times" => {
                let a = self.parse_expr()?;
                self.expect_symbol(",")?;
                let b = self.parse_expr()?;
                self.expect_symbol(",")?;
                let c = self.parse_expr()?;
                Constraint::IntTimes(a, b, c)
            }
            "int_div" => {
                let a = self.parse_expr()?;
                self.expect_symbol(",")?;
                let b = self.parse_expr()?;
                self.expect_symbol(",")?;
                let c = self.parse_expr()?;
                Constraint::IntDiv(a, b, c)
            }
            "int_mod" => {
                let a = self.parse_expr()?;
                self.expect_symbol(",")?;
                let b = self.parse_expr()?;
                self.expect_symbol(",")?;
                let c = self.parse_expr()?;
                Constraint::IntMod(a, b, c)
            }
            "bool_not" => {
                let a = self.parse_expr()?;
                self.expect_symbol(",")?;
                let b = self.parse_expr()?;
                Constraint::BoolNot(a, b)
            }
            "bool_and" => {
                let a = self.parse_expr()?;
                self.expect_symbol(",")?;
                let b = self.parse_expr()?;
                self.expect_symbol(",")?;
                let c = self.parse_expr()?;
                Constraint::BoolAnd(a, b, c)
            }
            "bool_or" => {
                let a = self.parse_expr()?;
                self.expect_symbol(",")?;
                let b = self.parse_expr()?;
                self.expect_symbol(",")?;
                let c = self.parse_expr()?;
                Constraint::BoolOr(a, b, c)
            }
            "bool_xor" => {
                let a = self.parse_expr()?;
                self.expect_symbol(",")?;
                let b = self.parse_expr()?;
                self.expect_symbol(",")?;
                let c = self.parse_expr()?;
                Constraint::BoolXor(a, b, c)
            }
            "bool_clause" => {
                let literals = self.parse_expr()?;
                Constraint::BoolClause(literals)
            }
            "bool_clause_reif" => {
                let literals = self.parse_expr()?;
                self.expect_symbol(",")?;
                let reif = self.parse_expr()?;
                Constraint::BoolClauseReif(literals, reif)
            }
            "bool_eq_reif" => {
                let left = self.parse_expr()?;
                self.expect_symbol(",")?;
                let right = self.parse_expr()?;
                self.expect_symbol(",")?;
                let reif = self.parse_expr()?;
                Constraint::BoolEqReif(left, right, reif)
            }
            "bool_le" => {
                let left = self.parse_expr()?;
                self.expect_symbol(",")?;
                let right = self.parse_expr()?;
                Constraint::BoolLe(left, right)
            }
            "bool_le_reif" => {
                let left = self.parse_expr()?;
                self.expect_symbol(",")?;
                let right = self.parse_expr()?;
                self.expect_symbol(",")?;
                let reif = self.parse_expr()?;
                Constraint::BoolLeReif(left, right, reif)
            }
            "bool_lt" => {
                let left = self.parse_expr()?;
                self.expect_symbol(",")?;
                let right = self.parse_expr()?;
                Constraint::BoolLt(left, right)
            }
            "bool_lt_reif" => {
                let left = self.parse_expr()?;
                self.expect_symbol(",")?;
                let right = self.parse_expr()?;
                self.expect_symbol(",")?;
                let reif = self.parse_expr()?;
                Constraint::BoolLtReif(left, right, reif)
            }
            "bool_lin_eq" => {
                self.expect_symbol("[")?;
                let coeffs = self.parse_int_list()?;
                self.expect_symbol("]")?;
                self.expect_symbol(",")?;
                self.expect_symbol("[")?;
                let vars = self.parse_expr_list()?;
                self.expect_symbol("]")?;
                self.expect_symbol(",")?;
                let rhs = self.expect_int()?;
                Constraint::BoolLinEq { coeffs, vars, rhs }
            }
            "bool_lin_le" => {
                self.expect_symbol("[")?;
                let coeffs = self.parse_int_list()?;
                self.expect_symbol("]")?;
                self.expect_symbol(",")?;
                self.expect_symbol("[")?;
                let vars = self.parse_expr_list()?;
                self.expect_symbol("]")?;
                self.expect_symbol(",")?;
                let rhs = self.expect_int()?;
                Constraint::BoolLinLe { coeffs, vars, rhs }
            }
            "array_bool_and" => {
                let xs = self.parse_expr()?;
                self.expect_symbol(",")?;
                let c = self.parse_expr()?;
                Constraint::ArrayBoolAnd(xs, c)
            }
            "array_bool_xor" => {
                let xs = self.parse_expr()?;
                self.expect_symbol(",")?;
                let c = self.parse_expr()?;
                Constraint::ArrayBoolXor(xs, c)
            }
            "array_bool_element" => {
                let array = self.parse_expr()?;
                self.expect_symbol(",")?;
                let index = self.parse_expr()?;
                self.expect_symbol(",")?;
                let value = self.parse_expr()?;
                Constraint::ArrayBoolElement(array, index, value)
            }
            "array_var_bool_element" | "array_var_bool_element_nonshifted" => {
                let array = self.parse_expr()?;
                self.expect_symbol(",")?;
                let index = self.parse_expr()?;
                self.expect_symbol(",")?;
                let value = self.parse_expr()?;
                Constraint::ArrayVarBoolElement(array, index, value)
            }
            "int_plus" => {
                let a = self.parse_expr()?;
                self.expect_symbol(",")?;
                let b = self.parse_expr()?;
                self.expect_symbol(",")?;
                let c = self.parse_expr()?;
                Constraint::IntPlus(a, b, c)
            }
            "int_min" => {
                let a = self.parse_expr()?;
                self.expect_symbol(",")?;
                let b = self.parse_expr()?;
                self.expect_symbol(",")?;
                let c = self.parse_expr()?;
                Constraint::IntMin(a, b, c)
            }
            "int_max" => {
                let a = self.parse_expr()?;
                self.expect_symbol(",")?;
                let b = self.parse_expr()?;
                self.expect_symbol(",")?;
                let c = self.parse_expr()?;
                Constraint::IntMax(a, b, c)
            }
            "int_pow" => {
                let base = self.parse_expr()?;
                self.expect_symbol(",")?;
                let exp = self.parse_expr()?;
                self.expect_symbol(",")?;
                let result = self.parse_expr()?;
                Constraint::IntPow(base, exp, result)
            }
            "int_pow_fixed" => {
                let base = self.parse_expr()?;
                self.expect_symbol(",")?;
                let exp = self.expect_int()?;
                self.expect_symbol(",")?;
                let result = self.parse_expr()?;
                Constraint::IntPowFixed(base, exp, result)
            }
            "array_int_element" => {
                let array = self.parse_expr()?;
                self.expect_symbol(",")?;
                let index = self.parse_expr()?;
                self.expect_symbol(",")?;
                let value = self.parse_expr()?;
                Constraint::ArrayIntElement(array, index, value)
            }
            "array_var_int_element" => {
                let array = self.parse_expr()?;
                self.expect_symbol(",")?;
                let index = self.parse_expr()?;
                self.expect_symbol(",")?;
                let value = self.parse_expr()?;
                Constraint::ArrayVarIntElement(array, index, value)
            }
            "array_int_maximum" => {
                let xs = self.parse_expr()?;
                self.expect_symbol(",")?;
                let m = self.parse_expr()?;
                Constraint::ArrayIntMaximum(xs, m)
            }
            "array_int_minimum" => {
                let xs = self.parse_expr()?;
                self.expect_symbol(",")?;
                let m = self.parse_expr()?;
                Constraint::ArrayIntMinimum(xs, m)
            }
            "int_lin_ne" => {
                self.expect_symbol("[")?;
                let coeffs = self.parse_int_list()?;
                self.expect_symbol("]")?;
                self.expect_symbol(",")?;
                self.expect_symbol("[")?;
                let vars = self.parse_expr_list()?;
                self.expect_symbol("]")?;
                self.expect_symbol(",")?;
                let rhs = self.expect_int()?;
                Constraint::IntLinNe { coeffs, vars, rhs }
            }
            "automaton" => {
                let vars_expr = self.parse_expr()?;
                let vars = match vars_expr {
                    Expr::List(items) => items,
                    other => vec![other],
                };
                self.expect_symbol(",")?;
                let num_symbols = self.expect_int()?;
                self.expect_symbol(",")?;
                let num_states = self.expect_int()?;
                self.expect_symbol(",")?;
                let transitions = self.expect_ident_token()?;
                self.expect_symbol(",")?;
                let start = self.expect_int()?;
                self.expect_symbol(",")?;
                let accepting = if self.peek_is_symbol("{") {
                    self.expect_symbol("{")?;
                    let mut states = Vec::new();
                    states.push(self.expect_int()?);
                    while self.peek_is_symbol(",") {
                        self.expect_symbol(",")?;
                        states.push(self.expect_int()?);
                    }
                    self.expect_symbol("}")?;
                    states
                } else if self.peek_is_symbol("[") {
                    self.expect_symbol("[")?;
                    let mut states = Vec::new();
                    states.push(self.expect_int()?);
                    while self.peek_is_symbol(",") {
                        self.expect_symbol(",")?;
                        states.push(self.expect_int()?);
                    }
                    self.expect_symbol("]")?;
                    states
                } else {
                    vec![self.expect_int()?]
                };
                Constraint::Automaton {
                    vars,
                    num_symbols,
                    num_states,
                    transitions,
                    start,
                    accepting,
                }
            }
            other => {
                let args = self.parse_expr_list()?;
                Constraint::PredicateCall {
                    name: other.to_string(),
                    args,
                }
            }
        };
        Ok(constraint)
    }

    fn parse_predicate_decl(&mut self) -> Result<PredicateDecl, FlatZincError> {
        self.expect_ident("predicate")?;
        let name = self.expect_ident_token()?;
        self.expect_symbol("(")?;
        let mut params = Vec::new();
        if !self.peek_is_symbol(")") {
            loop {
                self.expect_ident("var")?;
                if self.peek_is_ident("int") {
                    self.expect_ident("int")?;
                } else if self.peek_is_ident("bool") {
                    self.expect_ident("bool")?;
                } else if self.peek_is_ident("set") {
                    self.expect_ident("set")?;
                } else if self.peek_is_ident("float") {
                    self.expect_ident("float")?;
                } else {
                    return Err(FlatZincError::Unsupported(
                        "predicate parameters must be var int, bool, set, or float".to_string(),
                    ));
                }
                self.expect_symbol(":")?;
                params.push(self.expect_ident_token()?);
                if self.peek_is_symbol(")") {
                    break;
                }
                self.expect_symbol(",")?;
            }
        }
        self.expect_symbol(")")?;
        self.expect_symbol("=")?;
        let body = self.parse_predicate_body_constraints()?;
        Ok(PredicateDecl { name, params, body })
    }

    fn parse_predicate_body_constraints(&mut self) -> Result<Vec<Constraint>, FlatZincError> {
        let mut constraints = Vec::new();
        loop {
            if self.peek_is_ident("constraint") {
                self.expect_ident("constraint")?;
            }
            let name = self.expect_ident_token()?;
            self.expect_symbol("(")?;
            let constraint = self.parse_constraint_by_name(&name)?;
            self.expect_symbol(")")?;
            constraints.push(constraint);
            if self.peek_is_symbol("/") {
                self.expect_symbol("/")?;
                self.expect_symbol("\\")?;
                continue;
            }
            if self.peek_is_symbol(";") {
                self.expect_symbol(";")?;
                if self.peek_is_ident("constraint") {
                    continue;
                }
            }
            break;
        }
        Ok(constraints)
    }

    fn parse_objective_exprs(&mut self) -> Result<Vec<Expr>, FlatZincError> {
        let first = self.parse_expr()?;
        match first {
            Expr::List(items) => Ok(items),
            expr => {
                let mut exprs = vec![expr];
                while self.peek_is_symbol(",") {
                    self.expect_symbol(",")?;
                    exprs.push(self.parse_expr()?);
                }
                Ok(exprs)
            }
        }
    }

    fn parse_tuple_set(&mut self) -> Result<Vec<i32>, FlatZincError> {
        self.expect_symbol("{")?;
        let flat = self.parse_int_list_braced()?;
        self.expect_symbol("}")?;
        Ok(flat)
    }

    fn parse_int_list_braced(&mut self) -> Result<Vec<i32>, FlatZincError> {
        let mut values = Vec::new();
        if self.peek_is_symbol("}") {
            return Ok(values);
        }
        loop {
            values.push(self.expect_int()?);
            if self.peek_is_symbol("}") {
                break;
            }
            self.expect_symbol(",")?;
        }
        Ok(values)
    }

    fn parse_duration_spec(&mut self) -> Result<DurationSpec, FlatZincError> {
        if self.peek_is_symbol("[") {
            self.expect_symbol("[")?;
            let values = self.parse_int_list()?;
            self.expect_symbol("]")?;
            Ok(DurationSpec::Inline(values))
        } else if let Some(Token::Ident(name)) = self.peek().cloned() {
            self.pos += 1;
            Ok(DurationSpec::Name(name))
        } else {
            Err(FlatZincError::UnexpectedToken {
                found: format!("{:?}", self.peek()),
                expected: "duration array".to_string(),
            })
        }
    }

    fn parse_output(&mut self) -> Result<OutputDirective, FlatZincError> {
        self.expect_ident("output")?;
        self.expect_symbol("[")?;
        let mut segments = Vec::new();
        if !self.peek_is_symbol("]") {
            loop {
                segments.extend(self.parse_output_item()?);
                if self.peek_is_symbol("]") {
                    break;
                }
                self.expect_symbol(",")?;
            }
        }
        self.expect_symbol("]")?;
        Ok(OutputDirective { segments })
    }

    fn parse_output_item(&mut self) -> Result<Vec<OutputSegment>, FlatZincError> {
        if self.peek_is_ident("show") {
            self.expect_ident("show")?;
            self.expect_symbol("(")?;
            let mut parts = Vec::new();
            if !self.peek_is_symbol(")") {
                loop {
                    parts.push(self.parse_output_arg()?);
                    if self.peek_is_symbol(")") {
                        break;
                    }
                    self.expect_symbol(",")?;
                }
            }
            self.expect_symbol(")")?;
            return Ok(parts);
        }
        let expr = self.parse_expr()?;
        Ok(vec![self.expr_to_output_segment(expr)?])
    }

    fn parse_output_arg(&mut self) -> Result<OutputSegment, FlatZincError> {
        if let Some(Token::Int(value)) = self.peek().cloned() {
            self.pos += 1;
            return Ok(OutputSegment::Text(value.to_string()));
        }
        if let Some(Token::String(text)) = self.peek().cloned() {
            self.pos += 1;
            return Ok(OutputSegment::Text(text));
        }
        let expr = self.parse_expr()?;
        self.expr_to_output_segment(expr)
    }

    fn expr_to_output_segment(&self, expr: Expr) -> Result<OutputSegment, FlatZincError> {
        match expr {
            Expr::Name(name) => Ok(OutputSegment::Variable(name)),
            Expr::Index { name, index } => {
                let index_value = match *index {
                    Expr::Int(value) => value.to_string(),
                    other => format!("{other:?}"),
                };
                Ok(OutputSegment::Variable(format!("{name}[{index_value}]")))
            }
            Expr::Int(value) => Ok(OutputSegment::Text(value.to_string())),
            Expr::List(_) => Err(FlatZincError::Unsupported(
                "list expression in output".to_string(),
            )),
        }
    }

    fn parse_solve(&mut self) -> Result<SolveDirective, FlatZincError> {
        self.expect_ident("solve")?;
        let mut annotations = SearchAnnotations::default();
        while self.peek_is_symbol("::") {
            self.expect_symbol("::")?;
            self.parse_search_annotation(&mut annotations)?;
        }
        let goal = if self.peek_is_ident("minimize") {
            self.expect_ident("minimize")?;
            SolveGoal::Minimize(self.parse_objective_exprs()?)
        } else if self.peek_is_ident("maximize") {
            self.expect_ident("maximize")?;
            SolveGoal::Maximize(self.parse_objective_exprs()?)
        } else {
            self.expect_ident("satisfy")?;
            SolveGoal::Satisfy
        };
        Ok(SolveDirective { annotations, goal })
    }

    fn parse_search_annotation(
        &mut self,
        annotations: &mut SearchAnnotations,
    ) -> Result<(), FlatZincError> {
        let name = self.expect_ident_token()?;
        match name.as_str() {
            "int_search" => {
                if annotations.int_search.is_some() {
                    return Err(FlatZincError::Unsupported(
                        "multiple int_search annotations".to_string(),
                    ));
                }
                annotations.int_search = Some(self.parse_typed_search_annotation("int_search")?);
            }
            "bool_search" => {
                if annotations.bool_search.is_some() {
                    return Err(FlatZincError::Unsupported(
                        "multiple bool_search annotations".to_string(),
                    ));
                }
                annotations.bool_search = Some(self.parse_typed_search_annotation("bool_search")?);
            }
            "set_search" => {
                if annotations.set_search.is_some() {
                    return Err(FlatZincError::Unsupported(
                        "multiple set_search annotations".to_string(),
                    ));
                }
                annotations.set_search = Some(self.parse_typed_search_annotation("set_search")?);
            }
            "float_search" => {
                if annotations.float_search.is_some() {
                    return Err(FlatZincError::Unsupported(
                        "multiple float_search annotations".to_string(),
                    ));
                }
                annotations.float_search = Some(self.parse_float_search_annotation()?);
            }
            "seq_search" => {
                if annotations.seq_search.is_some() {
                    return Err(FlatZincError::Unsupported(
                        "multiple seq_search annotations".to_string(),
                    ));
                }
                annotations.seq_search = Some(self.parse_seq_search_annotation()?);
            }
            "restart_constant" => {
                if annotations.restart.is_some() {
                    return Err(FlatZincError::Unsupported(
                        "multiple restart annotations".to_string(),
                    ));
                }
                self.expect_symbol("(")?;
                let scale = self.expect_non_negative_u64("restart_constant scale")?;
                self.expect_symbol(")")?;
                annotations.restart = Some(RestartAnnotation {
                    kind: RestartKind::Constant { scale },
                });
            }
            "restart_geometric" => {
                if annotations.restart.is_some() {
                    return Err(FlatZincError::Unsupported(
                        "multiple restart annotations".to_string(),
                    ));
                }
                self.expect_symbol("(")?;
                let base = self.expect_float_text()?;
                self.expect_symbol(",")?;
                let scale = self.expect_non_negative_u64("restart_geometric scale")?;
                self.expect_symbol(")")?;
                annotations.restart = Some(RestartAnnotation {
                    kind: RestartKind::Geometric { base, scale },
                });
            }
            "restart_luby" => {
                if annotations.restart.is_some() {
                    return Err(FlatZincError::Unsupported(
                        "multiple restart annotations".to_string(),
                    ));
                }
                self.expect_symbol("(")?;
                let base = self.expect_non_negative_u64("restart_luby base")?;
                if self.peek_is_symbol(",") {
                    self.expect_symbol(",")?;
                    self.expect_non_negative_u64("restart_luby scale")?;
                }
                self.expect_symbol(")")?;
                annotations.restart = Some(RestartAnnotation {
                    kind: RestartKind::Luby { base },
                });
            }
            "restart_none" => {
                if annotations.restart.is_some() {
                    return Err(FlatZincError::Unsupported(
                        "multiple restart annotations".to_string(),
                    ));
                }
                if self.peek_is_symbol("(") {
                    self.expect_symbol("(")?;
                    self.expect_symbol(")")?;
                }
                annotations.restart = Some(RestartAnnotation {
                    kind: RestartKind::None,
                });
            }
            "restart_linear" => {
                if annotations.restart.is_some() {
                    return Err(FlatZincError::Unsupported(
                        "multiple restart annotations".to_string(),
                    ));
                }
                self.expect_symbol("(")?;
                let scale = self.expect_non_negative_u64("restart_linear scale")?;
                self.expect_symbol(")")?;
                annotations.restart = Some(RestartAnnotation {
                    kind: RestartKind::Linear { scale },
                });
            }
            "restart_on_solution" => {
                if annotations.restart.is_some() {
                    return Err(FlatZincError::Unsupported(
                        "multiple restart annotations".to_string(),
                    ));
                }
                if self.peek_is_symbol("(") {
                    self.expect_symbol("(")?;
                    self.expect_symbol(")")?;
                }
                annotations.restart = Some(RestartAnnotation {
                    kind: RestartKind::OnSolution,
                });
            }
            "pareto" => {
                if annotations.pareto.is_some() {
                    return Err(FlatZincError::Unsupported(
                        "multiple pareto annotations".to_string(),
                    ));
                }
                self.expect_symbol("(")?;
                let expr = self.parse_expr()?;
                let vars = match expr {
                    Expr::List(items) => items,
                    other => vec![other],
                };
                self.expect_symbol(")")?;
                annotations.pareto = Some(vars);
            }
            other => {
                return Err(FlatZincError::Unsupported(format!(
                    "unsupported search annotation `{other}`"
                )));
            }
        }
        Ok(())
    }

    fn parse_typed_search_annotation(
        &mut self,
        kind: &str,
    ) -> Result<IntSearchAnnotation, FlatZincError> {
        self.expect_symbol("(")?;
        let vars_expr = self.parse_expr()?;
        let vars = match vars_expr {
            Expr::List(items) => items,
            other => vec![other],
        };
        self.expect_symbol(",")?;
        let var_choice = self.expect_ident_token()?;
        self.expect_symbol(",")?;
        let value_choice = self.expect_ident_token()?;
        self.expect_symbol(",")?;
        let complete = match self.expect_ident_token()?.as_str() {
            "complete" => true,
            "incomplete" => false,
            other => {
                return Err(FlatZincError::Unsupported(format!(
                    "unsupported {kind} completeness `{other}`"
                )));
            }
        };
        self.expect_symbol(")")?;
        Ok(IntSearchAnnotation {
            vars,
            var_choice,
            value_choice,
            complete,
        })
    }

    fn parse_float_search_annotation(&mut self) -> Result<IntSearchAnnotation, FlatZincError> {
        // float_search(vars, precision, var_choice, value_choice, complete)
        self.expect_symbol("(")?;
        let vars_expr = self.parse_expr()?;
        let vars = match vars_expr {
            Expr::List(items) => items,
            other => vec![other],
        };
        self.expect_symbol(",")?;
        let _precision = self.expect_float()?;
        self.expect_symbol(",")?;
        let var_choice = self.expect_ident_token()?;
        self.expect_symbol(",")?;
        let value_choice = self.expect_ident_token()?;
        self.expect_symbol(",")?;
        let complete = match self.expect_ident_token()?.as_str() {
            "complete" => true,
            "incomplete" => false,
            other => {
                return Err(FlatZincError::Unsupported(format!(
                    "unsupported float_search completeness `{other}`"
                )));
            }
        };
        self.expect_symbol(")")?;
        Ok(IntSearchAnnotation {
            vars,
            var_choice,
            value_choice,
            complete,
        })
    }

    fn parse_seq_search_annotation(&mut self) -> Result<Vec<IntSearchAnnotation>, FlatZincError> {
        self.expect_symbol("(")?;
        self.expect_symbol("[")?;
        let mut items = Vec::new();
        if !self.peek_is_symbol("]") {
            loop {
                items.push(self.parse_nested_search_annotation()?);
                if self.peek_is_symbol(",") {
                    self.expect_symbol(",")?;
                    continue;
                }
                break;
            }
        }
        self.expect_symbol("]")?;
        self.expect_symbol(")")?;
        if items.is_empty() {
            return Err(FlatZincError::Unsupported(
                "seq_search requires at least one nested search".to_string(),
            ));
        }
        Ok(items)
    }

    fn parse_nested_search_annotation(&mut self) -> Result<IntSearchAnnotation, FlatZincError> {
        let name = self.expect_ident_token()?;
        match name.as_str() {
            "int_search" | "bool_search" | "set_search" => {
                self.parse_typed_search_annotation(&name)
            }
            "float_search" => self.parse_float_search_annotation(),
            other => Err(FlatZincError::Unsupported(format!(
                "unsupported nested search annotation `{other}` in seq_search"
            ))),
        }
    }

    fn parse_expr_list(&mut self) -> Result<Vec<Expr>, FlatZincError> {
        let mut exprs = Vec::new();
        if self.peek_is_symbol("]") || self.peek_is_symbol(")") {
            return Ok(exprs);
        }
        loop {
            exprs.push(self.parse_expr()?);
            if self.peek_is_symbol("]") || self.peek_is_symbol(")") {
                break;
            }
            self.expect_symbol(",")?;
        }
        Ok(exprs)
    }

    fn parse_int_list(&mut self) -> Result<Vec<i32>, FlatZincError> {
        let mut values = Vec::new();
        if self.peek_is_symbol("]") {
            return Ok(values);
        }
        loop {
            values.push(self.expect_int()?);
            if self.peek_is_symbol("]") {
                break;
            }
            self.expect_symbol(",")?;
        }
        Ok(values)
    }

    fn parse_float_list(&mut self) -> Result<Vec<f64>, FlatZincError> {
        let mut values = Vec::new();
        if self.peek_is_symbol("]") {
            return Ok(values);
        }
        loop {
            values.push(self.expect_float()?);
            if self.peek_is_symbol("]") {
                break;
            }
            self.expect_symbol(",")?;
        }
        Ok(values)
    }

    fn expect_float(&mut self) -> Result<f64, FlatZincError> {
        self.expect_float_text()?
            .parse::<f64>()
            .map_err(|_| FlatZincError::Unsupported("invalid float literal".to_string()))
    }

    fn parse_expr(&mut self) -> Result<Expr, FlatZincError> {
        if self.peek_is_symbol("[") {
            self.expect_symbol("[")?;
            let exprs = self.parse_expr_list()?;
            self.expect_symbol("]")?;
            return Ok(Expr::List(exprs));
        }

        if let Some(Token::Int(value)) = self.peek().cloned() {
            self.pos += 1;
            return Ok(Expr::Int(value));
        }

        if let Some(Token::Ident(name)) = self.peek().cloned() {
            if name == "array" {
                self.expect_ident("array")?;
                self.expect_symbol("(")?;
                self.expect_int()?;
                self.expect_symbol("..")?;
                self.expect_int()?;
                self.expect_symbol(")")?;
                self.expect_symbol("(")?;
                let inner = self.parse_expr()?;
                self.expect_symbol(")")?;
                return Ok(inner);
            }
            self.pos += 1;
            if self.peek_is_symbol("[") {
                self.expect_symbol("[")?;
                let index = self.parse_expr()?;
                self.expect_symbol("]")?;
                return Ok(Expr::Index {
                    name,
                    index: Box::new(index),
                });
            }
            return Ok(Expr::Name(name));
        }

        Err(FlatZincError::UnexpectedToken {
            found: format!("{:?}", self.peek()),
            expected: "expression".to_string(),
        })
    }

    fn consume_optional_semicolon(&mut self) {
        if self.peek_is_symbol(";") {
            self.pos += 1;
        }
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn peek_is_ident(&self, expected: &str) -> bool {
        matches!(self.peek(), Some(Token::Ident(name)) if name == expected)
    }

    fn peek_is_symbol(&self, expected: &str) -> bool {
        matches!(self.peek(), Some(Token::Symbol(symbol)) if symbol == expected)
    }

    fn expect_ident(&mut self, expected: &str) -> Result<(), FlatZincError> {
        match self.peek() {
            Some(Token::Ident(name)) if name == expected => {
                self.pos += 1;
                Ok(())
            }
            Some(other) => Err(FlatZincError::UnexpectedToken {
                found: format!("{other:?}"),
                expected: expected.to_string(),
            }),
            None => Err(FlatZincError::UnexpectedEof),
        }
    }

    fn expect_ident_token(&mut self) -> Result<String, FlatZincError> {
        if let Some(Token::Ident(name)) = self.peek().cloned() {
            self.pos += 1;
            Ok(name)
        } else {
            Err(FlatZincError::UnexpectedToken {
                found: format!("{:?}", self.peek()),
                expected: "identifier".to_string(),
            })
        }
    }

    fn expect_symbol(&mut self, expected: &str) -> Result<(), FlatZincError> {
        match self.peek() {
            Some(Token::Symbol(symbol)) if symbol == expected => {
                self.pos += 1;
                Ok(())
            }
            Some(other) => Err(FlatZincError::UnexpectedToken {
                found: format!("{other:?}"),
                expected: expected.to_string(),
            }),
            None => Err(FlatZincError::UnexpectedEof),
        }
    }

    fn expect_int(&mut self) -> Result<i32, FlatZincError> {
        if let Some(Token::Int(value)) = self.peek().cloned() {
            self.pos += 1;
            Ok(value)
        } else {
            Err(FlatZincError::UnexpectedToken {
                found: format!("{:?}", self.peek()),
                expected: "integer".to_string(),
            })
        }
    }

    fn expect_float_text(&mut self) -> Result<String, FlatZincError> {
        match self.peek().cloned() {
            Some(Token::Float(value)) => {
                self.pos += 1;
                Ok(value)
            }
            Some(Token::Int(value)) => {
                self.pos += 1;
                Ok(value.to_string())
            }
            _ => Err(FlatZincError::UnexpectedToken {
                found: format!("{:?}", self.peek()),
                expected: "float".to_string(),
            }),
        }
    }

    fn expect_non_negative_u64(&mut self, label: &str) -> Result<u64, FlatZincError> {
        let value = self.expect_int()?;
        if value < 0 {
            return Err(FlatZincError::Unsupported(format!(
                "{label} must be non-negative"
            )));
        }
        u64::try_from(value)
            .map_err(|_| FlatZincError::Unsupported(format!("{label} is too large")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_program() {
        let source = r#"
            int: n = 3;
            array [1..3] of var 1..3: x;
            constraint all_different(x);
            constraint int_lin_eq([1,1,1], [x[1], x[2], x[3]], 6);
            solve satisfy;
        "#;
        let program = parse(source).unwrap();
        assert_eq!(
            program.params,
            vec![ParamDecl::Int {
                name: "n".to_string(),
                value: 3
            }]
        );
        assert_eq!(program.constraints.len(), 2);
    }

    #[test]
    fn parses_global_cardinality_and_table() {
        let source = r#"
            array [1..2] of int: cards = [1, 2];
            array [1..2] of var 1..2: x;
            var 1..3: a;
            var 1..3: b;
            constraint global_cardinality(cards, x);
            constraint table([a, b], {1, 2, 2, 3});
            solve satisfy;
        "#;
        let program = parse(source).unwrap();
        assert_eq!(program.constraints.len(), 2);
    }

    #[test]
    fn parses_output_directive() {
        let source = r#"
            var 1..3: x;
            output [ show("x=", x) ];
            solve satisfy;
        "#;
        let program = parse(source).unwrap();
        assert_eq!(program.outputs.len(), 1);
        assert_eq!(program.outputs[0].segments.len(), 2);
    }

    #[test]
    fn parses_bool_variables_and_constraints() {
        let source = r#"
            var bool: b;
            array [1..2] of var bool: flags;
            var 0..5: x;
            constraint bool_eq(b, flags[1]);
            constraint bool2int(b, x);
            solve satisfy;
        "#;
        let program = parse(source).unwrap();
        assert_eq!(program.variables.len(), 3);
        assert_eq!(program.constraints.len(), 2);
        assert!(matches!(program.constraints[0], Constraint::BoolEq(_, _)));
        assert!(matches!(program.constraints[1], Constraint::Bool2Int(_, _)));
    }

    #[test]
    fn unknown_constraint_becomes_predicate_call() {
        let source = r#"
            var 1..3: x;
            constraint unknown_constraint(x);
            solve satisfy;
        "#;
        let program = parse(source).expect("unknown constraints parse as predicate calls");
        assert!(matches!(
            &program.constraints[0],
            crate::Constraint::PredicateCall { name, .. } if name == "unknown_constraint"
        ));
    }

    #[test]
    fn parses_predicate_declaration() {
        let source = r#"
            predicate foo(var int: x) = int_eq(x, 1);
            var 1..3: y;
            constraint foo(y);
            solve satisfy;
        "#;
        let program = parse(source).expect("predicate should parse");
        assert_eq!(program.predicates.len(), 1);
        assert_eq!(program.constraints.len(), 1);
    }

    #[test]
    fn skips_function_declaration() {
        let source = r#"
            function int: id(int: x) = x;
            var 1..3: a;
            constraint int_eq(a, 2);
            solve satisfy;
        "#;
        let program = parse(source).expect("function should be skipped like annotation");
        assert_eq!(program.variables.len(), 1);
        assert_eq!(program.constraints.len(), 1);
    }

    #[test]
    fn skips_test_declaration() {
        let source = r#"
            test check() = assert(true);
            var 1..3: a;
            constraint int_eq(a, 2);
            solve satisfy;
        "#;
        let program = parse(source).expect("test should be skipped like annotation");
        assert_eq!(program.variables.len(), 1);
        assert_eq!(program.constraints.len(), 1);
    }

    #[test]
    fn skips_annotation_top_level_statement() {
        let source = r#"
            annotation foo;
            var 1..3: x;
            constraint int_eq(x, 1);
            solve satisfy;
        "#;
        let program = parse(source).expect("annotation should be skipped");
        assert_eq!(program.variables.len(), 1);
        assert_eq!(program.constraints.len(), 1);
    }

    #[test]
    fn parses_int_search_annotation() {
        let source = r#"
            array [1..3] of var 1..3: x;
            solve :: int_search([x[1], x[2], x[3]], first_fail, indomain_min, complete) satisfy;
        "#;
        let program = parse(source).unwrap();
        let int_search = program
            .solve
            .annotations
            .int_search
            .as_ref()
            .expect("int_search");
        assert_eq!(int_search.vars.len(), 3);
        assert_eq!(int_search.var_choice, "first_fail");
        assert_eq!(int_search.value_choice, "indomain_min");
        assert!(int_search.complete);
        assert!(matches!(program.solve.goal, SolveGoal::Satisfy));
    }

    #[test]
    fn parses_float_search_with_precision() {
        let source = r#"
            var 0.0..1.0: x;
            solve :: float_search([x], 0.001, input_order, indomain_split, complete) satisfy;
        "#;
        let program = parse(source).unwrap();
        let float_search = program
            .solve
            .annotations
            .float_search
            .expect("float_search");
        assert_eq!(float_search.vars.len(), 1);
        assert_eq!(float_search.var_choice, "input_order");
        assert_eq!(float_search.value_choice, "indomain_split");
    }

    #[test]
    fn parses_set_search_annotation() {
        let source = r#"
            var set of 1..3: s;
            solve :: set_search([s], first_fail, indomain_min, complete) satisfy;
        "#;
        let program = parse(source).unwrap();
        assert!(program.solve.annotations.set_search.is_some());
    }

    #[test]
    fn parses_seq_search_annotation() {
        let source = r#"
            var 1..3: x;
            var 1..3: y;
            solve :: seq_search([int_search([x], first_fail, indomain_min, complete), int_search([y], input_order, indomain_max, complete)]) satisfy;
        "#;
        let program = parse(source).unwrap();
        let seq = program.solve.annotations.seq_search.expect("seq_search");
        assert_eq!(seq.len(), 2);
        assert_eq!(seq[0].var_choice, "first_fail");
        assert_eq!(seq[1].value_choice, "indomain_max");
    }

    #[test]
    fn parses_restart_and_int_search_annotations() {
        let source = r#"
            var 1..3: x;
            solve :: restart_luby(256) :: int_search([x], input_order, indomain_max, complete) satisfy;
        "#;
        let program = parse(source).unwrap();
        assert!(matches!(
            program.solve.annotations.restart,
            Some(RestartAnnotation {
                kind: RestartKind::Luby { base: 256 }
            })
        ));
        assert!(program.solve.annotations.int_search.is_some());
    }

    #[test]
    fn parses_constant_and_geometric_restart_annotations() {
        let constant = r#"
            var 1..3: x;
            solve :: restart_constant(100) satisfy;
        "#;
        let program = parse(constant).unwrap();
        assert!(matches!(
            program.solve.annotations.restart,
            Some(RestartAnnotation {
                kind: RestartKind::Constant { scale: 100 }
            })
        ));

        let geometric = r#"
            var 1..3: x;
            solve :: restart_geometric(1.5, 100) satisfy;
        "#;
        let program = parse(geometric).unwrap();
        assert!(matches!(
            program.solve.annotations.restart,
            Some(RestartAnnotation {
                kind: RestartKind::Geometric {
                    ref base,
                    scale: 100
                }
            }) if base == "1.5"
        ));
    }

    #[test]
    fn parses_int_search_with_array_name() {
        let source = r#"
            array [1..2] of var 1..2: x;
            solve :: int_search(x, first_fail, indomain_min, complete) satisfy;
        "#;
        let program = parse(source).unwrap();
        let int_search = program.solve.annotations.int_search.unwrap();
        assert_eq!(int_search.vars.len(), 1);
        assert!(matches!(int_search.vars[0], Expr::Name(ref name) if name == "x"));
    }

    #[test]
    fn parses_minimize_with_int_search() {
        let source = r#"
            var 0..10: x;
            solve :: int_search([x], first_fail, indomain_min, complete) minimize x;
        "#;
        let program = parse(source).unwrap();
        assert!(matches!(program.solve.goal, SolveGoal::Minimize(_)));
        assert!(program.solve.annotations.int_search.is_some());
    }

    #[test]
    fn parses_restart_none_without_parens() {
        let source = r#"
            var 1..3: x;
            solve :: restart_none :: int_search([x], first_fail, indomain_min, complete) satisfy;
        "#;
        let program = parse(source).unwrap();
        assert!(matches!(
            program.solve.annotations.restart,
            Some(RestartAnnotation {
                kind: RestartKind::None
            })
        ));
    }

    #[test]
    fn parses_multi_constraint_predicate() {
        let source = r#"
            predicate chained(var int: a, var int: b) =
              constraint int_le(a, b) /\
              constraint all_different([a, b]);
            var 1..3: x;
            solve satisfy;
        "#;
        let program = parse(source).expect("multi-constraint predicate should parse");
        assert_eq!(program.predicates[0].body.len(), 2);
    }

    #[test]
    fn parses_set_parameter() {
        let source = r#"
            set of 1..3: allowed = {1, 3};
            var 1..3: x;
            constraint set_in(x, allowed);
            solve satisfy;
        "#;
        let program = parse(source).expect("set parameter should parse");
        assert_eq!(program.params.len(), 1);
        assert!(matches!(&program.params[0], ParamDecl::Set { .. }));
    }

    #[test]
    fn parses_lexicographic_objectives() {
        let source = r#"
            var 1..3: x;
            var 1..3: y;
            solve minimize x, y;
        "#;
        let program = parse(source).expect("lexicographic objectives should parse");
        assert!(matches!(program.solve.goal, SolveGoal::Minimize(ref exprs) if exprs.len() == 2));
    }
}
