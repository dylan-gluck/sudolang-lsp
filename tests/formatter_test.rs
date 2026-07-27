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

// --- Indent-bearing constructs beyond `block` -------------------------
//
// Each of these used to flatten to the depth of the statement that opened
// it, because only `block` counted toward indent depth.

#[test]
fn multiline_object_literal_indents_its_entries() {
    let input = "files = {\ndux: infer(),\nstore: infer(),\n}\n";
    let expected = "files = {\n  dux: infer(),\n  store: infer(),\n}\n";
    assert_eq!(fmt(input), expected);
}

#[test]
fn multiline_array_literal_indents_its_entries() {
    let input = "characters = [\n\"Vega\",\n\"Juno\",\n]\n";
    let expected = "characters = [\n  \"Vega\",\n  \"Juno\",\n]\n";
    assert_eq!(fmt(input), expected);
}

#[test]
fn match_arms_indent_inside_the_match_braces() {
    // `match { ... }` braces are not a `block` node, so the arms used to
    // land at column 0.
    let input = "r = match (v) {\ncase 1 => \"one\",\ndefault => \"other\",\n}\n";
    let expected = "r = match (v) {\n  case 1 => \"one\",\n  default => \"other\",\n}\n";
    assert_eq!(fmt(input), expected);
}

#[test]
fn pipe_continuation_lines_indent_one_level() {
    let input = "options = pick(7)\n|> score\n|> takeTop(3)\n";
    let expected = "options = pick(7)\n  |> score\n  |> takeTop(3)\n";
    assert_eq!(fmt(input), expected);
}

#[test]
fn multiline_argument_list_indents_its_arguments() {
    let input = "createDraft(\nbase = \"development\",\ntitle,\n)\n";
    let expected = "createDraft(\n  base = \"development\",\n  title,\n)\n";
    assert_eq!(fmt(input), expected);
}

#[test]
fn multiline_pattern_indents_like_its_literal() {
    let input = "{\nname,\nage,\n} = user\n";
    let expected = "{\n  name,\n  age,\n} = user\n";
    assert_eq!(fmt(input), expected);
}

#[test]
fn stacked_openers_on_one_line_give_one_indent_level() {
    // The `argument_list` and the arrow function's `block` both open on
    // row 0. Counting nodes would double-indent `assert()`; counting
    // distinct opener rows gives it a single level.
    let input = "describe(\"unit\", () => {\nassert()\n})\n";
    let expected = "describe(\"unit\", () => {\n  assert()\n})\n";
    assert_eq!(fmt(input), expected);
}

#[test]
fn nesting_composes_across_construct_kinds() {
    let input = "Dux {\nfiles = {\ndux: infer(),\n}\n}\n";
    let expected = "Dux {\n  files = {\n    dux: infer(),\n  }\n}\n";
    assert_eq!(fmt(input), expected);
}

#[test]
fn single_line_literals_add_no_depth() {
    let input = "Foo {\n  config = { a: 1, b: [2, 3] }\n}\n";
    assert_eq!(fmt(input), input);
}

#[test]
fn canonical_examples_are_already_canonically_formatted() {
    // The strongest contract: the formatter agrees with hand-written
    // idiomatic SudoLang. If this fails, either an example drifted or the
    // indent rule regressed.
    let examples = [
        ("riteway", include_str!("../../tree-sitter-sudolang/examples/riteway.sudo")),
        ("autodux", include_str!("../../tree-sitter-sudolang/examples/autodux.sudo")),
        ("ai-rpg", include_str!("../../tree-sitter-sudolang/examples/ai-rpg.sudo")),
        ("sudolang", include_str!("../../tree-sitter-sudolang/examples/sudolang.sudo")),
        ("vector-search", include_str!("../../tree-sitter-sudolang/examples/vector-search.sudo")),
        ("issue-to-pr", include_str!("../../tree-sitter-sudolang/examples/issue-to-pr.sudo")),
    ];
    for (name, src) in examples {
        let formatted = fmt(src);
        if formatted != src {
            let diff: Vec<String> = src
                .lines()
                .zip(formatted.lines())
                .enumerate()
                .filter(|(_, (a, b))| a != b)
                .map(|(i, (a, b))| format!("  {}: -{a:?}\n     +{b:?}", i + 1))
                .collect();
            panic!("{name} is not canonically formatted:\n{}", diff.join("\n"));
        }
    }
}
