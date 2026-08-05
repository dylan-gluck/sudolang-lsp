//! ```` ```sudo ```` fence extraction for markdown documents.
//!
//! The preferred SudoLang authoring format is markdown with `sudo` code
//! fences — plain `.md`, or `.sudo.md` to signal SudoLang content. Each
//! fence body is a virtual SudoLang document: parsed, diagnosed,
//! formatted, and navigated independently, with every position mapped
//! back to the host file. Fences are line-offset insets only (they open
//! at column 0), so mapping never touches the character column.
//!
//! ```` ```sudo-next ```` fences (proposal syntax) are deliberately not
//! recognised.

/// One extracted fence body.
pub struct Fence {
    /// 0-based host line where the fence body starts (the line after
    /// the ```` ```sudo ```` opener).
    pub start_line: u32,
    /// Number of body lines (0 for an empty fence).
    pub line_count: u32,
    /// Fence body, newline-terminated when non-empty.
    pub text: String,
}

pub fn is_markdown_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    // `.mdc` hosts fences too — Cursor rule files. `scripts/validate.sh`
    // and the skill docs have always claimed it; the server used to
    // read it as pure SudoLang and diagnose every prose line.
    lower.ends_with(".md") || lower.ends_with(".markdown") || lower.ends_with(".mdc")
}

pub fn extract_fences(text: &str) -> Vec<Fence> {
    let mut fences = Vec::new();
    let mut open: Option<Fence> = None;

    for (i, line) in text.lines().enumerate() {
        match open.as_mut() {
            None => {
                if is_sudo_fence_opener(line) {
                    open = Some(Fence {
                        start_line: i as u32 + 1,
                        line_count: 0,
                        text: String::new(),
                    });
                }
            }
            Some(fence) => {
                if line.trim_end() == "```" {
                    fences.push(open.take().unwrap());
                } else {
                    fence.text.push_str(line);
                    fence.text.push('\n');
                    fence.line_count += 1;
                }
            }
        }
    }

    // Unclosed fence at EOF — the author is mid-edit. Keep the body so
    // diagnostics stay live while typing.
    if let Some(fence) = open {
        if fence.line_count > 0 {
            fences.push(fence);
        }
    }

    fences
}

/// Opener must sit at column 0 with an info string of exactly `sudo` or
/// `sudolang` (case-insensitive) — `sudo-next` and other languages don't
/// match.
fn is_sudo_fence_opener(line: &str) -> bool {
    let Some(info) = line.strip_prefix("```") else {
        return false;
    };
    let info = info.trim();
    info.eq_ignore_ascii_case("sudo") || info.eq_ignore_ascii_case("sudolang")
}
