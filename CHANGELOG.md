# Changelog

All notable changes to `sudolang-lsp` are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.2] - 2026-07-27

Lockstep release with tree-sitter-sudolang 0.3.2.

### Added

- **Decorator completion.** The server declared `@` as a completion trigger character, and it offered no decorators. It now returns the documented 2.2 vocabulary: `@agent`, `@retry`, `@timeout`, `@parallel`, `@memo`, and `@blocking`. Each item carries the same blurb that hover shows. An unknown decorator stays legal.

### Fixed

- **The formatter no longer flattens multi-line constructs.** Indent depth counted `block` ancestors only, so format-on-save destroyed the shape of a multi-line object literal, array literal, destructuring pattern, argument list, and pipe chain. It pushed `match` arms to column 0, because the braces of a `match` are not a `block` node.

  Indent depth now counts the distinct rows on which the enclosing indent-bearing constructs opened. Counting rows rather than nodes collapses stacked openers, so `describe("unit", () => {` indents its body one level and not two. A line made of closing delimiters dedents, and `}`, `})`, and `},` all work. A pipe continuation is not a closing line and keeps its level.

  All six canonical examples now format to themselves. `formatter_test` asserts that, so an indent regression fails the suite.

### Changed

- README rewritten in ASD-STE100 Simplified Technical English.
- `examples/format_canonical` prints the changed lines, not only a count.

## [0.3.1] - 2026-07-24

Lockstep release with tree-sitter-sudolang 0.3.1 (a packaging fix for
the grammar's Node binding). No functional changes to the server; the
grammar dependency moves to 0.3.1.

## [0.3.0] - 2026-07-23

Targets **SudoLang v2.2** via tree-sitter-sudolang 0.3.0. Version aligned
with the grammar and Zed extension for the coordinated release.

### Added

- **Markdown virtual documents** — `.md` / `.sudo.md` files are treated
  as hosts: every ```` ```sudo ```` fence is parsed, diagnosed,
  formatted, hovered, and navigated as its own SudoLang block, with all
  positions mapped back to host coordinates. The fences of one document
  share a symbol table (completion / hover / definition see every
  fence). Prose is untouched; broken fences are skipped by the
  formatter; ```` ```sudo-next ```` fences are ignored. This is the
  preferred authoring format; pure `.sudo` files behave as before.
- **Placeholder-misuse diagnostic** (2.2 §3.6) — warns when `_` appears
  outside a pipe stage (parameters and patterns are exempt as discards).
- **Decorator hover** — blurbs for `@agent`, `@retry`, `@timeout`,
  `@parallel`, `@memo`, `@blocking`; unknown decorators get the
  "legal and inferred" note.
- **Capability hover + completion** (2.2 §3.1) — hovering a qualified
  name shows the full `mcp::linear` path with a capability blurb;
  qualified paths already used in the document are offered as `MODULE`
  completions. New trigger characters `:` and `@`.
- Operator hover blurbs for `->`, `??`, `?.`, `::`, `...`.
- CI (test matrix with sibling grammar checkout) and a tag-driven
  release workflow: crates.io publish + prebuilt binaries for
  mac arm64/x64, linux x64/arm64, windows x64.

### Changed

- Grammar dependency: git tag → path + version
  (`../tree-sitter-sudolang`, `0.3.0`). `cargo publish` strips the path
  and resolves crates.io — the grammar crate must be published first.
- `Document` is now a set of blocks; feature modules stay single-tree.
  `hover::hover_with`, `completion::complete_many`,
  `definition::target_at` / `symbol_matches` expose the block-aware
  entry points.

## [0.2.0] - 2026-05-12

### Added

- `textDocument/hover` — Markdown blurbs for keywords (`fn`, `interface`,
  `require`, `warn`, `constraint`, …), in-document identifiers
  (function / interface / property / variable / parameter / constraint),
  and command invocations. Shows the declaration's first line as a
  `sudo`-fenced signature.
- `textDocument/completion` — keyword list plus every named declaration
  the document defines (functions, interfaces, properties, parameters,
  variables, constraint blocks, commands). De-duplicated by `kind::name`.
  Trigger characters: `.`, `/`, `$`.
- `textDocument/definition` — jumps from an identifier or `/command`
  invocation to its declaration in the same document. Clicking on the
  declaration itself returns no destinations (avoids self-target).
- Shared `symbols` module that walks the parse tree once and surfaces
  named declarations for the three providers above.
- `position_to_byte` helper alongside `byte_to_position` in
  `diagnostics`, with UTF-16-aware column handling for LSP positions.

## [0.1.0] - 2026-05-12

Initial release. A structural language server for SudoLang, built on
`tower-lsp` and `tree-sitter-sudolang`.

### Added

- `textDocument/publishDiagnostics` — syntax errors, missing tokens,
  malformed modifier lists, broken `${}` interpolations.
- `textDocument/formatting` — deterministic, AST-driven re-indenter.
  Walks `block` nodes from the parsed tree to compute indent depth per
  line, strips trailing whitespace, collapses 2+ blank lines, ensures a
  single terminal newline. Never reorders tokens, never splits or joins
  lines, never touches the interior of `block_comment` /
  `triple_quoted_block` / `double_string` / `template_string`. Declines
  to run when the document has parse errors.
- FULL document sync via `textDocument/didOpen` / `didChange` /
  `didClose`.
- Pinned to [`tree-sitter-sudolang`](https://github.com/dylan-gluck/tree-sitter-sudolang)
  v0.1.1 (SudoLang v2.1 dialect).

[0.3.0]: https://github.com/dylan-gluck/sudolang-lsp/releases/tag/v0.3.0
[0.2.0]: https://github.com/dylan-gluck/sudolang-lsp/releases/tag/v0.2.0
[0.1.0]: https://github.com/dylan-gluck/sudolang-lsp/releases/tag/v0.1.0
