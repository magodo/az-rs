use anyhow::{Context, Result};
use api::{invoke::CommandInvocation, ApiManager};
use arg::CliInput;
use azure_core::credentials::TokenCredential;
use clap::{ArgMatches, Command};
use client::Client;
use std::{path::PathBuf, sync::Arc};

pub mod api;
pub mod arg;
pub mod azidentityext;
pub mod client;
pub mod cmd;
pub mod log;

#[cfg(target_arch = "wasm32")]
pub mod wasm_exports;

pub async fn run<F>(metadata_path: PathBuf, raw_input: Vec<String>, cred_func: F) -> Result<String>
where
    F: FnOnce() -> Result<Arc<dyn TokenCredential>>,
{
    tracing::info!("Running CLI with input: {:?}", raw_input);
    let matches = get_matches(cmd::cmd(), raw_input.clone())?;

    match matches.subcommand() {
        Some(("api", matches)) => {
            let args = if let Some(args) = matches.get_many::<String>("args") {
                args.cloned().collect()
            } else {
                vec![]
            };
            let input = CliInput::new(args)?;
            let api_manager = ApiManager::new(metadata_path)?;
            let (cmd, cmd_metadata) = cmd::cmd_api(&api_manager, &input);
            let mut matches = get_matches(cmd, raw_input.clone())?;

            // Reaches here indicates an API command/operation is specified.
            let cmd_metadata = cmd_metadata.unwrap();

            // Match the subcommand to the end, which returns the matches for the last subcommand.
            while let Some((_, m)) = matches.subcommand() {
                matches = m.clone();
            }

            let mut body = None;
            if let Some(p) = matches.get_one::<String>("input") {
                body = Some(get_input(p.as_str())?);
            }
            let invoker = CommandInvocation::new(&cmd_metadata, &matches, body)?;

            let cred = cred_func()?;
            let client = Client::new(
                "https://management.azure.com",
                vec!["https://management.azure.com/.default"],
                cred,
                None,
            )?;

            let res = invoker.invoke(&client).await?;
            return Ok(res);
        }
        _ => unreachable!("Exhausted list of subcommands and subcommand_required prevents `None`"),
    }
}

#[cfg(target_arch = "wasm32")]
pub fn get_matches(cmd: Command, input: Vec<String>) -> Result<ArgMatches> {
    use anyhow::anyhow;
    use clap::builder::Styles;
    let cmd = cmd.styles(Styles::plain());
    cmd.try_get_matches_from(input)
        .map_err(|e| anyhow!("{}", e.render().ansi()))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn get_matches(cmd: Command, input: Vec<String>) -> Result<ArgMatches> {
    Ok(cmd.get_matches_from(input))
}

#[cfg(target_arch = "wasm32")]
pub fn get_input(p: PathBuf) -> Result<bytes::Bytes> {
    bail!(r#""--input" is not supported on wasm32"#);
}

#[cfg(not(target_arch = "wasm32"))]
pub fn get_input(p: &str) -> Result<bytes::Bytes> {
    let input = std::fs::read_to_string(p).context("reading the input from {p}")?;
    let body = hcl::parse(&input).context("parsing the input as HCL")?;
    let v: serde_json::Value = hcl::from_body(body)?;
    Ok(bytes::Bytes::from(v.to_string()))
}
