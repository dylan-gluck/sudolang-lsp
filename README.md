# sudolang-lsp

A structural language server for SudoLang, the pseudolanguage for instructing LLMs.

The server builds on [`tower-lsp`](https://crates.io/crates/tower-lsp) and [`tree-sitter-sudolang`](https://github.com/dylan-gluck/tree-sitter-sudolang).

## Features

| Capability                  | Status |
|-----------------------------|--------|
| `textDocument/publishDiagnostics` | ✅ syntax errors (unbalanced braces, missing tokens, malformed modifiers, broken `${}` interpolations) and the 2.2 placeholder-misuse lint |
| `textDocument/formatting`         | ✅ deterministic and structure-driven. Formats `sudo` fences in markdown in place |
| `textDocument/hover`              | ✅ keyword, decorator, and capability blurbs. Identifier and command signatures |
| `textDocument/completion`         | ✅ keywords, 2.2 decorators, every named declaration, capability namespaces (`mcp::linear`) |
| `textDocument/definition`         | ✅ jumps to a declaration, across all fences of a markdown document |

The server targets **SudoLang v2.2** with grammar 0.3.2. Version 2.2 covers qualified capability names (`::`), named arguments, and guard statements (`->`). It also covers decorators, optional chaining `?.`, nullish default `??`, spread `...`, and the pipe placeholder `_`.

## Markdown documents

Write SudoLang in markdown with `sudo` code fences. Use a plain `.md` file, or use `.sudo.md` to mark the file as SudoLang content.

The server treats each ```` ```sudo ```` fence as a virtual document. It diagnoses, formats, hovers, and navigates each fence on its own, and maps every position back to the host file. All fences in one document share one symbol table, so a function declared in one fence resolves from another.

The server never touches prose. The formatter skips a broken fence. The server ignores a ```` ```sudo-next ```` fence, which holds proposal syntax. A pure `.sudo` file behaves as before.

## Diagnostics

The server walks the parsed tree and reports:

- **Syntax error**: any `ERROR` node from the grammar.
- **Missing token**: any `MISSING` node. This is usually a `}` that the parser inferred to recover.
- **Malformed modifier**: a `modifier_list` that contains parse errors. A modifier looks like `:length=short;format=json`.
- **Broken interpolation**: a `string_interpolation` that contains parse errors. Use `$identifier` or `${expression}`.

## Hover

The server resolves the token under the cursor and returns a Markdown blurb:

- **Keyword** (`fn`, `interface`, `require`, `warn`, `constraint`, and others): a short description from a static table.
- **Identifier** that matches a named declaration in the document: a `kind name` header, plus the first line of the declaration in a `sudo` code block. The server joins multiple matches with a divider.
- **Command name** (`/welcome`, `/help`, and others): the first line of the matching `command_declaration`. If the file does not declare the command, the server returns a generic `command` blurb.

## Completion

The server returns a static list of SudoLang keywords, every named declaration in the document, and every capability namespace the document references (`mcp::linear`). Declarations cover functions, interfaces, properties, parameters, variables, constraint blocks, and commands. The server removes duplicates by `kind::name`. The trigger characters are `.`, `/`, `$`, `:`, and `@`. The client filters by prefix.

## Definition

The server jumps from an identifier or a `/command` invocation to its declaration in the same document. It does not resolve across files, because SudoLang has no module system to anchor that on. A click on the declaration itself returns no destinations, which avoids a self-target.

## Formatter

The formatter is conservative and deterministic. It walks the parse tree to find where blocks open and close, then re-indents each line of the source to match. It never reorders or rewrites tokens. It never splits or joins lines. It never touches the body of a multi-line string or comment.

The formatter normalizes:

- leading whitespace to `2 × block_depth` spaces, computed from the AST
- trailing whitespace, which it removes
- a run of two or more blank lines, which becomes one blank line
- a missing terminal newline, which it adds

The formatter leaves alone:

- everything inside `block_comment`, `triple_quoted_block`, `double_string`, and `template_string`
- operator spacing, brace placement, and comma placement on existing lines

If a pure `.sudo` document contains parse errors, the server refuses to format it, because the block ranges to re-indent against are unreliable. In markdown, clean fences still format and the server skips only the broken ones.

## Install

From crates.io:

```sh
cargo install sudolang-lsp
```

Prebuilt binaries for macOS (arm64 and x64), Linux (x64 and arm64), and Windows (x64) ship with each [GitHub Release](https://github.com/dylan-gluck/sudolang-lsp/releases). Download an archive, unpack it, and put `sudolang-lsp` on your `$PATH`.

You can also build from a local checkout. The grammar is a path dependency, so check out `tree-sitter-sudolang` as a sibling directory:

```sh
git clone https://github.com/dylan-gluck/tree-sitter-sudolang
git clone https://github.com/dylan-gluck/sudolang-lsp
cd sudolang-lsp
cargo install --path .
```

Both methods install the `sudolang-lsp` binary into `~/.cargo/bin`. The Zed extension (`zed-sudolang`) finds it on `$PATH`.

## Test

```sh
cargo test
cargo run --release --example format_canonical
```

## License

MIT.
