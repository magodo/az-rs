use anyhow::{anyhow, Context, Result};
use api::{cli_expander::CLIExpander, invoke::OperationInvocation, ApiManager};
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

            // Read the raw HCL body
            let hcl_body = if let Some(p) = matches.get_one::<PathBuf>("file") {
                Some(get_file(&p)?)
            } else if matches.get_flag("edit") {
                Some(edit("# ...".to_string())?)
            } else {
                None
            };

            // Convert the HCL body to JSON value
            let body = if let Some(hcl_body) = hcl_body {
                let body = hcl::parse(&hcl_body).context("parsing the file as HCL")?;
                let v: serde_json::Value = hcl::from_body(body)?;
                Some(v)
            } else {
                None
            };

            // Select the operation based on the user's input
            let operation = cmd_metadata
                .select_operation(&matches)
                .ok_or(anyhow!("no operation is selected"))?;

            // Print CLI and quit
            if matches.get_flag("print-cli") {
                let expander = CLIExpander::new(&operation, &raw_input, body);
                return expander.expand();
            }

            // Invoke the operation
            let invoker = OperationInvocation::new(operation, &matches, &body);
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
fn get_matches(cmd: Command, input: Vec<String>) -> Result<ArgMatches> {
    use anyhow::anyhow;
    use clap::builder::Styles;
    let cmd = cmd.styles(Styles::plain());
    cmd.try_get_matches_from(input)
        .map_err(|e| anyhow!("{}", e.render().ansi()))
}

#[cfg(not(target_arch = "wasm32"))]
fn get_matches(cmd: Command, input: Vec<String>) -> Result<ArgMatches> {
    Ok(cmd.get_matches_from(input))
}

#[cfg(target_arch = "wasm32")]
fn get_file(p: &PathBuf) -> Result<String> {
    bail!(r#""--file" is not supported on wasm32"#);
}

#[cfg(not(target_arch = "wasm32"))]
fn get_file(p: &PathBuf) -> Result<String> {
    std::fs::read_to_string(p).context("reading file from {p}")
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    pub fn edit_js(content: &str) -> &str;
}

#[cfg(target_arch = "wasm32")]
fn edit(content: String) -> Result<String> {
    Ok(edit_js(&content).to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn edit(content: String) -> Result<String> {
    Ok(edit::edit(&content)?.to_string())
}
