use crate::error::FlatZincError;

/// Parsed FlatZinc program (subset).
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone, PartialEq, Eq)]
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
}

/// A FlatZinc variable declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
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
}

/// A FlatZinc constraint call.
#[derive(Debug, Clone, PartialEq, Eq)]
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
    /// User-defined predicate call.
    PredicateCall {
        /// Predicate name.
        name: String,
        /// Call arguments.
        args: Vec<Expr>,
    },
}

/// A parsed user-defined predicate with a single-constraint body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PredicateDecl {
    /// Predicate name.
    pub name: String,
    /// Formal parameter names in order.
    pub params: Vec<String>,
    /// Inlined constraint body.
    pub body: Constraint,
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
    /// `restart_*` annotation, when present.
    pub restart: Option<RestartAnnotation>,
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
    /// `solve minimize expr`
    Minimize(Expr),
    /// `solve maximize expr`
    Maximize(Expr),
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
            } else if self.peek_is_ident("function") {
                return Err(FlatZincError::Unsupported(
                    "function declarations are not supported".to_string(),
                ));
            } else if self.peek_is_ident("test") {
                return Err(FlatZincError::Unsupported(
                    "test declarations are not supported".to_string(),
                ));
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

    fn parse_constraint(&mut self) -> Result<Constraint, FlatZincError> {
        self.expect_ident("constraint")?;
        let name = self.expect_ident_token()?;
        self.expect_symbol("(")?;
        let constraint = match name.as_str() {
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
            other => {
                let args = self.parse_expr_list()?;
                Constraint::PredicateCall {
                    name: other.to_string(),
                    args,
                }
            }
        };
        self.expect_symbol(")")?;
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
                } else {
                    return Err(FlatZincError::Unsupported(
                        "predicate parameters must be var int or var bool".to_string(),
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
        let body = self.parse_predicate_body_constraint()?;
        Ok(PredicateDecl { name, params, body })
    }

    fn parse_predicate_body_constraint(&mut self) -> Result<Constraint, FlatZincError> {
        if self.peek_is_ident("constraint") {
            self.expect_ident("constraint")?;
        }
        let name = self.expect_ident_token()?;
        self.expect_symbol("(")?;
        let constraint = match name.as_str() {
            "int_eq" => {
                let left = self.parse_expr()?;
                self.expect_symbol(",")?;
                let right = self.parse_expr()?;
                Constraint::IntEq(left, right)
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
            "all_different" => {
                let expr = self.parse_expr()?;
                let args = match expr {
                    Expr::List(items) => items,
                    other => vec![other],
                };
                Constraint::AllDifferent(args)
            }
            other => {
                return Err(FlatZincError::Unsupported(format!(
                    "predicate body constraint `{other}` is not supported for inline expansion"
                )));
            }
        };
        self.expect_symbol(")")?;
        Ok(constraint)
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
            let expr = self.parse_expr()?;
            SolveGoal::Minimize(expr)
        } else if self.peek_is_ident("maximize") {
            self.expect_ident("maximize")?;
            let expr = self.parse_expr()?;
            SolveGoal::Maximize(expr)
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
                            "unsupported int_search completeness `{other}`"
                        )));
                    }
                };
                self.expect_symbol(")")?;
                annotations.int_search = Some(IntSearchAnnotation {
                    vars,
                    var_choice,
                    value_choice,
                    complete,
                });
            }
            "bool_search" => {
                if annotations.bool_search.is_some() {
                    return Err(FlatZincError::Unsupported(
                        "multiple bool_search annotations".to_string(),
                    ));
                }
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
                            "unsupported bool_search completeness `{other}`"
                        )));
                    }
                };
                self.expect_symbol(")")?;
                annotations.bool_search = Some(IntSearchAnnotation {
                    vars,
                    var_choice,
                    value_choice,
                    complete,
                });
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
            other => {
                return Err(FlatZincError::Unsupported(format!(
                    "unsupported search annotation `{other}`"
                )));
            }
        }
        Ok(())
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
    fn rejects_unknown_top_level_statement() {
        let source = r#"
            var 1..3: x;
            annotation foo;
            solve satisfy;
        "#;
        let err = parse(source).unwrap_err();
        assert!(err.to_string().contains("unsupported top-level"));
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
    fn parses_restart_linear_annotation() {
        let source = r#"
            var 1..3: x;
            solve :: restart_linear(100) satisfy;
        "#;
        let program = parse(source).expect("restart_linear should parse");
        assert!(matches!(
            program.solve.annotations.restart,
            Some(RestartAnnotation {
                kind: RestartKind::Linear { scale: 100 }
            })
        ));
    }
}
