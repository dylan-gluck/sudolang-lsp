//! One-shot command-line modes for the `sudolang-lsp` binary.
//!
//! `sudolang-lsp check <file>...` reports the diagnostics the server
//! would publish, without an editor and without a build step. It reads a
//! pure `.sudo` file whole, and it reads a markdown host fence by fence,
//! so the line numbers it prints are the line numbers of the host file.
//!
//! This is the entry point for an agent or a CI job. The installed
//! binary is the only dependency: no workspace checkout, no grammar
//! checkout, and no `cargo run`.

use std::fmt;
use std::fs;

use crate::document::{Document, FormatResult};

pub const USAGE: &str = "\
sudolang-lsp — structural language server for SudoLang

USAGE
    sudolang-lsp                     serve the LSP over stdio (default)
    sudolang-lsp check <file>...     print the diagnostics for each file
    sudolang-lsp fmt [flag] <file>...  format SudoLang
    sudolang-lsp --help              print this message
    sudolang-lsp --version           print the version

CHECK
    Accepts .sudo, .md, .sudo.md, and .mdc. A markdown host is checked
    one sudo fence at a time, and every line number is a host line.
    Reports syntax errors, missing tokens, malformed modifier lists,
    broken string interpolations, and pipe-placeholder misuse.

    Output is one finding per line: <path>:<line>:<column>: <message>

FMT
    The same deterministic re-indent the editor runs on save. It never
    reorders tokens, it never splits or joins a line, and in a markdown
    host it formats each clean sudo fence in place and never touches
    prose. It declines a file that does not parse — run check first.

    sudolang-lsp fmt <file>          print the formatted text to stdout
    sudolang-lsp fmt --check <file>...  report which files would change
    sudolang-lsp fmt --write <file>...  rewrite each file in place

    Reading from stdout is the default, so a plain `fmt` takes one file.

EXIT
    0  clean, or formatted
    1  findings reported (check), or a file would change (fmt --check)
    2  usage error, an unreadable file, or a file too broken to format";

/// One diagnostic, flattened to a grep-friendly line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub path: String,
    /// 1-indexed, as an editor shows it.
    pub line: u32,
    /// 1-indexed, as an editor shows it.
    pub column: u32,
    pub message: String,
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}:{}: {}",
            self.path, self.line, self.column, self.message
        )
    }
}

/// What the process should print and exit with.
pub struct Report {
    pub output: String,
    pub code: i32,
}

/// Diagnose one already-read source. Pure: no file system, no process
/// exit — the unit tests drive this directly.
///
/// `path` decides the host kind (`.md` / `.markdown` / `.mdc` extract
/// fences; anything else is pure SudoLang), and it labels each finding.
pub fn check_source(path: &str, text: &str) -> Vec<Finding> {
    let Some(doc) = Document::for_path(path, text.to_string(), 0) else {
        return vec![Finding {
            path: path.to_string(),
            line: 1,
            column: 1,
            message: "Could not parse the file with the SudoLang grammar".to_string(),
        }];
    };

    doc.diagnostics()
        .into_iter()
        .map(|d| Finding {
            path: path.to_string(),
            line: d.range.start.line + 1,
            column: d.range.start.character + 1,
            message: one_line(&d.message),
        })
        .collect()
}

/// Read each path, diagnose it, and build the process report.
pub fn check_paths(paths: &[String]) -> Report {
    if paths.is_empty() {
        return Report {
            output: format!("error: check needs at least one file\n\n{USAGE}\n"),
            code: 2,
        };
    }

    let mut output = String::new();
    let mut findings = 0usize;
    let mut unreadable = 0usize;

    for path in paths {
        match fs::read_to_string(path) {
            Err(e) => {
                output.push_str(&format!("error: cannot read {path}: {e}\n"));
                unreadable += 1;
            }
            Ok(text) => {
                let found = check_source(path, &text);
                if found.is_empty() {
                    output.push_str(&format!("ok: {path}\n"));
                } else {
                    for f in &found {
                        output.push_str(&format!("{f}\n"));
                    }
                    findings += found.len();
                }
            }
        }
    }

    output.push_str(&summary(findings, paths.len(), unreadable));
    output.push('\n');

    let code = if unreadable > 0 {
        2
    } else if findings > 0 {
        1
    } else {
        0
    };
    Report { output, code }
}

/// How `fmt` delivers its result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FmtMode {
    /// Print the formatted text to stdout. One file only.
    Stdout,
    /// Report which files would change. Write nothing.
    Check,
    /// Rewrite each changed file in place.
    Write,
}

/// Format one already-read source. Pure: no file system, no process
/// exit. `path` decides the host kind, exactly as it does for `check`.
pub fn fmt_source(path: &str, text: &str) -> FormatResult {
    match Document::for_path(path, text.to_string(), 0) {
        Some(doc) => doc.format(),
        // A document the grammar cannot load at all is not formattable.
        None => FormatResult::Refused,
    }
}

/// Parse the `fmt` argument list, then format each path.
pub fn fmt_paths(args: &[String]) -> Report {
    let mut mode = FmtMode::Stdout;
    let mut paths: Vec<&String> = Vec::new();

    for arg in args {
        match arg.as_str() {
            "--check" => mode = FmtMode::Check,
            "--write" | "-w" => mode = FmtMode::Write,
            other if other.starts_with('-') => {
                return Report {
                    output: format!("error: unknown fmt flag `{other}`\n\n{USAGE}\n"),
                    code: 2,
                };
            }
            _ => paths.push(arg),
        }
    }

    if paths.is_empty() {
        return Report {
            output: format!("error: fmt needs at least one file\n\n{USAGE}\n"),
            code: 2,
        };
    }
    if mode == FmtMode::Stdout && paths.len() > 1 {
        return Report {
            output: format!(
                "error: fmt writes to stdout, so it takes one file — \
                 use --write to rewrite {} files in place\n",
                paths.len()
            ),
            code: 2,
        };
    }

    let mut output = String::new();
    let mut would_change = 0usize;
    let mut failed = 0usize;

    for path in &paths {
        let text = match fs::read_to_string(path.as_str()) {
            Err(e) => {
                output.push_str(&format!("error: cannot read {path}: {e}\n"));
                failed += 1;
                continue;
            }
            Ok(t) => t,
        };

        match fmt_source(path, &text) {
            FormatResult::Refused => {
                output.push_str(&format!(
                    "refused: {path} — it does not parse; run `sudolang-lsp check {path}` first\n"
                ));
                failed += 1;
            }
            FormatResult::Unchanged => match mode {
                FmtMode::Stdout => output.push_str(&text),
                _ => output.push_str(&format!("ok: {path}\n")),
            },
            FormatResult::Formatted(formatted) => match mode {
                FmtMode::Stdout => output.push_str(&formatted),
                FmtMode::Check => {
                    output.push_str(&format!("would reformat: {path}\n"));
                    would_change += 1;
                }
                FmtMode::Write => match fs::write(path.as_str(), &formatted) {
                    Ok(()) => {
                        output.push_str(&format!("formatted: {path}\n"));
                        would_change += 1;
                    }
                    Err(e) => {
                        output.push_str(&format!("error: cannot write {path}: {e}\n"));
                        failed += 1;
                    }
                },
            },
        }
    }

    // stdout mode emits the file itself — no summary line to corrupt it.
    if mode != FmtMode::Stdout {
        output.push_str(&fmt_summary(mode, would_change, paths.len(), failed));
        output.push('\n');
    }

    let code = if failed > 0 {
        2
    } else if mode == FmtMode::Check && would_change > 0 {
        1
    } else {
        0
    };
    Report { output, code }
}

fn fmt_summary(mode: FmtMode, changed: usize, files: usize, failed: usize) -> String {
    let file_word = if files == 1 { "file" } else { "files" };
    if failed > 0 {
        return format!("-- {failed} of {files} {file_word} could not be formatted");
    }
    match mode {
        FmtMode::Check if changed > 0 => {
            format!("-- {changed} of {files} {file_word} would change")
        }
        FmtMode::Write if changed > 0 => {
            format!("-- formatted {changed} of {files} {file_word}")
        }
        _ => format!("-- already formatted: {files} {file_word}"),
    }
}

fn summary(findings: usize, files: usize, unreadable: usize) -> String {
    let file_word = if files == 1 { "file" } else { "files" };
    if unreadable > 0 {
        return format!("-- {unreadable} of {files} {file_word} could not be read");
    }
    if findings == 0 {
        return format!("-- clean: {files} {file_word}");
    }
    let finding_word = if findings == 1 { "finding" } else { "findings" };
    format!("-- {findings} {finding_word} in {files} {file_word}")
}

/// Collapse a message onto one line. An ERROR node preview is a raw
/// source snippet, so it can carry newlines and runs of indentation;
/// one finding per line keeps the output greppable.
fn one_line(message: &str) -> String {
    message.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_sudo_source_reports_nothing() {
        let findings = check_source("clean.sudo", "greet(name) {\n  say(name)\n}\n");
        assert_eq!(findings, vec![]);
    }

    #[test]
    fn syntax_error_reports_a_one_indexed_position() {
        // The ASI hazard: `[` after an expression statement parses as an
        // index on the line above, so the ERROR node opens at `a` —
        // column 2, 1-indexed — and not at the bracket.
        let findings = check_source("bad.sudo", "x = 1\n[a, b] = pair\n");
        assert!(!findings.is_empty(), "expected a syntax error finding");
        assert_eq!(findings[0].path, "bad.sudo");
        assert_eq!(findings[0].line, 2, "line is 1-indexed like an editor");
        assert_eq!(findings[0].column, 2, "column is 1-indexed like an editor");
    }

    #[test]
    fn placeholder_misuse_is_reported() {
        let findings = check_source("ph.sudo", "f(_)\n");
        assert!(
            findings.iter().any(|f| f.message.contains("pipe placeholder")),
            "expected the placeholder lint, got {findings:?}"
        );
    }

    #[test]
    fn markdown_findings_carry_host_line_numbers() {
        // The fence body starts on line 4 of the host document, so the
        // broken line is host line 5 — not fence-local line 2.
        let doc = "# Notes\n\nProse.\n\n```sudo\nx = 1\n[a, b] = pair\n```\n";
        let findings = check_source("notes.md", doc);
        assert!(!findings.is_empty(), "expected a finding inside the fence");
        assert_eq!(findings[0].line, 7, "fence line lifted to the host line");
    }

    #[test]
    fn prose_outside_a_fence_is_not_diagnosed() {
        let doc = "# Title\n\nThis prose is not SudoLang, and it must not parse.\n";
        assert_eq!(check_source("notes.md", doc), vec![]);
    }

    #[test]
    fn mdc_is_treated_as_a_markdown_host() {
        // A `.mdc` file is a markdown host, like `validate.sh` treats it.
        // Read as pure SudoLang the prose below would be a syntax error.
        let doc = "# Rule\n\nProse that only parses as markdown.\n";
        assert_eq!(check_source("rule.mdc", doc), vec![]);
    }

    #[test]
    fn a_message_never_spans_lines() {
        // An ERROR node preview is raw source, so it can carry newlines.
        let findings = check_source("multi.sudo", "x = 1\n[a,\n b,\n c] = t\n");
        for f in &findings {
            assert!(!f.message.contains('\n'), "message must be one line: {f:?}");
        }
    }

    // ---- fmt ---------------------------------------------------------

    #[test]
    fn fmt_leaves_canonical_shape_alone() {
        let src = "greet(name) {\n  say(name)\n}\n";
        assert_eq!(fmt_source("clean.sudo", src), FormatResult::Unchanged);
    }

    #[test]
    fn fmt_reindents_a_block() {
        let src = "greet(name) {\nsay(name)\n}\n";
        let FormatResult::Formatted(out) = fmt_source("flat.sudo", src) else {
            panic!("expected a reindent");
        };
        assert_eq!(out, "greet(name) {\n  say(name)\n}\n");
    }

    #[test]
    fn fmt_refuses_a_broken_pure_file() {
        // Block ranges are unreliable once the tree has errors.
        assert_eq!(
            fmt_source("bad.sudo", "x = 1\n[a, b] = pair\n"),
            FormatResult::Refused
        );
    }

    #[test]
    fn fmt_never_touches_prose_in_a_markdown_host() {
        let doc = "# Title\n\nProse   with   odd   spacing.\n\n```sudo\nf() {\ng()\n}\n```\n";
        let FormatResult::Formatted(out) = fmt_source("notes.md", doc) else {
            panic!("expected the fence to be reindented");
        };
        assert!(
            out.contains("Prose   with   odd   spacing."),
            "prose must survive verbatim: {out}"
        );
        assert!(out.contains("  g()"), "the fence body must be indented: {out}");
    }

    #[test]
    fn fmt_to_stdout_takes_one_file() {
        let report = fmt_paths(&["a.sudo".to_string(), "b.sudo".to_string()]);
        assert_eq!(report.code, 2);
        assert!(report.output.contains("--write"));
    }

    #[test]
    fn fmt_rejects_an_unknown_flag() {
        let report = fmt_paths(&["--fix".to_string(), "a.sudo".to_string()]);
        assert_eq!(report.code, 2);
        assert!(report.output.contains("unknown fmt flag"));
    }

    #[test]
    fn fmt_with_no_file_is_a_usage_error() {
        let report = fmt_paths(&["--check".to_string()]);
        assert_eq!(report.code, 2);
        assert!(report.output.contains("at least one file"));
    }

    #[test]
    fn fmt_summary_matches_the_exit_code() {
        assert_eq!(
            fmt_summary(FmtMode::Check, 0, 2, 0),
            "-- already formatted: 2 files"
        );
        assert_eq!(
            fmt_summary(FmtMode::Check, 1, 2, 0),
            "-- 1 of 2 files would change"
        );
        assert_eq!(
            fmt_summary(FmtMode::Write, 2, 2, 0),
            "-- formatted 2 of 2 files"
        );
    }

    #[test]
    fn no_files_is_a_usage_error() {
        let report = check_paths(&[]);
        assert_eq!(report.code, 2);
        assert!(report.output.contains("USAGE"));
    }

    #[test]
    fn an_unreadable_file_exits_two() {
        let report = check_paths(&["/nonexistent/path/to/file.sudo".to_string()]);
        assert_eq!(report.code, 2, "a missing file is an environment error");
        assert!(report.output.contains("cannot read"));
    }

    #[test]
    fn summary_counts_agree_with_the_exit_code() {
        assert_eq!(summary(0, 1, 0), "-- clean: 1 file");
        assert_eq!(summary(1, 1, 0), "-- 1 finding in 1 file");
        assert_eq!(summary(3, 2, 0), "-- 3 findings in 2 files");
    }
}
