use anyhow::Result;
use azure::client::Client;
use azure::run;
use azure_identity::DefaultAzureCredential;
use std::{env, io, path::PathBuf, str::FromStr};

// Simple tracing setup without tracing-web
fn init_tracing_simple() {
    use tracing_subscriber::prelude::*;
    
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(io::stderr);                                                                                                                                                                                                                                                                                                                                                       
    
    tracing_subscriber::registry()
        .with(fmt_layer)
        .init();
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing_simple();

    let credential = DefaultAzureCredential::new()?;
    let client = Client::new(
        "https://management.azure.com",
        vec!["https://management.azure.com/.default"],
        credential,
        None,
    )?;
    let res = run(
        PathBuf::from_str("./metadata")?,
        &client,
        env::args_os()
            .into_iter()
            .map(|s| s.into_string().unwrap())
            .collect(),
    )
    .await?;
    println!("{res}");
    Ok(())
}
