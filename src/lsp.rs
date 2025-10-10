use backend::Backend;

pub mod backend;

pub async fn serve() {
    tracing::info!(
        "Az Language Server version \"{}\" starts.",
        env!("CARGO_PKG_VERSION")
    );

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = tower_lsp::LspService::build(|client| Backend::new(client)).finish();

    tower_lsp::Server::new(stdin, stdout, socket)
        .serve(service)
        .await;

    tracing::info!("Az LSP Server did shut down.");
}
