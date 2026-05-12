use dashmap::DashMap;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::completion;
use crate::definition;
use crate::diagnostics;
use crate::document::Document;
use crate::formatter;
use crate::hover;

pub struct Backend {
    client: Client,
    documents: DashMap<Url, Document>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: DashMap::new(),
        }
    }

    async fn refresh_diagnostics(&self, uri: Url) {
        let Some(doc) = self.documents.get(&uri) else {
            return;
        };
        let diags = diagnostics::collect(&doc.tree, &doc.text);
        let version = Some(doc.version);
        drop(doc);
        self.client
            .publish_diagnostics(uri, diags, version)
            .await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _params: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "sudolang-lsp".into(),
                version: Some(env!("CARGO_PKG_VERSION").into()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                document_formatting_provider: Some(OneOf::Left(true)),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![
                        ".".into(),
                        "/".into(),
                        "$".into(),
                    ]),
                    ..CompletionOptions::default()
                }),
                definition_provider: Some(OneOf::Left(true)),
                ..ServerCapabilities::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "sudolang-lsp initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let text = params.text_document.text;
        let version = params.text_document.version;
        if let Some(doc) = Document::new(text, version) {
            self.documents.insert(uri.clone(), doc);
            self.refresh_diagnostics(uri).await;
        }
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        // TextDocumentSyncKind::FULL — last change carries the whole text.
        let Some(change) = params.content_changes.into_iter().last() else {
            return;
        };
        let text = change.text;
        let version = params.text_document.version;

        if let Some(mut doc) = self.documents.get_mut(&uri) {
            doc.update(text, version);
        } else if let Some(doc) = Document::new(text, version) {
            self.documents.insert(uri.clone(), doc);
        }
        self.refresh_diagnostics(uri).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.documents.remove(&uri);
        self.client.publish_diagnostics(uri, vec![], None).await;
    }

    async fn formatting(
        &self,
        params: DocumentFormattingParams,
    ) -> Result<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri;
        let Some(doc) = self.documents.get(&uri) else {
            return Ok(None);
        };

        if formatter::tree_has_errors(&doc.tree) {
            self.client
                .log_message(
                    MessageType::INFO,
                    "sudolang-lsp: refusing to format — document has parse errors",
                )
                .await;
            return Ok(None);
        }

        let formatted = formatter::format(&doc.text, &doc.tree);
        if formatted == doc.text {
            return Ok(Some(vec![]));
        }

        let end = end_of_document_position(&doc.text);
        let edit = TextEdit {
            range: Range {
                start: Position::new(0, 0),
                end,
            },
            new_text: formatted,
        };
        Ok(Some(vec![edit]))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        let Some(doc) = self.documents.get(&uri) else {
            return Ok(None);
        };
        Ok(hover::hover(&doc.tree, &doc.text, position))
    }

    async fn completion(
        &self,
        params: CompletionParams,
    ) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let Some(doc) = self.documents.get(&uri) else {
            return Ok(None);
        };
        let items = completion::complete(&doc.tree, &doc.text);
        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        let Some(doc) = self.documents.get(&uri) else {
            return Ok(None);
        };
        let locs = definition::definitions(&doc.tree, &doc.text, &uri, position);
        if locs.is_empty() {
            return Ok(None);
        }
        Ok(Some(GotoDefinitionResponse::Array(locs)))
    }
}

fn end_of_document_position(text: &str) -> Position {
    let mut line: u32 = 0;
    let mut last_line_start = 0usize;
    let bytes = text.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'\n' {
            line += 1;
            last_line_start = i + 1;
        }
    }
    let character = text[last_line_start..].encode_utf16().count() as u32;
    Position { line, character }
}
