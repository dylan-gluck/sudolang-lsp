//! Document-symbol collector.
//!
//! Walks the parse tree once and surfaces the named declarations that
//! hover / completion / definition all need:
//!
//!   - `function_declaration`      → SymbolKind::Function
//!   - `interface_declaration`     → SymbolKind::Interface
//!   - `constraint_block`          → SymbolKind::Constraint  (named ones only)
//!   - `property_declaration`      → SymbolKind::Property
//!   - `assignment` (target ident) → SymbolKind::Variable
//!   - `parameter`                 → SymbolKind::Parameter
//!   - `command_declaration`       → SymbolKind::Command
//!
//! Each entry records the *name* range (the identifier itself) and the
//! *full* range (the whole declaration). Goto-definition jumps to the
//! name range; hover renders a one-line signature taken from the full
//! range.

use tower_lsp::lsp_types::Range;
use tree_sitter::{Node, Tree, TreeCursor};

use crate::diagnostics::node_range;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Function,
    Interface,
    Constraint,
    Property,
    Variable,
    Parameter,
    Command,
}

impl SymbolKind {
    pub fn label(self) -> &'static str {
        match self {
            SymbolKind::Function => "function",
            SymbolKind::Interface => "interface",
            SymbolKind::Constraint => "constraint",
            SymbolKind::Property => "property",
            SymbolKind::Variable => "variable",
            SymbolKind::Parameter => "parameter",
            SymbolKind::Command => "command",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub name_range: Range,
    pub full_range: Range,
    pub signature: String,
}

pub fn collect(tree: &Tree, source: &str) -> Vec<Symbol> {
    let mut out = Vec::new();
    let mut cursor = tree.walk();
    visit(&mut cursor, source, &mut out);
    out
}

fn visit(cursor: &mut TreeCursor, source: &str, out: &mut Vec<Symbol>) {
    let node = cursor.node();
    if let Some(sym) = symbol_for(node, source) {
        out.push(sym);
    }
    if cursor.goto_first_child() {
        loop {
            visit(cursor, source, out);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

fn symbol_for(node: Node, source: &str) -> Option<Symbol> {
    match node.kind() {
        "function_declaration" => named_child(node, "name").map(|name| Symbol {
            name: text(name, source).to_string(),
            kind: SymbolKind::Function,
            name_range: node_range(name, source),
            full_range: node_range(node, source),
            signature: first_line(text(node, source)),
        }),
        "interface_declaration" => named_child(node, "name").map(|name| Symbol {
            name: text(name, source).to_string(),
            kind: SymbolKind::Interface,
            name_range: node_range(name, source),
            full_range: node_range(node, source),
            signature: first_line(text(node, source)),
        }),
        "constraint_block" => named_child(node, "name").map(|name| Symbol {
            name: text(name, source).to_string(),
            kind: SymbolKind::Constraint,
            name_range: node_range(name, source),
            full_range: node_range(node, source),
            signature: first_line(text(node, source)),
        }),
        "property_declaration" => named_child(node, "name").map(|name| Symbol {
            name: text(name, source).to_string(),
            kind: SymbolKind::Property,
            name_range: node_range(name, source),
            full_range: node_range(node, source),
            signature: first_line(text(node, source)),
        }),
        "assignment" => {
            let target = named_child(node, "target")?;
            if target.kind() != "identifier" {
                return None;
            }
            Some(Symbol {
                name: text(target, source).to_string(),
                kind: SymbolKind::Variable,
                name_range: node_range(target, source),
                full_range: node_range(node, source),
                signature: first_line(text(node, source)),
            })
        }
        "parameter" => {
            let name = named_child(node, "name").or_else(|| named_child(node, "pattern"))?;
            if name.kind() != "identifier" {
                return None;
            }
            Some(Symbol {
                name: text(name, source).to_string(),
                kind: SymbolKind::Parameter,
                name_range: node_range(name, source),
                full_range: node_range(node, source),
                signature: first_line(text(node, source)),
            })
        }
        "command_declaration" => {
            let cmd = named_child(node, "command")?;
            Some(Symbol {
                name: text(cmd, source).to_string(),
                kind: SymbolKind::Command,
                name_range: node_range(cmd, source),
                full_range: node_range(node, source),
                signature: first_line(text(node, source)),
            })
        }
        _ => None,
    }
}

fn named_child<'tree>(node: Node<'tree>, field: &str) -> Option<Node<'tree>> {
    node.child_by_field_name(field)
}

fn text<'a>(node: Node, source: &'a str) -> &'a str {
    let start = node.start_byte().min(source.len());
    let end = node.end_byte().min(source.len());
    &source[start..end]
}

fn first_line(s: &str) -> String {
    let line = s.lines().next().unwrap_or("").trim_end();
    if line.chars().count() > 120 {
        let take: String = line.chars().take(117).collect();
        format!("{take}…")
    } else {
        line.to_string()
    }
}
