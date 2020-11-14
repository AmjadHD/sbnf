#![deny(clippy::all)]
#![allow(clippy::comparison_chain, clippy::too_many_arguments)]

pub mod compiler;
pub mod formatter;
pub mod sbnf;
pub mod sublime_syntax;
mod symbol_table;
mod util;
