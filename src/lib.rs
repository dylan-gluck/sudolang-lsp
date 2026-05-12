//! Structural LSP for SudoLang.
//!
//! Built on tower-lsp and tree-sitter-sudolang. The library exposes the
//! pieces (diagnostics, formatter, document state) used by the binary
//! entry point in `main.rs` and by integration tests.

pub mod diagnostics;
pub mod document;
pub mod formatter;
pub mod server;

pub use server::Backend;

use tree_sitter::Language;

pub fn sudolang_language() -> Language {
    tree_sitter_sudolang::LANGUAGE.into()
}
