use anyhow::Result;
use azure_identity::DefaultAzureCredential;
use azure_rs::client::Client;
use azure_rs::log::set_global_logger;
use azure_rs::run;
use std::{env, path::PathBuf, str::FromStr};

#[tokio::main]
async fn main() -> Result<()> {
    set_global_logger();

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
