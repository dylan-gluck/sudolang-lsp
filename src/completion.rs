//! Completion provider.
//!
//! Returns a static list of SudoLang keywords plus every named
//! declaration the [`symbols`] module finds in the current document.
//! No fuzzy ranking and no context-sensitivity yet — Zed (and most LSP
//! clients) will filter by prefix on their side.
//!
//! [`symbols`]: crate::symbols

use std::collections::HashSet;

use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, Documentation, MarkupContent, MarkupKind,
};
use tree_sitter::Tree;

use crate::symbols::{self, SymbolKind};

const KEYWORDS: &[&str] = &[
    "fn",
    "function",
    "interface",
    "match",
    "case",
    "default",
    "if",
    "else",
    "for",
    "each",
    "in",
    "while",
    "loop",
    "return",
    "throw",
    "try",
    "catch",
    "require",
    "warn",
    "constraint",
];

pub fn complete(tree: &Tree, source: &str) -> Vec<CompletionItem> {
    complete_many(std::iter::once((tree, source)))
}

/// Completion over a set of SudoLang blocks (the fences of one markdown
/// document form one program — symbols from every block are offered).
pub fn complete_many<'a>(
    blocks: impl IntoIterator<Item = (&'a Tree, &'a str)>,
) -> Vec<CompletionItem> {
    let mut items = Vec::with_capacity(KEYWORDS.len() + 16);
    let mut seen: HashSet<String> = HashSet::new();

    for kw in KEYWORDS {
        items.push(CompletionItem {
            label: (*kw).into(),
            kind: Some(CompletionItemKind::KEYWORD),
            ..Default::default()
        });
        seen.insert((*kw).into());
    }

    for (tree, source) in blocks {
        for sym in symbols::collect(tree, source) {
            // Skip duplicates — multiple parameters in different functions
            // may share names; one entry is enough.
            let key = format!("{}::{}", sym.kind.label(), sym.name);
            if !seen.insert(key) {
                continue;
            }
            items.push(CompletionItem {
                label: sym.name.clone(),
                kind: Some(lsp_kind(sym.kind)),
                detail: Some(sym.kind.label().into()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: format!("```sudo\n{}\n```", sym.signature),
                })),
                ..Default::default()
            });
        }

        // Capability namespaces (2.2): every qualified path already used
        // in the document completes as a unit — `mcp::linear`, `git::…`.
        for path in qualified_paths(tree, source) {
            if !seen.insert(format!("capability::{path}")) {
                continue;
            }
            items.push(CompletionItem {
                label: path,
                kind: Some(CompletionItemKind::MODULE),
                detail: Some("capability".into()),
                ..Default::default()
            });
        }
    }

    items
}

/// Collect the distinct `qualified_identifier` spellings in a tree.
fn qualified_paths(tree: &Tree, source: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cursor = tree.walk();
    collect_qualified(&mut cursor, source, &mut out);
    out
}

fn collect_qualified(
    cursor: &mut tree_sitter::TreeCursor,
    source: &str,
    out: &mut Vec<String>,
) {
    let node = cursor.node();
    if node.kind() == "qualified_identifier" {
        let start = node.start_byte().min(source.len());
        let end = node.end_byte().min(source.len());
        out.push(source[start..end].to_string());
    }
    if cursor.goto_first_child() {
        loop {
            collect_qualified(cursor, source, out);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

fn lsp_kind(kind: SymbolKind) -> CompletionItemKind {
    match kind {
        SymbolKind::Function => CompletionItemKind::FUNCTION,
        SymbolKind::Interface => CompletionItemKind::INTERFACE,
        SymbolKind::Constraint => CompletionItemKind::CLASS,
        SymbolKind::Property => CompletionItemKind::PROPERTY,
        SymbolKind::Variable => CompletionItemKind::VARIABLE,
        SymbolKind::Parameter => CompletionItemKind::VARIABLE,
        SymbolKind::Command => CompletionItemKind::METHOD,
    }
}
