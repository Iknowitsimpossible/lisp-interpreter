//! A Lisp tokenizer, bytecode compiler, and virtual machine.
//! The public API returns rendered results, so managed environment handles never escape.
mod syntax;
mod vm;

pub use syntax::{inspect_ast, inspect_tokens, Error};
pub use vm::{Execution, Interpreter};
