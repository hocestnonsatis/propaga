use thiserror::Error;

/// Errors raised while parsing or compiling FlatZinc.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum FlatZincError {
    /// Unexpected end of input.
    #[error("unexpected end of input")]
    UnexpectedEof,

    /// A referenced name was not declared.
    #[error("unknown identifier `{0}`")]
    UnknownIdentifier(String),

    /// A token did not match the expected syntax.
    #[error("unexpected token `{found}` (expected {expected})")]
    UnexpectedToken {
        /// Token that was found.
        found: String,
        /// Human-readable expectation.
        expected: String,
    },

    /// A statement is not supported by this subset parser.
    #[error("unsupported FlatZinc construct: {0}")]
    Unsupported(String),

    /// Integer overflow or invalid numeric literal.
    #[error("invalid integer `{0}`")]
    InvalidInteger(String),

    /// The model has no `solve` directive.
    #[error("missing solve directive")]
    MissingSolve,
}
