use sudolang_lsp::{hover, sudolang_language};
use tower_lsp::lsp_types::{HoverContents, MarkupKind, Position};
use tree_sitter::Parser;

fn parse(src: &str) -> tree_sitter::Tree {
    let mut p = Parser::new();
    p.set_language(&sudolang_language()).unwrap();
    p.parse(src, None).unwrap()
}

fn body(h: &tower_lsp::lsp_types::Hover) -> &str {
    match &h.contents {
        HoverContents::Markup(m) => {
            assert_eq!(m.kind, MarkupKind::Markdown);
            &m.value
        }
        _ => panic!("expected markup hover"),
    }
}

#[test]
fn hover_on_function_identifier_returns_signature() {
    let src = "fn greet(name) {\n  return name\n}\ngreet(\"world\")\n";
    let tree = parse(src);
    // cursor on `greet` in the call on line 3, column 0
    let h = hover::hover(&tree, src, Position::new(3, 1)).expect("hover");
    let b = body(&h);
    assert!(b.contains("function"), "got: {b}");
    assert!(b.contains("greet"), "got: {b}");
}

#[test]
fn hover_on_keyword_returns_blurb() {
    let src = "fn greet() {}\n";
    let tree = parse(src);
    let h = hover::hover(&tree, src, Position::new(0, 0)).expect("hover on `fn`");
    let b = body(&h);
    assert!(b.to_lowercase().contains("function"), "got: {b}");
}

#[test]
fn hover_on_unknown_identifier_is_none() {
    let src = "doesNotExist\n";
    let tree = parse(src);
    let h = hover::hover(&tree, src, Position::new(0, 2));
    assert!(h.is_none(), "expected None hover, got: {:?}", h);
}

#[test]
fn hover_on_command_name_returns_command_info() {
    // `/welcome` is declared inside an interface, then invoked at top level.
    let src = "Bot {\n  /welcome - greet the user\n}\n/welcome\n";
    let tree = parse(src);
    let h = hover::hover(&tree, src, Position::new(3, 2)).expect("hover on command");
    let b = body(&h);
    assert!(b.contains("command"), "got: {b}");
    assert!(b.contains("/welcome"), "got: {b}");
}
