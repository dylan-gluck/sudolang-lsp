use sudolang_lsp::{diagnostics, sudolang_language};
use tree_sitter::Parser;

fn parse(src: &str) -> tree_sitter::Tree {
    let mut p = Parser::new();
    p.set_language(&sudolang_language()).unwrap();
    p.parse(src, None).unwrap()
}

#[test]
fn clean_source_has_no_diagnostics() {
    let src = "Foo {\n  bar = 1\n}\n";
    let tree = parse(src);
    let diags = diagnostics::collect(&tree, src);
    assert!(diags.is_empty(), "unexpected diagnostics: {:?}", diags);
}

#[test]
fn unbalanced_brace_reports_a_diagnostic() {
    let src = "Foo {\n  bar = 1\n";
    let tree = parse(src);
    let diags = diagnostics::collect(&tree, src);
    assert!(
        !diags.is_empty(),
        "expected at least one diagnostic for unbalanced brace"
    );
}

#[test]
fn broken_interpolation_reports_a_diagnostic() {
    // `${` without closing `}` and without a following expression.
    let src = "x = \"hello ${\"\n";
    let tree = parse(src);
    let diags = diagnostics::collect(&tree, src);
    assert!(
        !diags.is_empty(),
        "expected diagnostic for broken interpolation"
    );
}
