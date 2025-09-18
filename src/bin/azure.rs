use anyhow::Result;
use az_rs::client::Client;
use az_rs::log::set_global_logger;
use az_rs::run;
use azure_identity::DefaultAzureCredential;
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
