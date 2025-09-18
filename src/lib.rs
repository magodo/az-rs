use anyhow::Result;
use api::{invoke::CommandInvocation, ApiManager};
use arg::CliInput;
use clap::{ArgMatches, Command};
use client::Client;
use std::path::PathBuf;

pub mod api;
pub mod arg;
pub mod azidentityext;
pub mod client;
pub mod cmd;

#[cfg(target_arch = "wasm32")]
pub mod wasm_exports;

pub async fn run(p: PathBuf, client: &Client, raw_input: Vec<String>) -> Result<String> {
    let api_manager = ApiManager::new(p)?;

    let matches = get_matches(cmd::cmd(), raw_input.clone())?;

    match matches.subcommand() {
        Some(("api", matches)) => {
            let args = if let Some(args) = matches.get_many::<String>("args") {
                args.cloned().collect()
            } else {
                vec![]
            };
            let input = CliInput::new(args)?;
            let mut matches = get_matches(cmd::cmd_api(&api_manager, &input), raw_input.clone())?;

            // Invoke the api call
            while let Some((_, m)) = matches.subcommand() {
                matches = m.clone();
            }
            let index = api_manager.read_index(input.pos_args().first().unwrap())?;
            let command_file = index.locate_command_file(input.api_version(), &input)?;
            let command = api_manager.read_command(&command_file)?;
            let invoker = CommandInvocation::new(&command, &matches)?;
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
