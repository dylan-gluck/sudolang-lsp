//! Deterministic SudoLang formatter.
//!
//! The strategy is conservative and structure-driven: we walk the parsed
//! tree to learn where blocks open and close, then re-indent each line of
//! source based on its position inside those blocks. We never reorder
//! tokens, never split or join lines, and never touch the interior of a
//! multi-line string or comment.
//!
//! What this normalizes:
//!   - leading whitespace on each line → `INDENT_WIDTH * indent_depth`
//!     spaces, where the depth counts the distinct rows on which the
//!     enclosing indent-bearing constructs opened (see [`INDENT_KINDS`])
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

/// Node kinds that add one indent level to the lines strictly inside them.
///
/// `block` is the obvious one. The rest are the multi-line constructs that
/// carry their own delimiters and would otherwise flatten to the depth of
/// the statement that opens them: literals and patterns (`{ }` / `[ ]`),
/// call and signature lists (`( )`), the braces of a `match` (which are not
/// a `block` node), and a pipe chain broken across lines.
const INDENT_KINDS: &[&str] = &[
    "block",
    "object_literal",
    "array_literal",
    "object_pattern",
    "array_pattern",
    "argument_list",
    "parameter_list",
    "match_expression",
    "pipe_expression",
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
        let depth = line_indent_depth(tree, bytes, first_non_ws, line_end);
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

/// Indent depth for the line whose first non-whitespace byte is
/// `first_non_ws`.
///
/// We walk the ancestor chain and collect the *rows on which each
/// enclosing construct opened*, then count the distinct rows. Counting
/// rows rather than nodes is what keeps stacked openers honest: in
///
/// ```text
/// describe("unit", () => {
///   assert()
/// })
/// ```
///
/// the `argument_list` and the `block` both open on row 0, so `assert()`
/// gets one indent level, not two. A construct that opens on its own row
/// still contributes its own level.
///
/// A node is skipped when the line *is* its opening delimiter or its
/// closing delimiter, so `{` and `}` sit at the depth of their parent.
/// Closers stack the same way openers do — on a `})` line both the block
/// and the argument list are closing, so both are skipped.
fn line_indent_depth(
    tree: &Tree,
    source_bytes: &[u8],
    first_non_ws: usize,
    line_end: usize,
) -> usize {
    let mut opener_rows: Vec<usize> = Vec::new();
    let mut node = tree
        .root_node()
        .descendant_for_byte_range(first_non_ws, first_non_ws);
    while let Some(n) = node {
        if INDENT_KINDS.contains(&n.kind()) && spans_multiple_lines(n) {
            if !opens_line(n, first_non_ws) && !closes_line(n, source_bytes, first_non_ws, line_end)
            {
                opener_rows.push(n.start_position().row);
            }
        }
        node = n.parent();
    }
    opener_rows.sort_unstable();
    opener_rows.dedup();
    opener_rows.len()
}

/// The line begins with this node's opening delimiter.
fn opens_line(node: Node, first_non_ws: usize) -> bool {
    node.start_byte() == first_non_ws
}

/// This line is the node's closing delimiter: the node ends on this line,
/// and everything from the first non-whitespace byte through that closing
/// delimiter is itself a closer or separator. That admits `}`, `})`, `},`
/// and `}]` while rejecting a continuation line that merely happens to
/// contain the node's last byte — `|> score |> takeTop(3)` ends a pipe
/// expression but is not a closing delimiter line.
fn closes_line(node: Node, source: &[u8], first_non_ws: usize, line_end: usize) -> bool {
    let Some(close) = node.end_byte().checked_sub(1) else {
        return false;
    };
    if close < first_non_ws || close >= line_end {
        return false;
    }
    source[first_non_ws..=close]
        .iter()
        .all(|b| matches!(b, b'}' | b']' | b')' | b',' | b';' | b' ' | b'\t'))
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
