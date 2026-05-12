use sudolang_lsp::{formatter, sudolang_language};
use tree_sitter::Parser;

fn parse(src: &str) -> tree_sitter::Tree {
    let mut p = Parser::new();
    p.set_language(&sudolang_language()).unwrap();
    p.parse(src, None).unwrap()
}

fn fmt(src: &str) -> String {
    let tree = parse(src);
    formatter::format(src, &tree)
}

#[test]
fn empty_file_stays_empty() {
    assert_eq!(fmt(""), "");
}

#[test]
fn missing_trailing_newline_is_added() {
    assert_eq!(fmt("x = 1"), "x = 1\n");
}

#[test]
fn reindents_block_body() {
    let input = "Foo {\nbar\n  baz\n}\n";
    let expected = "Foo {\n  bar\n  baz\n}\n";
    assert_eq!(fmt(input), expected);
}

#[test]
fn nested_blocks_indent_by_two() {
    let input = "Foo {\nBar {\nx\n}\n}\n";
    let expected = "Foo {\n  Bar {\n    x\n  }\n}\n";
    assert_eq!(fmt(input), expected);
}

#[test]
fn trailing_whitespace_stripped() {
    let input = "x = 1   \ny = 2\t\n";
    let expected = "x = 1\ny = 2\n";
    assert_eq!(fmt(input), expected);
}

#[test]
fn multiple_blank_lines_collapse_to_one() {
    let input = "a = 1\n\n\n\nb = 2\n";
    let expected = "a = 1\n\nb = 2\n";
    assert_eq!(fmt(input), expected);
}

#[test]
fn formatter_is_idempotent() {
    let input = include_str!("../../tree-sitter-sudolang/examples/riteway.sudo");
    let once = fmt(input);
    let twice = fmt(&once);
    assert_eq!(once, twice, "formatter must be idempotent");
}

#[test]
fn formatter_preserves_canonical_examples_under_reparse() {
    // Format each canonical example and confirm the formatted output
    // still parses cleanly (no new ERROR/MISSING nodes).
    let examples = [
        include_str!("../../tree-sitter-sudolang/examples/riteway.sudo"),
        include_str!("../../tree-sitter-sudolang/examples/autodux.sudo"),
        include_str!("../../tree-sitter-sudolang/examples/ai-rpg.sudo"),
        include_str!("../../tree-sitter-sudolang/examples/sudolang.sudo"),
        include_str!("../../tree-sitter-sudolang/examples/vector-search.sudo"),
    ];
    for src in examples {
        let formatted = fmt(src);
        let tree = parse(&formatted);
        assert!(
            !tree.root_node().has_error(),
            "formatted source no longer parses cleanly: first 200 chars:\n{}",
            &formatted.chars().take(200).collect::<String>()
        );
    }
}

#[test]
fn formatter_idempotent_on_all_canonical_examples() {
    let examples = [
        include_str!("../../tree-sitter-sudolang/examples/riteway.sudo"),
        include_str!("../../tree-sitter-sudolang/examples/autodux.sudo"),
        include_str!("../../tree-sitter-sudolang/examples/ai-rpg.sudo"),
        include_str!("../../tree-sitter-sudolang/examples/sudolang.sudo"),
        include_str!("../../tree-sitter-sudolang/examples/vector-search.sudo"),
    ];
    for src in examples {
        let once = fmt(src);
        let twice = fmt(&once);
        assert_eq!(once, twice);
    }
}

#[test]
fn close_brace_dedents() {
    let input = "Foo {\n  x\n  }\n";
    let expected = "Foo {\n  x\n}\n";
    assert_eq!(fmt(input), expected);
}
