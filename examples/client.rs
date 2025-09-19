use anyhow::Result;
use az_rs::client::Client;
use azure_core::http::Method::{Delete, Get, Put};
use azure_identity::DefaultAzureCredential;
use bytes::Bytes;
use clap::Parser;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[arg(short, long)]
    id: String,

    #[arg(short, long)]
    api_version: String,

    #[arg(short, long)]
    body: Option<Bytes>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let credential = DefaultAzureCredential::new()?;
    let client = Client::new(
        "https://management.azure.com",
        vec!["https://management.azure.com/.default"],
        credential,
        None,
    )?;

    let resp = client
        .run(Put, &args.id, &args.api_version, args.body, None)
        .await?;
    println!("PUT response: {}", String::from_utf8(resp.body.to_vec())?);

    let resp = client
        .run(Get, &args.id, &args.api_version, None, None)
        .await?;
    println!("GET response: {}", String::from_utf8(resp.body.to_vec())?);

    let resp = client
        .run(Delete, &args.id, &args.api_version, None, None)
        .await?;
    println!(
        "DELETE response: {}",
        String::from_utf8(resp.body.to_vec())?
    );

    Ok(())
}
