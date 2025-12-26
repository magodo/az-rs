use anyhow::{anyhow, bail, Context, Result};
use azure_core::credentials::TokenCredential;
use clap::ArgMatches;
use metadata_index::Index;
use std::collections::HashMap;
use std::io::{self, BufRead};
use std::{path::PathBuf, sync::Arc};

use std::str::FromStr;

use crate::api::metadata_command::ConditionOpt;
use crate::cmd::{self, STDIN_OPTION};
use crate::{
    api::{
        cli_expander::{CLIExpander, Shell},
        invoke::OperationInvocation,
    },
    arg::CliInput,
    client::Client,
};
pub mod cli_expander;
pub mod invoke;
pub mod metadata_command;
pub mod metadata_index;

#[derive(Debug, Clone)]
pub struct ApiManager {
    pub root_path: PathBuf,
    pub index: Index,
    #[allow(dead_code)]
    commands_path: PathBuf,
}

impl ApiManager {
    pub async fn run<F>(
        &self,
        subcommands: &Vec<String>,
        args: &CliInput,
        matches: &ArgMatches,
        cred_func: F,
    ) -> Result<String>
    where
        F: FnOnce() -> Result<Arc<dyn TokenCredential>>,
    {
        let cred = cred_func()?;

        // Print CLI and quit
        let print_cli = matches.get_one::<String>("print-cli").map(|v| v);

        // Locate the command metadata
        let command_file = self.index.locate_command_file(args)?;
        let cmd_metadata = self.read_command(&command_file)?;

        if matches.get_flag(STDIN_OPTION) {
            // Read the id and (optionally, only for PUT) body from stdin, where each line shall be a JSON object containing
            // the '.id' and other body attributes.
            let handle = io::stdin().lock();
            let mut results = vec![];
            for line_result in handle.lines() {
                let line = line_result?;
                let mut obj: serde_json::Map<String, serde_json::Value> =
                    serde_json::from_str(&line)?;
                let id = obj
                    .get("id")
                    .ok_or(anyhow!(r#""id" field not found"#))?
                    .as_str()
                    .ok_or(anyhow!(r#""id" field is not a str"#))?
                    .to_string();

                // Locate the operation
                let condition_opt = ConditionOpt::new(Some(id.clone()), None);
                let cmd_cond = cmd_metadata.build_condition(condition_opt);
                let operation = cmd_metadata
                .select_operation_by_cond(cmd_cond.as_ref())
                .ok_or(anyhow!(
                    "failed to select the operation out from multiple operations available for this command based on the input"
                ))?;

                let mut body = None;
                if operation.is_put() {
                    obj.remove("id").unwrap();
                    let mut obj = serde_json::Value::Object(obj);
                    if let Some(schema) = operation
                        .http
                        .as_ref()
                        .and_then(|http| http.request.body.as_ref())
                        .and_then(|b| b.json.schema.as_ref())
                    {
                        schema.shake_body(&mut obj)?;
                    }
                    body = Some(obj);
                }

                if let Some(shell) = print_cli {
                    let shell = Shell::from_str(shell.as_str())?;
                    let expander = CLIExpander::new(
                        &shell,
                        &cmd_metadata.arg_groups,
                        args,
                        body,
                        Some(id.clone()),
                    );
                    let args = expander.expand()?;
                    let mut cli = vec![];
                    cli.extend(subcommands.iter().cloned());
                    cli.extend(args);
                    let result = cli.join(" ");
                    results.push(result);
                    continue;
                }

                // Invoke the operation
                let invoker = OperationInvocation::new(operation, &matches, &Some(id), &body);
                let client = Client::new(
                    "https://management.azure.com",
                    vec!["https://management.azure.com/.default"],
                    cred.clone(),
                    None,
                )?;
                let result = invoker.invoke(&client).await?;
                results.push(result);
            }
            return Ok(results.join("\n"));
        }

        // Locate the operation (for metadata that contains multiple operations by conditions)
        let name_args = cmd_metadata
            .arg_groups
            .iter()
            .find(|ag| ag.name == "")
            .and_then(|ag| {
                Some(
                    ag.args
                        .iter()
                        .filter(|arg| !arg.hide.unwrap_or(false))
                        .filter(|arg| arg.id_part.is_some())
                        .map(|arg| {
                            (
                                arg.var.clone(),
                                matches.get_one::<String>(&arg.var).cloned(),
                            )
                        })
                        .collect::<HashMap<_, _>>(),
                )
            });
        let id_arg = matches.get_one::<String>(cmd::ID_OPTION).cloned();
        let condition_opt = ConditionOpt::new(id_arg, name_args);

        let cmd_cond = cmd_metadata.build_condition(condition_opt);
        let operation = cmd_metadata
                .select_operation_by_cond(cmd_cond.as_ref())
                .ok_or(anyhow!(
                    "failed to select the operation out from multiple operations available for this command based on the input"
                ))?;

        let mut body = None;
        if operation.contains_request_body() {
            let mut hcl_body = None;
            if let Some(p) = matches.get_one::<PathBuf>("file") {
                // Read the HCL from file
                hcl_body = Some(get_file(&p)?);
            } else if matches.get_flag("edit") {
                // Read the HCL from editor
                let header = "# ...".to_string();
                let content = edit(
                    &header,
                    self.root_path.to_string_lossy().as_ref(),
                    &command_file,
                    cmd_cond.as_ref(),
                )?;
                let content = content.trim();

                // If the content is "empty", pause the process and exit.
                // This behavior is similar to "git commit".
                if content == header || content.is_empty() {
                    bail!("Aborting due to empty body");
                }

                hcl_body = Some(content.to_string());
            }

            body = if let Some(hcl_body) = hcl_body {
                let body = hcl::parse(&hcl_body).context("parsing the file as HCL")?;
                let v: serde_json::Value = hcl::from_body(body)?;
                Some(v)
            } else {
                None
            };

            // Print CLI and quit
            if let Some(shell) = print_cli {
                let shell = Shell::from_str(shell.as_str())?;
                let expander = CLIExpander::new(&shell, &cmd_metadata.arg_groups, args, body, None);
                let args = expander.expand()?;
                let mut cli = vec![];
                cli.extend(subcommands.iter().cloned());
                cli.extend(args);
                return Ok(cli.join(" "));
            }
        }

        // Invoke the operation
        let invoker = OperationInvocation::new(
            operation,
            &matches,
            &matches.get_one::<String>(cmd::ID_OPTION).cloned(),
            &body,
        );
        let client = Client::new(
            "https://management.azure.com",
            vec!["https://management.azure.com/.default"],
            cred,
            None,
        )?;
        invoker.invoke(&client).await
    }
}

#[cfg(any(feature = "embed-api", target_arch = "wasm32"))]
mod embedded {
    use super::metadata_command::Command;
    use anyhow::{anyhow, Result};
    use std::path::PathBuf;

    use rust_embed::RustEmbed;

    #[derive(RustEmbed)]
    #[folder = "metadata/metadata"]
    struct Asset;

    impl super::ApiManager {
        pub fn new(_: &PathBuf) -> Result<Self> {
            let bytes: Vec<u8> = Asset::get("index.json")
                .map(|d| d.data.to_vec())
                .ok_or(anyhow!("index.json doesn't exist"))?;
            let index = serde_json::from_slice(&bytes)?;

            Ok(Self {
                root_path: PathBuf::new(),
                index,
                commands_path: PathBuf::new(),
            })
        }

        pub fn read_command(&self, command_file: &str) -> Result<Command> {
            let bytes: Vec<u8> = Asset::get(format!("commands/{}", command_file).as_str())
                .map(|d| d.data.to_vec())
                .ok_or(anyhow!("{command_file} doesn't exist"))?;
            Ok(serde_json::from_slice(&bytes)?)
        }
    }
}

#[cfg(not(any(feature = "embed-api", target_arch = "wasm32")))]
mod fs {
    use super::metadata_command::Command;
    use anyhow::{Context, Result};
    use std::fs::read;
    use std::path::PathBuf;

    impl super::ApiManager {
        pub fn new(path: &PathBuf) -> Result<Self> {
            // TODO: Validate the files
            let index_path = path.join("index.json");
            let commands_path = path.join("commands");
            let bytes = read(index_path).context(format!("reading the index file"))?;
            let index = serde_json::from_slice(&bytes)?;
            Ok(Self {
                root_path: path.clone(),
                index,
                commands_path,
            })
        }

        pub fn read_command(&self, command_file: &str) -> Result<Command> {
            let bytes = read(self.commands_path.join(command_file))
                .context(format!("reading {command_file}"))?;
            Ok(serde_json::from_slice(&bytes)?)
        }
    }
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
fn edit(_: &String, _: &str, _: &str, _: Option<&String>) -> Result<String> {
    Err(anyhow!(r#""--edit" is not supported on wasm32"#))
}

#[cfg(not(target_arch = "wasm32"))]
fn edit(
    content: &String,
    metadata_path: &str,
    cmd_file: &str,
    cmd_cond: Option<&String>,
) -> Result<String> {
    use crate::lsp;

    let mut envs = std::collections::HashMap::from([
        (lsp::LSP_METADATA_PATH, metadata_path),
        (lsp::LSP_CMD_FILE, cmd_file),
    ]);
    if let Some(cmd_cond) = cmd_cond {
        envs.insert(lsp::LSP_CMD_CONDITION, cmd_cond);
    }
    Ok(
        edit::edit_with_builder_with_env(content, tempfile::Builder::new().suffix(".az"), envs)?
            .to_string(),
    )
}
