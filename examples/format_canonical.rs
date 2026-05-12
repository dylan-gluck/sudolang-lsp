//! Run the formatter against every canonical example and print a diff
//! summary. Useful for visually checking that the formatter does
//! something sensible without rewriting the user's structure.
//!
//! Usage: `cargo run --release --example format_canonical`

use sudolang_lsp::{formatter, sudolang_language};
use tree_sitter::Parser;

const EXAMPLES: &[(&str, &str)] = &[
    ("riteway", include_str!("../../tree-sitter-sudolang/examples/riteway.sudo")),
    ("autodux", include_str!("../../tree-sitter-sudolang/examples/autodux.sudo")),
    ("ai-rpg", include_str!("../../tree-sitter-sudolang/examples/ai-rpg.sudo")),
    ("sudolang", include_str!("../../tree-sitter-sudolang/examples/sudolang.sudo")),
    ("vector-search", include_str!("../../tree-sitter-sudolang/examples/vector-search.sudo")),
];

fn main() {
    let mut parser = Parser::new();
    parser.set_language(&sudolang_language()).unwrap();

    for (name, src) in EXAMPLES {
        let tree = parser.parse(src, None).unwrap();
        let errors = tree.root_node().has_error();
        let formatted = formatter::format(src, &tree);
        let bytes_diff = formatted.len() as isize - src.len() as isize;
        let lines_in = src.lines().count();
        let lines_out = formatted.lines().count();
        let changed_lines = src.lines().zip(formatted.lines()).filter(|(a, b)| a != b).count();
        println!(
            "{name:<14} parse_errors={errors:<5} lines={lines_in}→{lines_out} \
             changed_lines={changed_lines} bytes_delta={bytes_diff:+}"
        );
    }
}
