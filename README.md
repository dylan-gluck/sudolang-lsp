# sudolang-lsp

A structural language server for [SudoLang](https://github.com/paralleldrive/sudolang-llm-support).

Built on [`tower-lsp`](https://crates.io/crates/tower-lsp) and
[`tree-sitter-sudolang`](https://github.com/dylan-gluck/tree-sitter-sudolang).

## Features

| Capability                  | Status |
|-----------------------------|--------|
| `textDocument/publishDiagnostics` | ✅ syntax errors (unbalanced braces, missing tokens, malformed modifiers, broken `${}` interpolations) + 2.2 placeholder-misuse lint |
| `textDocument/formatting`         | ✅ deterministic, structure-driven; formats `sudo` fences in markdown in place |
| `textDocument/hover`              | ✅ keyword / decorator / capability blurbs + identifier / command signatures |
| `textDocument/completion`         | ✅ keywords, every named declaration, capability namespaces (`mcp::linear`) |
| `textDocument/definition`         | ✅ jumps to declaration, across all fences of a markdown document |

Targets **SudoLang v2.2** (grammar 0.3.0): qualified capability names
(`::`), named arguments, guard statements (`->`), decorators, optional
chaining `?.`, nullish default `??`, spread `...`, pipe placeholder `_`.

## Markdown documents

The preferred SudoLang authoring format is **markdown with `sudo` code
fences** — plain `.md`, or `.sudo.md` to signal SudoLang content. The
server treats each ```` ```sudo ```` fence as a virtual document: fences
are diagnosed, formatted, hovered, and navigated independently, with all
positions mapped back to the host file, while the fences of one document
share a single symbol table (a function declared in one fence resolves
from another). Prose is never touched; broken fences are skipped by the
formatter; ```` ```sudo-next ```` fences (proposal syntax) are ignored.
Pure `.sudo` files behave as before.

## Diagnostics

The server walks the parsed tree and reports:

- **Syntax error** — any `ERROR` node produced by the grammar.
- **Missing token** — any `MISSING` node (typically a `}` the parser inferred to recover).
- **Malformed modifier** — a `modifier_list` containing parse errors. Modifiers look like `:length=short;format=json`.
- **Broken interpolation** — a `string_interpolation` containing parse errors. Use `$identifier` or `${expression}`.

## Hover

Resolves the token under the cursor and returns a Markdown blurb:

- **Keyword** (`fn`, `interface`, `require`, `warn`, `constraint`, …) —
  short prose description from a static table.
- **Identifier** that matches a named declaration in the document —
  `kind name` header plus the first line of the declaration, rendered as
  a `sudo` code block. Multiple matches are joined with a divider.
- **Command name** (`/welcome`, `/help`, …) — the matching
  `command_declaration`'s first line, or a generic `command` blurb when
  the command isn't declared in-file.

## Completion

Returns a static list of SudoLang keywords, every named declaration
the document defines (functions, interfaces, properties, parameters,
variables, constraint blocks, commands), and every capability namespace
it references (`mcp::linear`). De-duplicated by `kind::name`. Trigger
characters: `.`, `/`, `$`, `:`, `@`. The client filters by prefix.

## Definition

Jumps from an identifier or `/command` invocation to its declaration in
the same document. Cross-file resolution is not implemented — SudoLang
has no module system to anchor it on. Clicking on the declaration itself
returns no destinations (avoids self-target).

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

If a pure `.sudo` document contains parse errors, the server declines to
format — the block ranges we'd re-indent against would be unreliable. In
markdown, clean fences still format; only the broken ones are skipped.

## Install

From crates.io:

```sh
cargo install sudolang-lsp
```

Prebuilt binaries for macOS (arm64/x64), Linux (x64/arm64), and Windows
(x64) ship with each [GitHub Release](https://github.com/dylan-gluck/sudolang-lsp/releases) —
download, unpack, and put `sudolang-lsp` on your `$PATH`.

Or from a local checkout (needs `tree-sitter-sudolang` checked out as a
sibling directory — the grammar is a path dependency):

```sh
git clone https://github.com/dylan-gluck/tree-sitter-sudolang
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
