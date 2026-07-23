//! Markdown virtual-document tests: fence extraction, position mapping,
//! cross-fence navigation, and in-place fence formatting.

use sudolang_lsp::document::{Document, DocumentKind, FormatResult};
use sudolang_lsp::markdown;
use tower_lsp::lsp_types::{Position, Url};

const DOC: &str = "\
# Title

Some prose.

```sudo
greet(name) {
\"Say hello to $name.\"
}
```

More prose in between.

```sudo
greet(\"world\")
```
";

fn md(text: &str) -> Document {
    Document::with_kind(DocumentKind::Markdown, text.to_string(), 1).expect("document")
}

#[test]
fn extracts_fences_with_correct_offsets() {
    let fences = markdown::extract_fences(DOC);
    assert_eq!(fences.len(), 2);
    assert_eq!(fences[0].start_line, 5);
    assert_eq!(fences[0].line_count, 3);
    assert!(fences[0].text.starts_with("greet(name) {"));
    assert_eq!(fences[1].start_line, 13);
    assert_eq!(fences[1].line_count, 1);
    assert_eq!(fences[1].text, "greet(\"world\")\n");
}

#[test]
fn sudo_next_and_other_fences_are_skipped() {
    let text = "```sudo-next\nx = 1\n```\n\n```js\ny = 2\n```\n\n```sudo\nz = 3\n```\n";
    let fences = markdown::extract_fences(text);
    assert_eq!(fences.len(), 1);
    assert_eq!(fences[0].text, "z = 3\n");
}

#[test]
fn unclosed_fence_still_extracts_while_typing() {
    let text = "prose\n\n```sudo\nx = 1\nbroken {\n";
    let fences = markdown::extract_fences(text);
    assert_eq!(fences.len(), 1);
    assert_eq!(fences[0].start_line, 3);
    assert_eq!(fences[0].line_count, 2);
}

#[test]
fn clean_markdown_has_no_diagnostics_and_prose_is_ignored() {
    let doc = md(DOC);
    assert_eq!(doc.blocks.len(), 2);
    let diags = doc.diagnostics();
    assert!(diags.is_empty(), "unexpected: {diags:?}");
}

#[test]
fn diagnostics_are_offset_to_host_lines() {
    let text = "# Doc\n\n```sudo\nFoo {\n  bar = 1\n```\n";
    let doc = md(text);
    let diags = doc.diagnostics();
    assert!(!diags.is_empty(), "expected diagnostic for unbalanced brace");
    // Every diagnostic must land inside the fence body (host lines 3-4).
    for d in &diags {
        assert!(
            d.range.start.line >= 3 && d.range.start.line <= 5,
            "diagnostic outside fence: {d:?}"
        );
    }
}

#[test]
fn hover_maps_positions_through_the_fence() {
    let doc = md(DOC);
    // `greet` in the second fence: host line 12, col 1.
    let h = doc.hover(Position::new(13, 1)).expect("hover");
    let range = h.range.expect("range");
    assert_eq!(range.start.line, 13, "hover range must be in host coords");
    // Hover on prose produces nothing.
    assert!(doc.hover(Position::new(2, 3)).is_none());
}

#[test]
fn definitions_resolve_across_fences() {
    let doc = md(DOC);
    let uri = Url::parse("file:///doc.sudo.md").unwrap();
    // `greet` call in the second fence resolves to the declaration in the
    // first fence, at host coordinates.
    let locs = doc.definitions(&uri, Position::new(13, 1));
    assert_eq!(locs.len(), 1, "got: {locs:?}");
    assert_eq!(locs[0].range.start.line, 5);
}

#[test]
fn completions_only_inside_fences_but_see_all_fences() {
    let doc = md(DOC);
    assert!(doc.completions(Position::new(2, 0)).is_none(), "prose");
    let items = doc.completions(Position::new(13, 0)).expect("in fence");
    assert!(items.iter().any(|i| i.label == "greet"));
}

#[test]
fn formatting_reindents_fences_and_leaves_prose_alone() {
    let doc = md(DOC);
    match doc.format() {
        FormatResult::Formatted(out) => {
            assert!(out.contains("  \"Say hello to $name.\""), "got:\n{out}");
            assert!(out.contains("# Title"));
            assert!(out.contains("More prose in between."));
            // Idempotent: formatting the result changes nothing.
            let again = md(&out);
            assert!(matches!(again.format(), FormatResult::Unchanged));
        }
        other => panic!(
            "expected Formatted, got {}",
            match other {
                FormatResult::Refused => "Refused",
                FormatResult::Unchanged => "Unchanged",
                FormatResult::Formatted(_) => unreachable!(),
            }
        ),
    }
}

#[test]
fn broken_fence_is_left_alone_but_clean_fences_still_format() {
    let text = "```sudo\nFoo {\n  bar = 1\n```\n\n```sudo\nBar {\nbaz = 2\n}\n```\n";
    let doc = md(text);
    match doc.format() {
        FormatResult::Formatted(out) => {
            // Broken fence untouched, clean fence reindented.
            assert!(out.contains("Foo {\n  bar = 1\n"), "got:\n{out}");
            assert!(out.contains("Bar {\n  baz = 2\n}"), "got:\n{out}");
        }
        _ => panic!("expected Formatted"),
    }
}

#[test]
fn markdown_without_fences_is_inert() {
    let doc = md("# Just prose\n\nNothing else.\n");
    assert!(doc.blocks.is_empty());
    assert!(doc.diagnostics().is_empty());
    assert!(matches!(doc.format(), FormatResult::Unchanged));
}

#[test]
fn pure_sudo_documents_still_work_via_for_path() {
    let doc = Document::for_path("/x/program.sudo", "Foo {\nbar = 1\n}\n".into(), 1).unwrap();
    assert_eq!(doc.kind, DocumentKind::Sudo);
    match doc.format() {
        FormatResult::Formatted(out) => assert_eq!(out, "Foo {\n  bar = 1\n}\n"),
        _ => panic!("expected Formatted"),
    }
    let md_doc = Document::for_path("/x/notes.sudo.md", "# hi\n".into(), 1).unwrap();
    assert_eq!(md_doc.kind, DocumentKind::Markdown);
}
