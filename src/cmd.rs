use std::collections::HashMap;

use crate::api::{metadata_command, metadata_index, ApiManager};
use crate::arg::CliInput;
use clap::builder::PossibleValuesParser;
use clap::{command, Arg, Command};

pub fn cmd() -> Command {
    cmd_base().subcommand(cmd_api_stub())
}

fn cmd_base() -> Command {
    command!()
        .subcommand_required(true)
        .arg_required_else_help(true)
        .disable_help_subcommand(true)
}

fn cmd_api_stub() -> Command {
    cmd_api_base().disable_help_flag(true).arg(
        Arg::new("args")
            .num_args(0..)
            .trailing_var_arg(true)
            .allow_hyphen_values(true)
            .hide(true),
    )
}

fn cmd_api_base_real() -> Command {
    cmd_api_base()
        .subcommand_required(true)
        .arg_required_else_help(true)
}

pub fn cmd_api_base() -> Command {
    Command::new("api").about("Directly invoke the Azure API primitives.")
}

// cmd_api parses the raw CLI args for `api` subcommand, returns a precise clap::Command and
// a potential Command metadata (if the raw CLI args ends to a command).
pub fn cmd_api(
    api_manager: &ApiManager,
    input: &CliInput,
) -> (Command, Option<metadata_command::Command>) {
    let pos_args = input.pos_args();

    // No positional argument specified, list the rps
    if pos_args.is_empty() {
        let rps = &api_manager.index.rps;
        let mut keys: Vec<_> = rps.keys().collect();
        keys.sort();
        return (
            cmd_base().subcommand(cmd_api_base_real().subcommands(keys.iter().map(|k| {
                Command::new(*k).about(
                    rps.get(k.as_str())
                        .unwrap()
                        .help
                        .as_ref()
                        .map_or("".to_string(), |v| v.short.clone()),
                )
            }))),
            None,
        );
    }

    struct CommandDesc {
        name: String,
        help: Option<metadata_index::Help>,
    }

    let mut command_metadata = None;
    let rp = pos_args.first().unwrap();
    let cmd = match api_manager.index.rps.get(*rp) {
        Some(rp_meta) => {
            let mut args = pos_args.iter();
            let mut commands = vec![];

            // Construct a fake command group here to initiate the following while loop
            let mut cg = metadata_index::CommandGroup {
                command_groups: Some(HashMap::from([(
                    rp.to_string(),
                    metadata_index::CommandGroup {
                        help: rp_meta.help.clone(),
                        command_groups: rp_meta.command_groups.clone(),
                        commands: rp_meta.commands.clone(),
                    },
                )])),
                help: None,
                commands: None,
            };

            let mut c: Option<metadata_index::Command> = None;

            while let Some(arg) = args.next() {
                if let Some(v) = cg
                    .command_groups
                    .as_ref()
                    .and_then(|cg| cg.get(*arg).cloned())
                {
                    commands.push(CommandDesc {
                        name: arg.to_string(),
                        help: v.help.clone(),
                    });
                    cg = v;
                } else if let Some(v) = cg.commands.as_ref().and_then(|c| c.get(*arg).cloned()) {
                    commands.push(CommandDesc {
                        name: arg.to_string(),
                        help: v.help.clone(),
                    });
                    // Stop once we meet a command.
                    // It can happen that there are still remaining positional arguments here, we
                    // tolerate them here as there is no obvious way to handle it correctly during
                    // constructing clap::Command.
                    c = Some(v);
                    break;
                } else {
                    // Stop if we encountered an unknown argument, which is neither a command nor a
                    // command group.
                    break;
                }
            }

            let mut commands_rev = commands.iter().rev();
            let last_command = commands_rev.next().unwrap();
            let mut cmd = Command::new(last_command.name.clone()).about(
                last_command
                    .help
                    .as_ref()
                    .map_or("".to_string(), |v| v.short.clone()),
            );
            if let Some(c) = c {
                // Construct the last command name as a Command, which contains args
                match api_manager.index.locate_command_file(input) {
                    Ok(command_file) => match api_manager.read_command(&command_file) {
                        Ok(command) => {
                            cmd = cmd.args(build_args(&c.versions, &command));
                            command_metadata = Some(command);
                        }
                        Err(err) => {
                            tracing::error!("read command failed: {err}");
                        }
                    },
                    Err(err) => {
                        tracing::error!("locate command file failed: {err}");
                    }
                }
            } else {
                // Construct the last command name as a CommandGroup, which can contain commands and command groups
                cmd = cmd.subcommand_required(true).arg_required_else_help(true);
                if let Some(commands) = cg.commands {
                    let mut keys: Vec<_> = commands.keys().collect();
                    keys.sort();
                    cmd = cmd.subcommands(keys.iter().map(|name| {
                        let c = commands.get(*name).unwrap();
                        Command::new(*name)
                            .about(c.help.as_ref().map_or("".to_string(), |v| v.short.clone()))
                    }))
                }
                if let Some(cgs) = cg.command_groups {
                    let mut keys: Vec<_> = cgs.keys().collect();
                    keys.sort();
                    cmd = cmd.subcommands(keys.iter().map(|name| {
                        let cg = cgs.get(*name).unwrap();
                        Command::new(*name)
                            .about(cg.help.as_ref().map_or("".to_string(), |v| v.short.clone()))
                    }));
                }
            }
            for command in commands_rev {
                cmd = Command::new(command.name.clone())
                    .about(
                        command
                            .help
                            .as_ref()
                            .map_or("".to_string(), |v| v.short.clone()),
                    )
                    .subcommand(cmd)
            }
            cmd_base().subcommand(cmd_api_base_real().subcommand(cmd))
        }
        None => cmd_base().subcommand(cmd_api_base_real()),
    };
    (cmd, command_metadata)
}

fn build_args(versions: &Vec<String>, command: &metadata_command::Command) -> Vec<Arg> {
    let mut out = vec![];

    // Build the api-version arg
    out.push(
        Arg::new("api-version")
            .long("api-version")
            .help(format!(
                "API version (default: {})",
                versions.iter().max().unwrap_or(&"".to_string())
            ))
            .value_parser(PossibleValuesParser::new(versions)),
    );

    // Build the file & edit args
    out.push(
        Arg::new("file")
            .long("file")
            .short('f')
            .value_name("PATH")
            .value_parser(clap::value_parser!(std::path::PathBuf))
            .conflicts_with("edit")
            .help("Read request payload from the file"),
    );
    out.push(
        Arg::new("edit")
            .long("edit")
            .short('e')
            .action(clap::ArgAction::SetTrue)
            .conflicts_with("file")
            .help("Open default editor to compose request payload"),
    );

    // Build the remaining arguments based on the command metadata.
    // NOTE: Only the default argument group (which contains the path segments) will be considered for required or not.
    command
        .arg_groups
        .iter()
        .filter(|ag| ag.name == "")
        .for_each(|ag| out.extend(ag.args.iter().map(|arg| build_arg(arg, true))));

    command
        .arg_groups
        .iter()
        .filter(|ag| ag.name != "")
        .for_each(|ag| out.extend(ag.args.iter().map(|arg| build_arg(arg, false))));

    out
}

fn build_arg(arg: &metadata_command::Arg, handle_required: bool) -> Arg {
    // The options of one argument can have 0/N short, 0/N long.
    // We reagard the first short(prefered)/long as the name.
    let mut short: Option<char> = None;
    let mut short_aliases = vec![];
    let mut long: Option<String> = None;
    let mut long_aliases = vec![];
    arg.options.iter().for_each(|opt| {
        if opt.len() == 1 {
            let c = opt.chars().next().unwrap();
            if short.is_none() {
                short = Some(c);
            } else {
                short_aliases.push(c);
            }
        } else {
            if long.is_none() {
                long = Some(opt.clone());
            } else {
                long_aliases.push(opt.clone());
            }
        }
    });
    let mut out = Arg::new(arg.var.clone())
        .value_name("value")
        .visible_short_aliases(short_aliases)
        .visible_aliases(long_aliases);
    if let Some(short) = short {
        out = out.short(short);
    }
    if let Some(long) = long {
        out = out.long(long);
    }
    if let Some(help) = &arg.help {
        out = out.help(help.short.clone());
    }

    if handle_required {
        if let Some(required) = arg.required {
            out = out.required(required);
        }
    }

    out
}

#[test]
fn verify_cmd() {
    cmd().debug_assert();
}
