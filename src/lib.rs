use anyhow::Result;
use api::ApiManager;
use arg::CliInput;
use azure_core::credentials::TokenCredential;
use clap::{ArgMatches, Command};
use std::{path::PathBuf, sync::Arc};

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

pub async fn run<CF, RF>(
    metadata_path: PathBuf,
    raw_input: Vec<String>,
    cred_func: CF,
    mut resp_func: RF,
) -> Result<()>
where
    CF: FnOnce() -> Result<Arc<dyn TokenCredential>>,
    RF: FnMut(String) -> (),
{
    tracing::info!("Running CLI with input: {:?}", raw_input);
    let matches = get_matches(cmd::cmd(), raw_input.clone())?;

    match matches.subcommand() {
        #[cfg(not(target_arch = "wasm32"))]
        Some(("lsp", _)) => {
            lsp::serve().await;
            resp_func("".to_string());
            return Ok(());
        }

        Some(("api", matches)) => {
            let args = if let Some(args) = matches.get_many::<String>("args") {
                args.cloned().collect()
            } else {
                vec![]
            };
            let args = CliInput::new(args)?;
            let api_manager = ApiManager::new(&metadata_path)?;
            let cmd = cmd::cmd_api(&api_manager, &args);
            let mut matches = get_matches(cmd, raw_input.clone())?;

            // Reaches here indicates an API command/operation is specified.

            // Match the subcommand to the end, which returns the matches for the last subcommand.
            let mut subcommands = vec![raw_input[0].to_string()];
            while let Some((cmd, m)) = matches.subcommand() {
                subcommands.push(cmd.to_string());
                matches = m.clone();
            }

            api_manager
                .run(&subcommands, &args, &matches, cred_func, resp_func)
                .await?;
            return Ok(());
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
