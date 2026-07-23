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

/// 2.2 §3.6 — `_` is the pipe placeholder: inside a pipe stage it means
/// "the piped value"; anywhere else it has no subject. The grammar
/// accepts it everywhere (it lexes as a plain identifier); the LSP owns
/// the misuse diagnostic. Declaration positions (parameters, patterns)
/// are exempt — `_` is the conventional discard there.
pub fn placeholder_misuse(tree: &Tree, source: &str) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    let mut cursor = tree.walk();
    visit_placeholders(&mut cursor, source, &mut out);
    out
}

fn visit_placeholders(cursor: &mut TreeCursor, source: &str, out: &mut Vec<Diagnostic>) {
    let node = cursor.node();
    if node.kind() == "identifier"
        && node_text(node, source) == "_"
        && !in_pipe_stage(node)
        && !in_declaration_position(node)
    {
        out.push(Diagnostic {
            range: node_range(node, source),
            severity: Some(DiagnosticSeverity::WARNING),
            source: Some("sudolang-lsp".into()),
            message: "`_` is the pipe placeholder — outside a pipe stage it has no \
                      subject. Use it in `value |> stage(_.field)` positions."
                .into(),
            ..Default::default()
        });
    }
    if cursor.goto_first_child() {
        loop {
            visit_placeholders(cursor, source, out);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

/// True when some ancestor edge passes through the right-hand side of a
/// `pipe_expression` — i.e. the node is inside a pipe stage.
fn in_pipe_stage(node: Node) -> bool {
    let mut child = node;
    while let Some(parent) = child.parent() {
        if parent.kind() == "pipe_expression" {
            if let Some(right) = parent.child_by_field_name("right") {
                if right.start_byte() <= child.start_byte()
                    && child.end_byte() <= right.end_byte()
                {
                    return true;
                }
            }
        }
        child = parent;
    }
    false
}

fn in_declaration_position(node: Node) -> bool {
    let mut current = node;
    while let Some(parent) = current.parent() {
        match parent.kind() {
            "parameter" | "array_pattern" | "object_pattern" | "rest_pattern" => return true,
            _ => {}
        }
        current = parent;
    }
    false
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

/// Convert an LSP `Position` (line + UTF-16 column) into a byte offset.
/// Returns `None` if the position is past end-of-document. Inverse of
/// [`byte_to_position`].
pub fn position_to_byte(source: &str, pos: Position) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut line = 0u32;
    let mut byte = 0usize;

    while line < pos.line {
        let b = *bytes.get(byte)?;
        byte += 1;
        if b == b'\n' {
            line += 1;
        }
    }

    let mut utf16 = 0u32;
    let line_end = source[byte..]
        .find('\n')
        .map(|n| byte + n)
        .unwrap_or(source.len());
    let mut i = byte;
    while utf16 < pos.character && i < line_end {
        let ch = source[i..].chars().next()?;
        utf16 += ch.len_utf16() as u32;
        i += ch.len_utf8();
    }
    Some(i)
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
