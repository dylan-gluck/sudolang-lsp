use sudolang_lsp::{definition, sudolang_language};
use tower_lsp::lsp_types::{Position, Url};
use tree_sitter::Parser;

fn parse(src: &str) -> tree_sitter::Tree {
    let mut p = Parser::new();
    p.set_language(&sudolang_language()).unwrap();
    p.parse(src, None).unwrap()
}

fn uri() -> Url {
    Url::parse("file:///test.sudo").unwrap()
}

#[test]
fn definition_resolves_function_call_to_declaration() {
    let src = "fn greet(name) {\n  return name\n}\ngreet(\"world\")\n";
    let tree = parse(src);
    // cursor on the `g` of `greet(` on line 3
    let locs = definition::definitions(&tree, src, &uri(), Position::new(3, 1));
    assert_eq!(locs.len(), 1, "expected 1 definition, got {:?}", locs);
    assert_eq!(locs[0].range.start.line, 0);
}

#[test]
fn definition_resolves_command_invocation() {
    let src = "Bot {\n  /welcome - greet the user\n}\n/welcome\n";
    let tree = parse(src);
    // cursor on the `w` of `/welcome` on line 3
    let locs = definition::definitions(&tree, src, &uri(), Position::new(3, 2));
    assert_eq!(locs.len(), 1, "expected 1 definition, got {:?}", locs);
    assert_eq!(locs[0].range.start.line, 1);
}

#[test]
fn definition_on_unknown_returns_empty() {
    let src = "fn greet() {}\nunknownThing\n";
    let tree = parse(src);
    let locs = definition::definitions(&tree, src, &uri(), Position::new(1, 2));
    assert!(locs.is_empty(), "expected no definitions, got {:?}", locs);
}

#[test]
fn definition_does_not_return_cursor_position() {
    // Clicking on the declaration itself should not jump back to itself.
    let src = "fn greet() {}\n";
    let tree = parse(src);
    let locs = definition::definitions(&tree, src, &uri(), Position::new(0, 4));
    assert!(
        locs.is_empty(),
        "clicking on the declaration should not self-target: {:?}",
        locs
    );
}
