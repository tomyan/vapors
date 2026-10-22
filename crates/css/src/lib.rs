//! The CSS engine: a stylesheet goes in as text and comes out as a cascade
//! that, given an element tree, yields the computed value of every
//! supported property for every element.
//!
//! Written fresh against sumi's test corpus (`~/projects/sumi`), module by
//! module. Nothing here reads files: stylesheets arrive as strings, so the
//! crate compiles for wasm32.

pub mod ast;
pub mod cascade;
pub mod error;
pub mod parse;
pub mod property;
pub mod selector;
pub mod serialize;
pub mod syntax;
pub mod token;
pub mod tree;
pub mod value;

pub use error::{Error, Position, Result};
pub use parse::parse;
