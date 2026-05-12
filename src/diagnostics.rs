use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};
use tree_sitter::{Node, Tree, TreeCursor};

/// Walk the parse tree and emit diagnostics for:
/// - syntactic errors (tree-sitter ERROR nodes)
/// - missing tokens (tree-sitter MISSING — unbalanced braces, missing `}` in
///   `${...}` interpolations, etc.)
/// - malformed modifiers (modifier_list nodes containing an error)
/// - broken string interpolations (string_interpolation containing an error
///   or missing token)
pub fn collect(tree: &Tree, source: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut cursor = tree.walk();
    visit(&mut cursor, source, &mut diagnostics);
    diagnostics
}

fn visit(cursor: &mut TreeCursor, source: &str, out: &mut Vec<Diagnostic>) {
    let node = cursor.node();

    if node.is_missing() {
        out.push(missing_diagnostic(node, source));
    } else if node.is_error() {
        out.push(error_diagnostic(node, source));
    } else {
        match node.kind() {
            "modifier_list" if node_has_error(node) => {
                out.push(malformed_modifier_diagnostic(node, source));
            }
            "string_interpolation" if node_has_error(node) => {
                out.push(broken_interpolation_diagnostic(node, source));
            }
            _ => {}
        }
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

fn node_has_error(node: Node) -> bool {
    node.has_error()
}

fn missing_diagnostic(node: Node, source: &str) -> Diagnostic {
    let kind = node.kind();
    let message = if kind.is_empty() {
        "Missing token".to_string()
    } else {
        format!("Missing `{kind}`")
    };
    Diagnostic {
        range: node_range(node, source),
        severity: Some(DiagnosticSeverity::ERROR),
        source: Some("sudolang-lsp".into()),
        message,
        ..Default::default()
    }
}

fn error_diagnostic(node: Node, source: &str) -> Diagnostic {
    let snippet = node_text(node, source);
    let preview = preview_for_message(&snippet);
    let message = if preview.is_empty() {
        "Syntax error".to_string()
    } else {
        format!("Syntax error near `{preview}`")
    };
    Diagnostic {
        range: node_range(node, source),
        severity: Some(DiagnosticSeverity::ERROR),
        source: Some("sudolang-lsp".into()),
        message,
        ..Default::default()
    }
}

fn malformed_modifier_diagnostic(node: Node, source: &str) -> Diagnostic {
    Diagnostic {
        range: node_range(node, source),
        severity: Some(DiagnosticSeverity::ERROR),
        source: Some("sudolang-lsp".into()),
        message: "Malformed modifier list (expected `:name=value;` pairs after the call)".into(),
        ..Default::default()
    }
}

fn broken_interpolation_diagnostic(node: Node, source: &str) -> Diagnostic {
    Diagnostic {
        range: node_range(node, source),
        severity: Some(DiagnosticSeverity::ERROR),
        source: Some("sudolang-lsp".into()),
        message: "Broken string interpolation (expected `${expression}` or `$identifier`)".into(),
        ..Default::default()
    }
}

fn preview_for_message(s: &str) -> String {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let take: String = trimmed.chars().take(24).collect();
    if trimmed.chars().count() > 24 {
        format!("{take}…")
    } else {
        take
    }
}

fn node_text<'a>(node: Node, source: &'a str) -> &'a str {
    let start = node.start_byte().min(source.len());
    let end = node.end_byte().min(source.len());
    &source[start..end]
}

pub fn node_range(node: Node, source: &str) -> Range {
    Range {
        start: byte_to_position(node.start_byte(), source),
        end: byte_to_position(node.end_byte(), source),
    }
}

/// Convert a byte offset into an LSP `Position` (line + UTF-16 column).
pub fn byte_to_position(byte: usize, source: &str) -> Position {
    let byte = byte.min(source.len());
    let mut line = 0u32;
    let mut line_start = 0usize;
    let bytes = source.as_bytes();
    let mut i = 0usize;
    while i < byte {
        if bytes[i] == b'\n' {
            line += 1;
            line_start = i + 1;
        }
        i += 1;
    }
    let line_text = &source[line_start..byte];
    let character = line_text.encode_utf16().count() as u32;
    Position { line, character }
}
