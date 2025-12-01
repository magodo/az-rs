use std::str::FromStr;
use std::{env, path::PathBuf};

use backend::Backend;

use crate::api::ApiManager;

pub mod backend;
mod complete;
mod document;
mod hcl;

pub const LSP_METADATA_PATH: &str = "AZURE_LSP_METADATA_PATH";
pub const LSP_CMD_FILE: &str = "AZURE_LSP_CMD_FILE";
pub const LSP_CMD_CONDITION: &str = "AZURE_LSP_CMD_CONDITION";

#[tracing::instrument]
pub async fn serve() {
    tracing::info!(
        "Az Language Server version \"{}\" starts.",
        env!("CARGO_PKG_VERSION")
    );

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let metadata_path = env::var(LSP_METADATA_PATH)
        .expect(format!(r#"environment variable "{LSP_METADATA_PATH}""#).as_str());
    let api_manager = ApiManager::new(
        &PathBuf::from_str(&metadata_path).expect("converting metadata path to PathBuf"),
    )
    .expect("new ApiManager");

    let cmd_file =
        env::var(LSP_CMD_FILE).expect(format!(r#"environment variable "{LSP_CMD_FILE}""#).as_str());
    let cmd = api_manager
        .read_command(&cmd_file)
        .expect("read api command");

    let cond = env::var(LSP_CMD_CONDITION).ok();
    let Some(operation) = cmd.select_operation(cond.as_ref()) else {
        panic!("failed to select the API operation");
    };

    let (service, socket) =
        tower_lsp::LspService::build(|client| Backend::new(client, operation)).finish();

    tower_lsp::Server::new(stdin, stdout, socket)
        .serve(service)
        .await;

    tracing::info!("Az LSP Server did shut down.");
}
