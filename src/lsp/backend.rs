use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use tower_lsp::{
    jsonrpc::Result,
    lsp_types::{
        ClientInfo, CompletionItem, CompletionOptions, CompletionParams, CompletionResponse,
        DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams, Hover,
        HoverContents, HoverParams, HoverProviderCapability, InitializeParams, InitializeResult,
        InitializedParams, MarkedString, PositionEncodingKind, ServerCapabilities,
        TextDocumentSyncCapability, TextDocumentSyncKind,
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
                // NOTE that we enforce to use UTF8 though the spec asks the server to always
                // support UTF16.
                // This is to ease the document offset implementation. In case there is
                // editor/client does only support UTF16, we shall consider update the code and
                // support UTF16.
                position_encoding: Some(PositionEncodingKind::UTF8),
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions::default()),
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
        // TODO: consider publish diags
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
        // TODO: consider publish diags
    }

    #[tracing::instrument(level = "debug", skip_all)]
    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        tracing::debug!("message received");
        tracing::trace!(?params);
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
