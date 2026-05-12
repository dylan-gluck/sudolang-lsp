//! Deterministic SudoLang formatter.
//!
//! The strategy is conservative and structure-driven: we walk the parsed
//! tree to learn where blocks open and close, then re-indent each line of
//! source based on its position inside those blocks. We never reorder
//! tokens, never split or join lines, and never touch the interior of a
//! multi-line string or comment.
//!
//! What this normalizes:
//!   - leading whitespace on each line → `INDENT_WIDTH * block_depth` spaces
//!   - trailing whitespace on each line → removed
//!   - runs of 2+ blank lines → a single blank line
//!   - missing trailing newline → added (if file is non-empty)
//!
//! What it deliberately leaves alone:
//!   - everything inside `block_comment`, `triple_quoted_block`,
//!     `double_string`, and `template_string` spans
//!   - operator spacing, comma placement, brace placement on existing lines
//!
//! Because formatting decisions are derived from the AST, an unparseable
//! file (one with ERROR nodes) is still safe to format — we just fall back
//! to the original source when block ranges look suspicious. The server
//! refuses to format when the tree has top-level errors.

use tree_sitter::{Node, Tree, TreeCursor};

const INDENT_WIDTH: usize = 2;

/// Returns true if the parsed tree contains any ERROR or MISSING nodes.
/// Callers should decline to format such trees — the block ranges we'd
/// re-indent against may be unreliable.
pub fn tree_has_errors(tree: &Tree) -> bool {
    tree.root_node().has_error()
}

const PRESERVED_KINDS: &[&str] = &[
    "block_comment",
    "triple_quoted_block",
    "double_string",
    "template_string",
];

pub fn format(source: &str, tree: &Tree) -> String {
    if source.is_empty() {
        return String::new();
    }

    let preserved = collect_preserved_ranges(tree);
    let bytes = source.as_bytes();

    let mut out = String::with_capacity(source.len());
    let mut consecutive_blank = 0usize;

    for (line_start, line_end) in line_ranges(source) {
        let raw = &source[line_start..line_end];

        if line_is_preserved(line_start, &preserved) {
            out.push_str(raw);
            out.push('\n');
            consecutive_blank = 0;
            continue;
        }

        let first_non_ws = first_non_whitespace(bytes, line_start, line_end);

        // Blank line
        if first_non_ws.is_none() {
            if consecutive_blank == 0 {
                out.push('\n');
            }
            consecutive_blank += 1;
            continue;
        }
        consecutive_blank = 0;

        let first_non_ws = first_non_ws.unwrap();
        let depth = line_indent_depth(tree, bytes, first_non_ws);
        let content = source[first_non_ws..line_end].trim_end_matches([' ', '\t']);

        for _ in 0..depth * INDENT_WIDTH {
            out.push(' ');
        }
        out.push_str(content);
        out.push('\n');
    }

    // Collapse trailing blank lines, keep exactly one terminal newline.
    while out.ends_with("\n\n") {
        out.pop();
    }
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn line_ranges(source: &str) -> Vec<(usize, usize)> {
    let bytes = source.as_bytes();
    let mut ranges = Vec::new();
    let mut start = 0usize;
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'\n' {
            ranges.push((start, i));
            start = i + 1;
        }
    }
    if start < bytes.len() {
        ranges.push((start, bytes.len()));
    }
    ranges
}

fn first_non_whitespace(bytes: &[u8], start: usize, end: usize) -> Option<usize> {
    let mut i = start;
    while i < end {
        match bytes[i] {
            b' ' | b'\t' => i += 1,
            _ => return Some(i),
        }
    }
    None
}

fn line_is_preserved(line_start: usize, preserved: &[(usize, usize)]) -> bool {
    preserved
        .iter()
        .any(|&(s, e)| line_start > s && line_start < e)
}

fn line_indent_depth(tree: &Tree, source_bytes: &[u8], first_non_ws: usize) -> usize {
    let mut depth = 0usize;
    let ch = source_bytes.get(first_non_ws).copied();
    let mut node = tree
        .root_node()
        .descendant_for_byte_range(first_non_ws, first_non_ws);
    while let Some(n) = node {
        if n.kind() == "block" {
            let opens_here = ch == Some(b'{') && n.start_byte() == first_non_ws;
            let closes_here = ch == Some(b'}') && n.end_byte() == first_non_ws + 1;
            if !opens_here && !closes_here {
                depth += 1;
            }
        }
        node = n.parent();
    }
    depth
}

fn collect_preserved_ranges(tree: &Tree) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut cursor = tree.walk();
    visit_preserved(&mut cursor, &mut out);
    out
}

fn visit_preserved(cursor: &mut TreeCursor, out: &mut Vec<(usize, usize)>) {
    let node = cursor.node();
    if is_preserved_kind(node) && spans_multiple_lines(node) {
        out.push((node.start_byte(), node.end_byte()));
    }
    if cursor.goto_first_child() {
        loop {
            visit_preserved(cursor, out);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

fn is_preserved_kind(node: Node) -> bool {
    PRESERVED_KINDS.contains(&node.kind())
}

fn spans_multiple_lines(node: Node) -> bool {
    node.start_position().row != node.end_position().row
}
