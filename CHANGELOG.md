# Changelog

All notable changes to `sudolang-lsp` are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

[0.1.0]: https://github.com/dylan-gluck/sudolang-lsp/releases/tag/v0.1.0
