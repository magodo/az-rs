use anyhow::Result;
use azure::client::Client;
use azure::run;
use azure_identity::DefaultAzureCredential;
use std::{env, path::PathBuf, str::FromStr};

#[tokio::main]
async fn main() -> Result<()> {
    let credential = DefaultAzureCredential::new()?;
    let client = Client::new(
        "https://management.azure.com",
        vec!["https://management.azure.com/.default"],
        credential,
        None,
    )?;
    let res = run(
        PathBuf::from_str("./metadata/metadata")?,
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
