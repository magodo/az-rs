use anyhow::Result;
use az_rs::log::set_global_logger;
use az_rs::run;
use azure_core::credentials::TokenCredential;
use azure_identity::DefaultAzureCredential;
use std::{env, path::PathBuf, str::FromStr, sync::Arc};

#[tokio::main]
async fn main() -> Result<()> {
    set_global_logger();

    let cred_func = || -> Result<Arc<dyn TokenCredential>> {
        let cred = DefaultAzureCredential::new()?;
        Ok(cred)
    };

    let res = run(
        PathBuf::from_str("./metadata/metadata")?,
        env::args_os()
            .into_iter()
            .map(|s| s.into_string().unwrap())
            .collect(),
        cred_func,
    )
    .await?;
    println!("{res}");
    Ok(())
}
