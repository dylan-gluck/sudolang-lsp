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

    items
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
