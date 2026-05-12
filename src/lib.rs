//! Structural LSP for SudoLang.
//!
//! Built on tower-lsp and tree-sitter-sudolang. The library exposes the
//! pieces (diagnostics, formatter, document state) used by the binary
//! entry point in `main.rs` and by integration tests.

pub mod completion;
pub mod definition;
pub mod diagnostics;
pub mod document;
pub mod formatter;
pub mod hover;
pub mod server;
pub mod symbols;

pub use server::Backend;

use tree_sitter::Language;

pub fn sudolang_language() -> Language {
    tree_sitter_sudolang::LANGUAGE.into()
}
