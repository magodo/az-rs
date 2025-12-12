use anyhow::{Context, Result, anyhow, bail};
use api::{
    ApiManager,
    cli_expander::{CLIExpander, Shell},
    invoke::OperationInvocation,
};
use arg::CliInput;
use azure_core::credentials::TokenCredential;
use clap::{ArgMatches, Command};
use client::Client;
use std::{path::PathBuf, str::FromStr, sync::Arc};

use crate::azidentityext::profile::FileSystemProfileManager;

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

pub async fn run(
    metadata_path: PathBuf,
    raw_input: Vec<String>,
    credential: Option<Arc<dyn TokenCredential>>,
) -> Result<String> {
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
            let api_manager = ApiManager::new(&metadata_path)?;
            let cmd = cmd::cmd_api(&api_manager, &input);
            let mut matches = get_matches(cmd, raw_input.clone())?;

            // Reaches here indicates an API command/operation is specified.

            // Match the subcommand to the end, which returns the matches for the last subcommand.
            while let Some((_, m)) = matches.subcommand() {
                matches = m.clone();
            }

            // Locate the operation
            let command_file = api_manager.locate_command_file(&input)?;
            let cmd_metadata = api_manager.read_command(&command_file)?;
            let cmd_cond = cmd_metadata.match_condition(&matches);
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
                        metadata_path.to_string_lossy().as_ref(),
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
                if let Some(shell) = matches.get_one::<String>("print-cli") {
                    let shell = Shell::from_str(shell.as_str())?;
                    let expander =
                        CLIExpander::new(&shell, &cmd_metadata.arg_groups, &raw_input, body);
                    return expander.expand();
                }
            }

            // Invoke the operation
            let invoker = OperationInvocation::new(operation, &matches, &body);
            let client = Client::new(
                "https://management.azure.com",
                vec!["https://management.azure.com/.default"],
                credential.expect("Login required to invoke API"),
                None,
            )?;
            let res = invoker.invoke(&client).await?;
            return Ok(res);
        }
        Some(("login", matches)) => {
            use azidentityext::login::Login;
            use azidentityext::login::interactive_browser::{
                InteractiveBrowserLogin, InteractiveBrowserLoginOptions,
            };
            use azidentityext::profile::ProfileManager;
            let options = InteractiveBrowserLoginOptions {
                tenant_id: matches
                    .get_one::<String>("tenant-id")
                    .cloned()
                    .expect("tenant-id is required"),
                client_id: "04b07795-8ddb-461a-bbee-02f9e1bf7b46".to_string(),
                client_secret: None,
                redirect_port: 47828,
                scopes: vec![
                    "https://management.core.windows.net//.default".to_string(),
                    "offline_access".to_string(),
                ],
                prompt: Some("select_account".to_string()),
                login_hint: Some("user@example.com".to_string()),
                success_template: "<html><body><h1>Login Successful</h1></body></html>".to_string(),
                error_template: "<html><body><h1>Login Failed</h1></body></html>".to_string(),
                server_timeout: std::time::Duration::from_secs(300),
            };
            let login = InteractiveBrowserLogin;
            let http_client = azure_core::http::new_http_client();
            let session = login
                .login(http_client.clone(), options)
                .await
                .expect("Login failed");
            let profile_manager = FileSystemProfileManager::new(
                std::env::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".az-rs")
                    .join("profile.json"),
            );
            profile_manager
                .login(&azidentityext::profile::AuthSession::RefreshTokenSession(
                    session,
                ))
                .await
                .expect("Login successful but failed to save profile");
            Ok("Login successful".to_string())
        }
        Some(("logout", _matches)) => {
            use azidentityext::profile::ProfileManager;
            let profile_manager = FileSystemProfileManager::new(
                std::env::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".az-rs")
                    .join("profile.json"),
            );
            profile_manager.logout().await.expect("Logout failed");
            Ok("Logout successful".to_string())
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
