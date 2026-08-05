use std::{env, process};

use sudolang_lsp::{cli, Backend};
use tower_lsp::{LspService, Server};

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    match args.first().map(String::as_str) {
        // No arguments: the editor case. Serve the LSP over stdio.
        None => serve().await,
        Some("check") => {
            let report = cli::check_paths(&args[1..]);
            print!("{}", report.output);
            process::exit(report.code);
        }
        Some("fmt") => {
            let report = cli::fmt_paths(&args[1..]);
            print!("{}", report.output);
            process::exit(report.code);
        }
        Some("--help" | "-h" | "help") => println!("{}", cli::USAGE),
        Some("--version" | "-V") => {
            println!("sudolang-lsp {}", env!("CARGO_PKG_VERSION"));
        }
        Some(other) => {
            eprintln!("error: unknown argument `{other}`\n\n{}", cli::USAGE);
            process::exit(2);
        }
    }
}

async fn serve() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
