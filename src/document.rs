//! Tracked document state.
//!
//! Every document is a set of SudoLang *blocks*. A pure `.sudo` file is
//! one block covering the whole text; a markdown file (`.md`,
//! `.sudo.md`) contributes one block per ```` ```sudo ```` fence. All
//! host↔block position mapping lives here — the feature modules
//! (diagnostics, formatter, hover, completion, definition) stay
//! single-tree and block-agnostic.
//!
//! Fences open at column 0, so mapping is a pure line offset.

use tower_lsp::lsp_types::{
    CompletionItem, Diagnostic, Hover, Location, Position, Range, Url,
};
use tree_sitter::{Parser, Tree};

use crate::{completion, definition, diagnostics, formatter, hover, markdown, symbols};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentKind {
    /// Pure SudoLang — the whole file is one program.
    Sudo,
    /// Markdown host — SudoLang lives in ```` ```sudo ```` fences.
    Markdown,
}

/// One parsed SudoLang region of the host document.
pub struct Block {
    /// 0-based host line where the block's text begins.
    pub start_line: u32,
    /// Number of lines the block occupies in the host document.
    pub line_count: u32,
    pub text: String,
    pub tree: Tree,
}

pub struct Document {
    pub version: i32,
    pub kind: DocumentKind,
    pub text: String,
    pub blocks: Vec<Block>,
}

/// Outcome of a whole-document format request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatResult {
    /// Parse errors make block ranges unreliable — decline.
    Refused,
    /// Nothing to change.
    Unchanged,
    /// The full replacement text for the host document.
    Formatted(String),
}

fn parse(text: &str) -> Option<Tree> {
    let mut parser = Parser::new();
    parser
        .set_language(&crate::sudolang_language())
        .expect("loading SudoLang grammar should not fail");
    parser.parse(text, None)
}

impl Document {
    /// Pure `.sudo` document (kept for callers/tests that predate
    /// markdown support).
    pub fn new(text: String, version: i32) -> Option<Self> {
        Self::with_kind(DocumentKind::Sudo, text, version)
    }

    /// Kind chosen from the file path: `.md` / `.markdown` hosts get
    /// fence extraction, everything else is pure SudoLang.
    pub fn for_path(path: &str, text: String, version: i32) -> Option<Self> {
        let kind = if markdown::is_markdown_path(path) {
            DocumentKind::Markdown
        } else {
            DocumentKind::Sudo
        };
        Self::with_kind(kind, text, version)
    }

    pub fn with_kind(kind: DocumentKind, text: String, version: i32) -> Option<Self> {
        let blocks = match kind {
            DocumentKind::Sudo => {
                let tree = parse(&text)?;
                vec![Block {
                    start_line: 0,
                    line_count: text.split('\n').count() as u32,
                    text: text.clone(),
                    tree,
                }]
            }
            DocumentKind::Markdown => markdown::extract_fences(&text)
                .into_iter()
                .filter_map(|f| {
                    let tree = parse(&f.text)?;
                    Some(Block {
                        start_line: f.start_line,
                        line_count: f.line_count,
                        text: f.text,
                        tree,
                    })
                })
                .collect(),
        };
        Some(Self {
            version,
            kind,
            text,
            blocks,
        })
    }

    pub fn update(&mut self, text: String, version: i32) {
        if let Some(doc) = Self::with_kind(self.kind, text, version) {
            *self = doc;
        }
    }

    // ---- position mapping -------------------------------------------

    fn block_at(&self, pos: Position) -> Option<(&Block, Position)> {
        self.blocks
            .iter()
            .find(|b| pos.line >= b.start_line && pos.line < b.start_line + b.line_count)
            .map(|b| (b, Position::new(pos.line - b.start_line, pos.character)))
    }

    fn lift_position(block: &Block, pos: Position) -> Position {
        Position::new(pos.line + block.start_line, pos.character)
    }

    fn lift_range(block: &Block, range: Range) -> Range {
        Range {
            start: Self::lift_position(block, range.start),
            end: Self::lift_position(block, range.end),
        }
    }

    // ---- features ----------------------------------------------------

    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        let mut out = Vec::new();
        for b in &self.blocks {
            for mut d in diagnostics::collect(&b.tree, &b.text) {
                d.range = Self::lift_range(b, d.range);
                out.push(d);
            }
            for mut d in diagnostics::placeholder_misuse(&b.tree, &b.text) {
                d.range = Self::lift_range(b, d.range);
                out.push(d);
            }
        }
        out
    }

    pub fn hover(&self, pos: Position) -> Option<Hover> {
        let (block, local) = self.block_at(pos)?;
        // Symbols from every block — the fences of one document form one
        // program. Hover only reads names/kinds/signatures, so the
        // block-local ranges inside are harmless.
        let syms: Vec<symbols::Symbol> = self
            .blocks
            .iter()
            .flat_map(|b| symbols::collect(&b.tree, &b.text))
            .collect();
        let mut h = hover::hover_with(&block.tree, &block.text, local, &syms)?;
        h.range = h.range.map(|r| Self::lift_range(block, r));
        Some(h)
    }

    /// Completions at `pos`. In markdown, only positions inside a fence
    /// complete — but symbols are gathered from *every* fence, since the
    /// fences of one document form one program.
    pub fn completions(&self, pos: Position) -> Option<Vec<CompletionItem>> {
        if self.kind == DocumentKind::Markdown {
            self.block_at(pos)?;
        }
        Some(completion::complete_many(
            self.blocks.iter().map(|b| (&b.tree, b.text.as_str())),
        ))
    }

    /// Definitions of the symbol at `pos`, searched across all blocks.
    pub fn definitions(&self, uri: &Url, pos: Position) -> Vec<Location> {
        let Some((block, local)) = self.block_at(pos) else {
            return Vec::new();
        };
        let Some(target) = definition::target_at(&block.tree, &block.text, local) else {
            return Vec::new();
        };
        let clicked = Self::lift_range(block, target.range);

        let mut out = Vec::new();
        for b in &self.blocks {
            for sym in symbols::collect(&b.tree, &b.text) {
                if !definition::symbol_matches(&sym, &target) {
                    continue;
                }
                let range = Self::lift_range(b, sym.name_range);
                if range == clicked {
                    continue; // where the user clicked, not a destination
                }
                out.push(Location {
                    uri: uri.clone(),
                    range,
                });
            }
        }
        out
    }

    /// Format the document. Pure files refuse when the tree has errors.
    /// Markdown formats every cleanly-parsing fence in place and leaves
    /// broken fences (and all prose) untouched; it refuses only when
    /// every fence is broken.
    pub fn format(&self) -> FormatResult {
        match self.kind {
            DocumentKind::Sudo => {
                let block = &self.blocks[0];
                if formatter::tree_has_errors(&block.tree) {
                    return FormatResult::Refused;
                }
                let formatted = formatter::format(&block.text, &block.tree);
                if formatted == self.text {
                    FormatResult::Unchanged
                } else {
                    FormatResult::Formatted(formatted)
                }
            }
            DocumentKind::Markdown => {
                if self.blocks.is_empty() {
                    return FormatResult::Unchanged;
                }
                if self
                    .blocks
                    .iter()
                    .all(|b| formatter::tree_has_errors(&b.tree))
                {
                    return FormatResult::Refused;
                }
                let spliced = self.splice_formatted_blocks();
                if spliced == self.text {
                    FormatResult::Unchanged
                } else {
                    FormatResult::Formatted(spliced)
                }
            }
        }
    }

    fn splice_formatted_blocks(&self) -> String {
        let src_lines: Vec<&str> = self.text.split('\n').collect();
        let mut out: Vec<String> = Vec::new();
        let mut idx = 0usize;

        for b in &self.blocks {
            let start = (b.start_line as usize).min(src_lines.len());
            let end = (start + b.line_count as usize).min(src_lines.len());
            while idx < start {
                out.push(src_lines[idx].to_string());
                idx += 1;
            }
            let replacement = if formatter::tree_has_errors(&b.tree) {
                b.text.clone()
            } else {
                formatter::format(&b.text, &b.tree)
            };
            let body = replacement.trim_end_matches('\n');
            if !body.is_empty() {
                for line in body.split('\n') {
                    out.push(line.to_string());
                }
            }
            idx = end;
        }
        while idx < src_lines.len() {
            out.push(src_lines[idx].to_string());
            idx += 1;
        }
        out.join("\n")
    }
}
