# Changelog

All notable changes to `sudolang-lsp` are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

[0.2.0]: https://github.com/dylan-gluck/sudolang-lsp/releases/tag/v0.2.0
[0.1.0]: https://github.com/dylan-gluck/sudolang-lsp/releases/tag/v0.1.0
