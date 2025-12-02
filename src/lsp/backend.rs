use std::{
    collections::HashMap,
    default,
    sync::{Arc, RwLock},
};

use tower_lsp::{
    jsonrpc::Result,
    lsp_types::{
        ClientInfo, CompletionOptions, CompletionParams, CompletionResponse,
        DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams, Hover,
        HoverParams, HoverProviderCapability, InitializeParams, InitializeResult,
        InitializedParams, PositionEncodingKind, SemanticTokenType, SemanticTokensLegend,
        SemanticTokensOptions, SemanticTokensParams, SemanticTokensResult,
        SemanticTokensServerCapabilities, ServerCapabilities, TextDocumentSyncCapability,
        TextDocumentSyncKind, Url,
    },
    Client, LanguageServer,
};

use crate::{api::metadata_command::Operation, lsp::semantic_tokens};

use super::document::Document;

pub struct Backend {
    client: Client,
    operation: Operation,
    documents: Arc<RwLock<HashMap<tower_lsp::lsp_types::Url, Document>>>,
}

impl Backend {
    pub fn new(client: Client, operation: &Operation) -> Self {
        Self {
            client,
            operation: operation.clone(),
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
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            legend: SemanticTokensLegend {
                                token_types: semantic_tokens::semantic_token_types()
                                    .iter()
                                    .map(|t| SemanticTokenType::from(t.to_string()))
                                    .collect(),
                                token_modifiers: vec![],
                            },
                            ..Default::default()
                        },
                    ),
                ),
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
    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        tracing::debug!("message received");
        tracing::trace!(?params);

        let doc = params.text_document_position.text_document;
        let documents = self.documents.read().unwrap();
        let Some(document) = documents.get(&doc.uri) else {
            return Ok(None);
        };
        if let Some(items) =
            document.complete(&self.operation, &params.text_document_position.position)
        {
            return Ok(Some(CompletionResponse::Array(items)));
        } else {
            return Ok(None);
        }
    }

    #[tracing::instrument(level = "debug", skip_all)]
    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        tracing::debug!("message received");
        tracing::trace!(?params);

        let doc = params.text_document_position_params.text_document;
        let documents = self.documents.read().unwrap();
        let Some(document) = documents.get(&doc.uri) else {
            return Ok(None);
        };
        Ok(document.hover(
            &self.operation,
            &params.text_document_position_params.position,
        ))
    }

    #[tracing::instrument(level = "debug", skip_all)]
    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        tracing::debug!("message received");
        tracing::trace!(?params);

        let doc = params.text_document;
        let documents = self.documents.read().unwrap();
        let Some(document) = documents.get(&doc.uri) else {
            return Ok(None);
        };
        Ok(document.semantic_tokens_full())
    }
}
