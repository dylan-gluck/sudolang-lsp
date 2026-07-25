//! Dump the diagnostics the server would publish for a file, with host
//! line numbers (1-indexed, as an editor shows them). Debugging aid:
//!
//!   cargo run --example diag_dump -- path/to/file.md

use std::{env, fs};

use sudolang_lsp::document::Document;

fn main() {
    let path = env::args().nth(1).expect("usage: diag_dump <file>");
    let text = fs::read_to_string(&path).expect("read file");
    let doc = Document::for_path(&path, text, 0).expect("parse document");
    let diags = doc.diagnostics();
    println!("{} diagnostics", diags.len());
    for d in diags {
        println!(
            "  {}:{}..{}:{}  {}",
            d.range.start.line + 1,
            d.range.start.character,
            d.range.end.line + 1,
            d.range.end.character,
            d.message
        );
    }
}
