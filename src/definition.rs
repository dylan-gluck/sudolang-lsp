//! Goto-definition.
//!
//! Resolves the identifier (or `command_name`) at the cursor to the
//! locations where it is declared in the document. Cross-file resolution
//! is not implemented — SudoLang has no module system to anchor it on.

use tower_lsp::lsp_types::{Location, Position, Url};
use tree_sitter::{Node, Tree};

use crate::diagnostics::position_to_byte;
use crate::symbols::{self, Symbol};

pub fn definitions(tree: &Tree, source: &str, uri: &Url, position: Position) -> Vec<Location> {
    let Some(byte) = position_to_byte(source, position) else {
        return Vec::new();
    };
    let Some(node) = identifier_or_command_at(tree, byte) else {
        return Vec::new();
    };
    let name = node_text(node, source);
    if name.is_empty() {
        return Vec::new();
    }

    let syms = symbols::collect(tree, source);
    let is_command = node.kind() == "command_name";

    syms.into_iter()
        .filter(|s| {
            if s.name != name {
                return false;
            }
            if is_command {
                matches!(s.kind, crate::symbols::SymbolKind::Command)
            } else {
                !matches!(s.kind, crate::symbols::SymbolKind::Command)
            }
        })
        // Don't return the user's own cursor position as the destination —
        // that's just where they clicked.
        .filter(|s| !is_same_range(s, node, source))
        .map(|s: Symbol| Location {
            uri: uri.clone(),
            range: s.name_range,
        })
        .collect()
}

fn identifier_or_command_at(tree: &Tree, byte: usize) -> Option<Node<'_>> {
    let mut node = tree.root_node().descendant_for_byte_range(byte, byte)?;
    while !node.is_named() {
        node = node.parent()?;
    }
    if node.kind() == "identifier" || node.kind() == "command_name" {
        return Some(node);
    }
    // Sometimes the descendant is a finer-grained token; try parent once.
    let parent = node.parent()?;
    if parent.kind() == "identifier" || parent.kind() == "command_name" {
        Some(parent)
    } else {
        None
    }
}

fn is_same_range(sym: &Symbol, node: Node, source: &str) -> bool {
    let node_r = crate::diagnostics::node_range(node, source);
    sym.name_range == node_r
}

fn node_text<'a>(node: Node, source: &'a str) -> &'a str {
    let start = node.start_byte().min(source.len());
    let end = node.end_byte().min(source.len());
    &source[start..end]
}
