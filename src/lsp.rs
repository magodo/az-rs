use std::env;

use backend::Backend;

pub mod backend;
mod document;

pub const LSP_CMD_METADATA_VAR: &str = "AZURE_CMD_METADATA";

#[tracing::instrument]
pub async fn serve() {
    tracing::info!(
        "Az Language Server version \"{}\" starts.",
        env!("CARGO_PKG_VERSION")
    );

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let cmd_json = env::var(LSP_CMD_METADATA_VAR)
        .expect(format!(r#"environment variable "{LSP_CMD_METADATA_VAR}""#).as_str());
    let cmd = serde_json::from_str(&cmd_json).expect(r#"decode command metadata from JSON"#);

    let (service, socket) =
        tower_lsp::LspService::build(|client| Backend::new(client, cmd)).finish();

    tower_lsp::Server::new(stdin, stdout, socket)
        .serve(service)
        .await;

    tracing::info!("Az LSP Server did shut down.");
}
