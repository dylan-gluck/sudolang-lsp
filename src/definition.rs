//! Goto-definition.
//!
//! Resolves the identifier (or `command_name`) at the cursor to the
//! locations where it is declared in the document. Cross-file resolution
//! is not implemented — SudoLang has no module system to anchor it on.

use tower_lsp::lsp_types::{Location, Position, Url};
use tree_sitter::{Node, Tree};

use crate::diagnostics::position_to_byte;
use crate::symbols::{self, Symbol};

/// The symbol reference under the cursor: what to look up, and where the
/// cursor's own node sits (so callers can exclude it from results).
pub struct Target {
    pub name: String,
    pub is_command: bool,
    pub range: tower_lsp::lsp_types::Range,
}

pub fn target_at(tree: &Tree, source: &str, position: Position) -> Option<Target> {
    let byte = position_to_byte(source, position)?;
    let node = identifier_or_command_at(tree, byte)?;
    let name = node_text(node, source);
    if name.is_empty() {
        return None;
    }
    Some(Target {
        name: name.to_string(),
        is_command: node.kind() == "command_name",
        range: crate::diagnostics::node_range(node, source),
    })
}

pub fn symbol_matches(sym: &Symbol, target: &Target) -> bool {
    if sym.name != target.name {
        return false;
    }
    if target.is_command {
        matches!(sym.kind, crate::symbols::SymbolKind::Command)
    } else {
        !matches!(sym.kind, crate::symbols::SymbolKind::Command)
    }
}

pub fn definitions(tree: &Tree, source: &str, uri: &Url, position: Position) -> Vec<Location> {
    let Some(target) = target_at(tree, source, position) else {
        return Vec::new();
    };

    symbols::collect(tree, source)
        .into_iter()
        .filter(|s| symbol_matches(s, &target))
        // Don't return the user's own cursor position as the destination —
        // that's just where they clicked.
        .filter(|s| s.name_range != target.range)
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

fn node_text<'a>(node: Node, source: &'a str) -> &'a str {
    let start = node.start_byte().min(source.len());
    let end = node.end_byte().min(source.len());
    &source[start..end]
}
