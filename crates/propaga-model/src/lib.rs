//! High-level modeling API for Propaga.
//!
//! [`Model`] wraps the engine and propagators with a concise API for declaring
//! variables, posting constraints, and running search or optimization.
//!
//! # Example
//!
//! ```
//! use propaga_model::Model;
//!
//! let mut model = Model::new();
//! let x = model.int_var(1, 9);
//! let y = model.int_var(1, 9);
//! model.all_different(&[x, y]);
//! model.equal(x, y);
//! assert!(model.solve_subset(vec![x, y]).is_none());
//! ```

mod model;

pub use model::Model;
