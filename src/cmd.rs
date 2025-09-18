use crate::api::{metadata, ApiManager};
use crate::arg::CliInput;
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

pub fn cmd_api(api_manager: &ApiManager, input: &CliInput) -> Command {
    let pos_args = input.pos_args();

    // No positional argument specified, list the rps
    if pos_args.is_empty() {
        return cmd_base().subcommand(
            cmd_api_base_real().subcommands(api_manager.list_rps().iter().map(Command::new)),
        );
    }
    let rp = pos_args.first().unwrap();
    match api_manager.read_metadata(rp) {
        Ok(metadata) => {
            let mut args = pos_args.iter();
            let mut command_names = vec![];

            // Construct a fake command group here to initiate the following while loop
            let mut cg = metadata::CommandGroup {
                name: rp.to_string(),
                command_groups: Some(metadata.command_groups),
                ..metadata::CommandGroup::default()
            };

            let mut c: Option<metadata::Command> = None;

            while let Some(arg) = args.next() {
                command_names.push(arg.to_string());

                if let Some(v) = cg
                    .command_groups
                    .clone()
                    .and_then(|cgs| cgs.iter().find(|cg| cg.name.as_str() == *arg).cloned())
                {
                    cg = v;
                } else if let Some(v) = cg
                    .commands
                    .iter()
                    .find(|c| c.name.as_str() == *arg)
                    .cloned()
                {
                    // Stop once we meet a command.
                    // It can happen that there are still remaining positional arguments here, we
                    // tolerate them here as there is no obvious way to handle it correctly during
                    // constructing clap::Command.
                    c = Some(v);
                    break;
                }
            }

            let mut command_names_rev = command_names.iter().rev();
            let mut cmd = Command::new(command_names_rev.next().unwrap());
            if let Some(c) = c {
                // Construct the last command name as a Command, which contains args
                cmd = cmd.args(build_args(&c.arg_groups));
            } else {
                // Construct the last command name as a CommandGroup, which contains commands and potential
                // command groups
                cmd = cmd
                    .subcommands(cg.commands.iter().map(|c| Command::new(c.name.clone())))
                    .subcommand_required(true)
                    .arg_required_else_help(true);
                if let Some(cgs) = cg.command_groups {
                    cmd = cmd.subcommands(cgs.iter().map(|c| Command::new(c.name.clone())));
                }
            }
            for name in command_names_rev {
                cmd = Command::new(name.clone())
                    .subcommand(cmd)
                    .subcommand_required(true)
                    .arg_required_else_help(true);
            }
            cmd_base().subcommand(cmd_api_base_real().subcommand(cmd))
        }
        Err(err) => {
            dbg!("subcommand construction failed: {}", err);
            cmd_base().subcommand(cmd_api_base_real().subcommand(Command::new(rp.to_string())))
        }
    }
}

fn build_args(arg_groups: &Vec<metadata::ArgGroup>) -> Vec<Arg> {
    let mut out = vec![];
    arg_groups
        .iter()
        .for_each(|ag| out.extend(ag.args.iter().map(build_arg)));
    out
}

fn build_arg(arg: &metadata::Arg) -> Arg {
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
    if let Some(required) = arg.required {
        out = out.required(required);
    }
    if let Some(help) = &arg.help {
        out = out.help(help.short.clone());
    }
    out
}

#[test]
fn verify_cmd() {
    cmd().debug_assert();
}
