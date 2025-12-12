use anyhow::Result;
use az_rs::azidentityext::profile::ProfileManager;
use az_rs::{azidentityext::profile::FileSystemProfileManager, log::set_global_logger};
use az_rs::run;
use std::{env, path::PathBuf, str::FromStr, sync::Arc};

#[tokio::main]
async fn main() -> Result<()> {
    set_global_logger();

    let profile_manager = FileSystemProfileManager::new(
        std::env::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".az-rs")
            .join("profile.json"),
    );
    let http_client = azure_core::http::new_http_client();
    let credential = profile_manager
        .get_credential(http_client)
        .await?
        .map(Arc::from);

    let res = run(
        PathBuf::from_str("./metadata/metadata")?,
        env::args_os()
            .into_iter()
            .map(|s| s.into_string().unwrap())
            .collect(),
        credential,
    )
    .await?;
    println!("{res}");
    Ok(())
}
