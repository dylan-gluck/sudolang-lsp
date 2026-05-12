# sudolang-lsp

A structural language server for [SudoLang](https://github.com/paralleldrive/sudolang-llm-support).

Built on [`tower-lsp`](https://crates.io/crates/tower-lsp) and
[`tree-sitter-sudolang`](https://github.com/dylan-gluck/tree-sitter-sudolang).

## Features

| Capability                  | Status |
|-----------------------------|--------|
| `textDocument/publishDiagnostics` | ✅ syntax errors (unbalanced braces, missing tokens, malformed modifiers, broken `${}` interpolations) |
| `textDocument/formatting`         | ✅ deterministic, structure-driven |
| `textDocument/hover`              | — |
| `textDocument/completion`         | — |
| `textDocument/definition`         | — |

Hover / completion / definition are planned; this v0.1 focuses on the two
things every editor needs first: tell me when my code is broken, and keep
it tidy on save.

## Diagnostics

The server walks the parsed tree and reports:

- **Syntax error** — any `ERROR` node produced by the grammar.
- **Missing token** — any `MISSING` node (typically a `}` the parser inferred to recover).
- **Malformed modifier** — a `modifier_list` containing parse errors. Modifiers look like `:length=short;format=json`.
- **Broken interpolation** — a `string_interpolation` containing parse errors. Use `$identifier` or `${expression}`.

## Formatter

The formatter is conservative and deterministic. It walks the parse tree
to learn where blocks open and close, then re-indents each line of source
accordingly. It never reorders or rewrites tokens, never splits or joins
lines, and never touches the body of a multi-line string or comment.

What it normalises:

- leading whitespace → `2 × block_depth` spaces (computed from the AST)
- trailing whitespace → removed
- runs of 2+ blank lines → a single blank line
- missing terminal newline → added

What it leaves alone:

- everything inside `block_comment`, `triple_quoted_block`, `double_string`, `template_string`
- operator spacing, brace placement, comma placement on existing lines

If the document contains parse errors, the server declines to format —
the block ranges we'd re-indent against would be unreliable.

## Install

```sh
cargo install --git https://github.com/dylan-gluck/sudolang-lsp --tag v0.1.0
```

Or from a local checkout:

```sh
git clone https://github.com/dylan-gluck/sudolang-lsp
cd sudolang-lsp
cargo install --path .
```

Either way, this installs the `sudolang-lsp` binary into `~/.cargo/bin`.
The Zed extension (`zed-sudolang`) auto-discovers it from `$PATH`.

## Test

```sh
cargo test
cargo run --release --example format_canonical
```

## License

MIT.
