use anyhow::{anyhow, bail, Context, Result};
use api::{
    cli_expander::{CLIExpander, Shell},
    invoke::OperationInvocation,
    ApiManager,
};
use arg::CliInput;
use azure_core::credentials::TokenCredential;
use clap::{ArgMatches, Command};
use client::Client;
use std::{path::PathBuf, str::FromStr, sync::Arc};

pub mod api;
pub mod arg;
pub mod azidentityext;
pub mod client;
pub mod cmd;
pub mod log;

#[cfg(not(target_arch = "wasm32"))]
pub mod lsp;

#[cfg(target_arch = "wasm32")]
pub mod wasm_exports;

pub async fn run<F>(metadata_path: PathBuf, raw_input: Vec<String>, cred_func: F) -> Result<String>
where
    F: FnOnce() -> Result<Arc<dyn TokenCredential>>,
{
    tracing::info!("Running CLI with input: {:?}", raw_input);
    let matches = get_matches(cmd::cmd(), raw_input.clone())?;

    match matches.subcommand() {
        #[cfg(not(target_arch = "wasm32"))]
        Some(("lsp", _)) => {
            lsp::serve().await;
            Ok("".to_string())
        }

        Some(("api", matches)) => {
            let args = if let Some(args) = matches.get_many::<String>("args") {
                args.cloned().collect()
            } else {
                vec![]
            };
            let input = CliInput::new(args)?;
            let api_manager = ApiManager::new(metadata_path)?;
            let cmd = cmd::cmd_api(&api_manager, &input);
            let mut matches = get_matches(cmd, raw_input.clone())?;

            // Reaches here indicates an API command/operation is specified.
            let cmd_metadata = api_manager.locate_command_metadata(&input)?;

            // Match the subcommand to the end, which returns the matches for the last subcommand.
            while let Some((_, m)) = matches.subcommand() {
                matches = m.clone();
            }

            // Read the HCL body, if any.
            let mut hcl_body = None;
            if let Some(p) = matches.get_one::<PathBuf>("file") {
                hcl_body = Some(get_file(&p)?);
            } else if matches.get_flag("edit") {
                let header = "# ...".to_string();
                let cmd_json = serde_json::to_string(&cmd_metadata)?;
                let content = edit(&header, &cmd_json)?;
                let content = content.trim();

                // If the content is "empty", pause the process and exit.
                // This behavior is similar to "git commit".
                if content == header || content.is_empty() {
                    bail!("Aborting due to empty body");
                }

                hcl_body = Some(content.to_string());
            }

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
            if let Some(shell) = matches.get_one::<String>("print-cli") {
                let shell = Shell::from_str(shell.as_str())?;
                let expander = CLIExpander::new(&shell, &cmd_metadata.arg_groups, &raw_input, body);
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
fn get_file(_: &PathBuf) -> Result<String> {
    Err(anyhow!(r#""--file" is not supported on wasm32"#))
}

#[cfg(not(target_arch = "wasm32"))]
fn get_file(p: &PathBuf) -> Result<String> {
    std::fs::read_to_string(p).context(format!("reading file from {p:?}"))
}

#[cfg(target_arch = "wasm32")]
fn edit(_: &String, _: &String) -> Result<String> {
    Err(anyhow!(r#""--edit" is not supported on wasm32"#))
}

#[cfg(not(target_arch = "wasm32"))]
fn edit(content: &String, cmd_metadata: &String) -> Result<String> {
    Ok(edit::edit_with_builder_with_env(
        content,
        tempfile::Builder::new().suffix(".az"),
        std::collections::HashMap::from([(lsp::LSP_CMD_METADATA_VAR, cmd_metadata)]),
    )?
    .to_string())
}
