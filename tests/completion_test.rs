use sudolang_lsp::{completion, sudolang_language};
use tower_lsp::lsp_types::CompletionItemKind;
use tree_sitter::Parser;

fn parse(src: &str) -> tree_sitter::Tree {
    let mut p = Parser::new();
    p.set_language(&sudolang_language()).unwrap();
    p.parse(src, None).unwrap()
}

#[test]
fn completion_always_includes_core_keywords() {
    let src = "";
    let tree = parse(src);
    let items = completion::complete(&tree, src);
    for kw in ["fn", "interface", "match", "require", "warn", "constraint"] {
        assert!(
            items
                .iter()
                .any(|i| i.label == kw && i.kind == Some(CompletionItemKind::KEYWORD)),
            "missing keyword `{kw}` in completion"
        );
    }
}

#[test]
fn completion_includes_document_function_names() {
    let src = "fn greet(name) {\n  return name\n}\n";
    let tree = parse(src);
    let items = completion::complete(&tree, src);
    let greet = items
        .iter()
        .find(|i| i.label == "greet")
        .expect("`greet` should be a completion item");
    assert_eq!(greet.kind, Some(CompletionItemKind::FUNCTION));
}

#[test]
fn completion_includes_interface_names() {
    let src = "Foo {\n  bar = 1\n}\n";
    let tree = parse(src);
    let items = completion::complete(&tree, src);
    let foo = items
        .iter()
        .find(|i| i.label == "Foo")
        .expect("`Foo` should be a completion item");
    assert_eq!(foo.kind, Some(CompletionItemKind::INTERFACE));
}

#[test]
fn completion_does_not_double_count_duplicate_parameters() {
    let src = "fn a(x) { return x }\nfn b(x) { return x }\n";
    let tree = parse(src);
    let items = completion::complete(&tree, src);
    let x_count = items.iter().filter(|i| i.label == "x").count();
    assert!(x_count <= 1, "parameter `x` should be de-duplicated; got {x_count}");
}
