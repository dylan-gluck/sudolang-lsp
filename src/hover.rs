//! Hover provider.
//!
//! Resolves the token under the cursor and returns a Markdown blurb:
//!
//!   - keyword (`fn`, `interface`, `require`, `warn`, `constraint`, …)
//!     → short prose description from a static table
//!   - identifier that matches a document symbol
//!     → `kind name` header + the first line of the declaration
//!   - command name (`/welcome`, `/help`, …)
//!     → the command's first line (its description)
//!
//! No hover is produced for whitespace, operators, or punctuation.

use tower_lsp::lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Position, Range};
use tree_sitter::{Node, Tree};

use crate::diagnostics::{node_range, position_to_byte};
use crate::symbols::{self, Symbol};

pub fn hover(tree: &Tree, source: &str, position: Position) -> Option<Hover> {
    hover_with(tree, source, position, &symbols::collect(tree, source))
}

/// Hover with an externally supplied symbol table — markdown documents
/// pass the symbols of *all* their fences, since the fences of one
/// document form one program. Only symbol names/kinds/signatures are
/// used, so block-local ranges in `syms` are fine.
pub fn hover_with(
    tree: &Tree,
    source: &str,
    position: Position,
    syms: &[Symbol],
) -> Option<Hover> {
    let byte = position_to_byte(source, position)?;
    let leaf = tree.root_node().descendant_for_byte_range(byte, byte)?;

    // 1. Anonymous keyword tokens (`fn`, `if`, `match`, …) — checked on
    //    the raw leaf so they aren't collapsed into a parent node.
    if !leaf.is_named() {
        if let Some(h) = keyword_hover_anonymous(leaf, source) {
            return Some(h);
        }
    }

    // 2. Named-node hover paths (constraint_keyword, command_name,
    //    decorator_name, qualified names, identifier).
    let named = nearest_named(leaf)?;
    if let Some(h) = keyword_hover_named(named, source) {
        return Some(h);
    }
    if let Some(h) = decorator_hover(named, source) {
        return Some(h);
    }
    if let Some(h) = capability_hover(named, source) {
        return Some(h);
    }
    if let Some(h) = command_hover(named, source, syms) {
        return Some(h);
    }
    if let Some(h) = identifier_hover(named, source, syms) {
        return Some(h);
    }
    None
}

/// Hover on a decorator (`@retry`, `@agent`, …) — known decorators get a
/// blurb; unknown ones are legal and inferred, and the hover says so.
fn decorator_hover(node: Node, source: &str) -> Option<Hover> {
    if node.kind() != "decorator_name" {
        return None;
    }
    let name = node_text(node, source);
    Some(markdown(decorator_blurb(name), node_range(node, source)))
}

fn decorator_blurb(name: &str) -> String {
    let known = match name {
        "@agent" => "Run the decorated unit as the named subagent. `@agent(general)`",
        "@retry" => "Retry the decorated unit on failure. `@retry(3)` — up to 3 attempts.",
        "@timeout" => "Abort the decorated unit after the given duration. `@timeout(120)`",
        "@parallel" => "Run iterations (or the decorated unit) concurrently.",
        "@memo" => "Memoize — repeated calls with the same inputs reuse the previous result.",
        "@blocking" => "Requires interaction before continuing. `@blocking(user)` waits for the user.",
        _ => {
            return format!(
                "**decorator** `{name}`\n\nExecution metadata. Unknown decorators are \
                 legal — the interpreter infers their meaning from the name."
            )
        }
    };
    format!("**decorator** `{name}`\n\n{known}")
}

/// Hover anywhere inside a qualified capability name (`mcp::linear.…`).
fn capability_hover(node: Node, source: &str) -> Option<Hover> {
    let qualified = if node.kind() == "qualified_identifier" {
        node
    } else if node.kind() == "identifier" {
        let parent = node.parent()?;
        if parent.kind() != "qualified_identifier" {
            return None;
        }
        parent
    } else {
        return None;
    };
    let path = node_text(qualified, source);
    Some(markdown(
        format!(
            "**capability** `{path}`\n\nCapability namespace — resolved against the \
             environment (tools, MCP servers, agents), not read off a value."
        ),
        node_range(qualified, source),
    ))
}

fn nearest_named(mut node: Node<'_>) -> Option<Node<'_>> {
    while !node.is_named() {
        node = node.parent()?;
    }
    Some(node)
}

fn keyword_hover_anonymous(node: Node, source: &str) -> Option<Hover> {
    let text = node_text(node, source);
    let blurb = keyword_blurb(text)?;
    Some(markdown(blurb, node_range(node, source)))
}

fn keyword_hover_named(node: Node, source: &str) -> Option<Hover> {
    if node.kind() == "constraint_keyword" {
        return Some(markdown(
            keyword_blurb("constraint")?,
            node_range(node, source),
        ));
    }
    None
}

fn command_hover(node: Node, source: &str, syms: &[Symbol]) -> Option<Hover> {
    if node.kind() != "command_name" {
        return None;
    }
    let name = node_text(node, source);
    let matched = syms
        .iter()
        .find(|s| s.name == name && matches!(s.kind, crate::symbols::SymbolKind::Command));
    let body = match matched {
        Some(sym) => format_symbol(sym),
        None => format!("**command** `{name}`"),
    };
    Some(markdown(body, node_range(node, source)))
}

fn identifier_hover(node: Node, source: &str, syms: &[Symbol]) -> Option<Hover> {
    if node.kind() != "identifier" {
        return None;
    }
    let name = node_text(node, source);
    let matched: Vec<&Symbol> = syms.iter().filter(|s| s.name == name).collect();
    if matched.is_empty() {
        return None;
    }
    let body = matched
        .iter()
        .map(|s| format_symbol(s))
        .collect::<Vec<_>>()
        .join("\n\n---\n\n");
    Some(markdown(body, node_range(node, source)))
}

fn format_symbol(sym: &Symbol) -> String {
    format!(
        "**{kind}** `{name}`\n\n```sudo\n{sig}\n```",
        kind = sym.kind.label(),
        name = sym.name,
        sig = sym.signature,
    )
}

fn markdown(body: impl Into<String>, range: Range) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: body.into(),
        }),
        range: Some(range),
    }
}

fn keyword_blurb(word: &str) -> Option<String> {
    let blurb = match word {
        "fn" | "function" => "Function declaration. `fn name(params) { … }`",
        "interface" => "Interface declaration — a named bag of properties and commands.",
        "match" => "Pattern-match expression. `match (expr) { case … => …, default => … }`",
        "case" => "Branch of a `match` expression.",
        "default" => "Fallback branch of a `match` expression.",
        "if" => "Conditional. `if (cond) { … } else { … }`",
        "else" => "Else branch of an `if`.",
        "for" => "Iteration. `for each x in xs { … }`",
        "each" => "Used with `for`. `for each x in xs { … }`",
        "in" => "Iteration source. `for each x in xs { … }`",
        "while" => "Loop until the condition is false.",
        "loop" => "Unconditional loop.",
        "return" => "Return from the enclosing function.",
        "throw" => "Throw an error.",
        "try" => "Try block. Pair with `catch`.",
        "catch" => "Catch block. Handles errors thrown by `try`.",
        "->" => "Guard consequence: run the right side iff the left side holds. `cond -> statement`",
        "??" => "Nullish default: left side unless it is null/absent, else the right side.",
        "?." => "Optional chaining: short-circuits to null when the receiver is null/absent.",
        "::" => "Capability namespace separator — `mcp::linear` resolves against the environment.",
        "..." => "Spread / rest: merge into literals and calls, collect in patterns.",
        "require" => "Precondition. `require <expression>` — must hold for the function to proceed.",
        "warn" => "Soft constraint stated to the model in natural language.",
        "constraint" | "Constraints" | "constraints" => {
            "Constraint block — natural-language rules the model should follow."
        }
        _ => return None,
    };
    Some(blurb.to_string())
}

fn node_text<'a>(node: Node, source: &'a str) -> &'a str {
    let start = node.start_byte().min(source.len());
    let end = node.end_byte().min(source.len());
    &source[start..end]
}
