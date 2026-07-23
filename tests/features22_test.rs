//! SudoLang 2.2 feature tests: placeholder diagnostics, decorator and
//! capability hovers, qualified-name completion, and formatter behavior
//! on 2.2 constructs.

use sudolang_lsp::{completion, diagnostics, formatter, hover, sudolang_language};
use tower_lsp::lsp_types::{HoverContents, Position};
use tree_sitter::Parser;

fn parse(src: &str) -> tree_sitter::Tree {
    let mut p = Parser::new();
    p.set_language(&sudolang_language()).unwrap();
    p.parse(src, None).unwrap()
}

fn hover_body(h: &tower_lsp::lsp_types::Hover) -> &str {
    match &h.contents {
        HoverContents::Markup(m) => &m.value,
        _ => panic!("expected markup hover"),
    }
}

const SHOWCASE: &str = "\
@agent(general)
gatherContext() {
  issue = mcp::linear.getIssue(ISSUE_ID)
  !issue -> throw \"unresolved\"
  git::worktree.add(branch = issue.branchName, base = \"origin/development\")
  open = issues |> filter(_.state == \"open\") |> map(_.title)
  parent = issue?.parent?.title ?? \"none\"
  config = { ...defaults, theme: \"dark\" }
}
";

#[test]
fn showcase_parses_clean_and_formats_idempotently() {
    let tree = parse(SHOWCASE);
    assert!(!tree.root_node().has_error(), "2.2 showcase must parse");
    let once = formatter::format(SHOWCASE, &tree);
    let twice = formatter::format(&once, &parse(&once));
    assert_eq!(once, twice, "formatter must be idempotent on 2.2 syntax");
}

#[test]
fn placeholder_outside_pipe_stage_warns() {
    let src = "x = _.title\n";
    let tree = parse(src);
    let diags = diagnostics::placeholder_misuse(&tree, src);
    assert_eq!(diags.len(), 1, "got: {diags:?}");
    assert!(diags[0].message.contains("pipe placeholder"));
}

#[test]
fn placeholder_inside_pipe_stage_is_fine() {
    let src = "open = issues |> filter(_.state == \"open\") |> map(_.title)\n";
    let tree = parse(src);
    let diags = diagnostics::placeholder_misuse(&tree, src);
    assert!(diags.is_empty(), "got: {diags:?}");
}

#[test]
fn placeholder_as_parameter_or_pattern_is_exempt() {
    let src = "handle(_) {\n  ok\n}\n[_, second] = pair\n";
    let tree = parse(src);
    let diags = diagnostics::placeholder_misuse(&tree, src);
    assert!(diags.is_empty(), "got: {diags:?}");
}

#[test]
fn decorator_hover_has_blurbs_for_known_and_unknown() {
    let tree = parse(SHOWCASE);
    // `@agent` on line 0.
    let h = hover::hover(&tree, SHOWCASE, Position::new(0, 2)).expect("hover");
    let b = hover_body(&h);
    assert!(b.contains("decorator"), "got: {b}");
    assert!(b.contains("subagent"), "got: {b}");

    let src = "@mystery(1)\nrun() {\n  go\n}\n";
    let t2 = parse(src);
    let h2 = hover::hover(&t2, src, Position::new(0, 3)).expect("hover");
    let b2 = hover_body(&h2);
    assert!(b2.contains("Unknown decorators are"), "got: {b2}");
}

#[test]
fn capability_hover_shows_the_full_path() {
    let tree = parse(SHOWCASE);
    // `linear` inside `mcp::linear.getIssue` (line 2).
    let h = hover::hover(&tree, SHOWCASE, Position::new(2, 16)).expect("hover");
    let b = hover_body(&h);
    assert!(b.contains("capability"), "got: {b}");
    assert!(b.contains("mcp::linear"), "got: {b}");
}

#[test]
fn qualified_paths_offered_as_completions() {
    let tree = parse(SHOWCASE);
    let items = completion::complete(&tree, SHOWCASE);
    assert!(
        items
            .iter()
            .any(|i| i.label == "mcp::linear" && i.detail.as_deref() == Some("capability")),
        "missing mcp::linear capability item"
    );
    assert!(items.iter().any(|i| i.label == "git::worktree"));
}

#[test]
fn guard_and_named_argument_have_no_false_diagnostics() {
    let tree = parse(SHOWCASE);
    let diags = diagnostics::collect(&tree, SHOWCASE);
    assert!(diags.is_empty(), "got: {diags:?}");
}
