use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use tower_lsp::{
    jsonrpc::Result,
    lsp_types::{
        ClientInfo, CompletionItem, CompletionOptions, CompletionParams, CompletionResponse,
        DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
        DocumentDiagnosticParams, DocumentDiagnosticReport, DocumentDiagnosticReportResult,
        FullDocumentDiagnosticReport, Hover, HoverContents, HoverParams, HoverProviderCapability,
        InitializeParams, InitializeResult, InitializedParams, MarkedString, PositionEncodingKind,
        RelatedFullDocumentDiagnosticReport, ServerCapabilities, TextDocumentSyncCapability,
        TextDocumentSyncKind, Url,
    },
    Client, LanguageServer,
};

use crate::api::metadata_command::Command;

use super::document::Document;

pub struct Backend {
    client: Client,
    cmd: Command,
    documents: Arc<RwLock<HashMap<tower_lsp::lsp_types::Url, Document>>>,
}

impl Backend {
    pub fn new(client: Client, cmd: Command) -> Self {
        Self {
            client,
            cmd,
            documents: Default::default(),
        }
    }

    async fn reset_diagnostics(&self, document_uri: &Url) {
        self.client
            .publish_diagnostics(document_uri.clone(), Vec::new(), None)
            .await;
    }

    async fn publish_diagnostics(&self, document_uri: &Url) {
        let diags;
        {
            let documents = self.documents.read().unwrap();
            let Some(document) = documents.get(document_uri) else {
                return;
            };
            diags = document.get_diagnostics();
        }

        self.client
            .publish_diagnostics(document_uri.clone(), diags, None)
            .await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    #[tracing::instrument(level = "debug", skip_all)]
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        tracing::debug!("message received");
        tracing::trace!(?params);

        let InitializeParams { client_info, .. } = params;

        if let Some(ClientInfo { name, version }) = client_info {
            let version = version.unwrap_or_default();
            tracing::info!("{name} version: {version}",);
        }

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                position_encoding: Some(PositionEncodingKind::UTF16),
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions::default()),
                // TODO: Enable the pull-style diagnostics will cause double diagnostics: pulled
                // and pushed.
                //
                // diagnostic_provider: Some(DiagnosticServerCapabilities::Options(
                //     DiagnosticOptions {
                //         inter_file_dependencies: false,
                //         workspace_diagnostics: false,
                //         ..Default::default()
                //     },
                // )),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    #[tracing::instrument(level = "debug", skip_all)]
    async fn initialized(&self, params: InitializedParams) {
        tracing::debug!("message received");
        tracing::trace!(?params);
    }

    #[tracing::instrument(level = "debug", skip_all)]
    async fn shutdown(&self) -> Result<()> {
        tracing::debug!("message received");
        Ok(())
    }

    #[tracing::instrument(level = "debug", skip_all)]
    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        tracing::debug!("message received");
        tracing::trace!(?params);
        let doc = params.text_document;
        {
            let mut documents = self.documents.write().unwrap();
            documents.insert(doc.uri.clone(), Document::new(&doc.text));
            tracing::debug!(doc.text);
        }
        self.publish_diagnostics(&doc.uri).await;
    }

    #[tracing::instrument(level = "debug", skip_all)]
    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        tracing::debug!("message received");
        tracing::trace!(?params);
        let doc = params.text_document;
        {
            let mut documents = self.documents.write().unwrap();
            let Some(document) = documents.get_mut(&doc.uri) else {
                return;
            };

            for change in &params.content_changes {
                document.apply_change(change);
            }
        }
        self.publish_diagnostics(&doc.uri).await;
    }

    #[tracing::instrument(level = "debug", skip_all)]
    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        tracing::debug!("message received");
        tracing::trace!(?params);
        let doc = params.text_document;
        self.reset_diagnostics(&doc.uri).await;
    }

    #[tracing::instrument(level = "debug", skip_all)]
    async fn diagnostic(
        &self,
        params: DocumentDiagnosticParams,
    ) -> Result<DocumentDiagnosticReportResult> {
        let documents = self.documents.read().unwrap();
        let Some(document) = documents.get(&params.text_document.uri) else {
            return Ok({
                DocumentDiagnosticReportResult::Report(DocumentDiagnosticReport::Full(
                    RelatedFullDocumentDiagnosticReport::default(),
                ))
            });
        };
        Ok({
            DocumentDiagnosticReportResult::Report(DocumentDiagnosticReport::Full(
                RelatedFullDocumentDiagnosticReport {
                    full_document_diagnostic_report: FullDocumentDiagnosticReport {
                        items: document.get_diagnostics(),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ))
        })
    }

    #[tracing::instrument(level = "debug", skip_all)]
    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        tracing::debug!("message received");
        tracing::trace!(?params);
        Ok(Some(CompletionResponse::Array(vec![
            CompletionItem::new_simple("Hello".to_string(), "Some detail".to_string()),
            CompletionItem::new_simple("Bye".to_string(), "More detail".to_string()),
        ])))
    }

    #[tracing::instrument(level = "debug", skip_all)]
    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        tracing::debug!("message received");
        tracing::trace!(?params);
        Ok(Some(Hover {
            contents: HoverContents::Scalar(MarkedString::String("You're hovering!".to_string())),
            range: None,
        }))
    }
}
