use std::str::FromStr;

use super::metadata_command::ArgGroup;
use anyhow::{anyhow, bail, Context, Result};
use clap::builder::PossibleValue;

pub struct CLIExpander {
    shell: Shell,
    arg_groups: Vec<ArgGroup>,
    raw_input: Vec<String>,
    body: Option<serde_json::Value>,
}

impl CLIExpander {
    pub fn new(
        shell: &Shell,
        arg_groups: &Vec<ArgGroup>,
        raw_input: &Vec<String>,
        body: Option<serde_json::Value>,
    ) -> Self {
        Self {
            shell: shell.clone(),
            arg_groups: arg_groups.clone(),
            raw_input: raw_input.clone(),
            body,
        }
    }

    pub fn expand(&self) -> Result<String> {
        let mut cli_inputs = vec![];
        let mut it = self.raw_input.iter();
        while let Some(opt) = it.next() {
            if *opt == "--print-cli" {
                it.next();
                continue;
            }
            if opt.starts_with("--print-cli=") {
                continue;
            }

            if *opt == "--edit" || *opt == "-e" {
                continue;
            }

            if *opt == "--file" || *opt == "-f" {
                it.next();
                continue;
            }
            if opt.starts_with("--file=") || opt.starts_with("-f=") {
                continue;
            }

            cli_inputs.push(opt.clone());
        }

        if let Some(ref body) = self.body {
            // Expand the root level parameters (except "properties"), plus the top level properties of the "properties".
            self.arg_groups
                .iter()
                .skip_while(|ag| ag.name.is_empty())
                .for_each(|ag| {
                    ag.args.iter().for_each(|arg| {
                        if arg.var.starts_with("$parameters.") {
                            let paths: Vec<_> = arg
                                .var
                                .strip_prefix("$parameters.")
                                .unwrap()
                                .split('.')
                                .collect();

                            if let Ok(val) = self.find_value(&paths, body) {
                                if let Some(option_name) = arg.options.first() {
                                    let prefix = if option_name.len() == 1 { "-" } else { "--" };
                                    cli_inputs.push(format!("{prefix}{}", option_name.clone()));
                                    cli_inputs.push(self.shell.escape(&val.to_string()));
                                }
                            }
                        }
                    });
                });
        }
        Ok(cli_inputs.join(" "))
    }

    fn find_value<'a>(
        &'a self,
        paths: &Vec<&str>,
        val: &'a serde_json::Value,
    ) -> Result<&'a serde_json::Value> {
        let mut val = val;
        for path in paths {
            match val {
                serde_json::Value::Null
                | serde_json::Value::Bool(_)
                | serde_json::Value::Number(_)
                | serde_json::Value::String(_) => {
                    bail!("looking up the field {path} on {val:?}");
                }
                serde_json::Value::Array(values) => {
                    let idx: usize = path.parse().context(format!(
                        "looking up the field {path} on an array: {values:?}"
                    ))?;
                    val = values.get(idx).context(format!(
                        "getting the {idx}-th element on the array: {values:?}"
                    ))?;
                }
                serde_json::Value::Object(map) => {
                    val = map
                        .get(*path)
                        .context(format!("looking up the field {path} on an object {map:?}"))?;
                }
            }
        }
        return Ok(val);
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Shell {
    Cmd,
    PowerShell,
    Unix,
}

impl Shell {
    pub fn variants() -> impl Iterator<Item = PossibleValue> {
        [
            PossibleValue::new("cmd"),
            PossibleValue::new("powershell"),
            PossibleValue::new("unix"),
        ]
        .into_iter()
    }

    fn escape(&self, arg: &str) -> String {
        match self {
            Shell::Unix => {
                let mut out = String::from(r#"""#);
                for c in arg.chars() {
                    if c == '"' {
                        out.push('\\'); // escape `"` by `\"`
                    }
                    out.push(c);
                }
                out.push('"');
                out
            }
            Shell::PowerShell | Shell::Cmd => {
                let mut out = String::from(r#"""#);
                for c in arg.chars() {
                    if c == '"' {
                        out.push('"'); // escape `"` by `""`
                    }
                    out.push(c);
                }
                out.push('"');
                out
            }
        }
    }
}

impl FromStr for Shell {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "cmd" => Ok(Shell::Cmd),
            "powershell" => Ok(Shell::PowerShell),
            "unix" => Ok(Shell::Unix),
            _ => Err(anyhow!("invalid shell: {s}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unix_escape_simple() {
        assert_eq!(Shell::Unix.escape("foo"), r#""foo""#);
    }

    #[test]
    fn test_unix_escape_with_space() {
        assert_eq!(Shell::Unix.escape("foo bar"), r#""foo bar""#);
    }

    #[test]
    fn test_unix_escape_with_quote() {
        assert_eq!(Shell::Unix.escape("foo'bar"), r#""foo'bar""#);
    }

    #[test]
    fn test_unix_escape_with_double_quote() {
        assert_eq!(Shell::Unix.escape(r#"foo"bar"#), r#""foo\"bar""#);
    }

    #[test]
    fn test_cmd_escape_simple() {
        assert_eq!(Shell::Cmd.escape("foo"), r#""foo""#);
    }

    #[test]
    fn test_cmd_escape_with_space() {
        assert_eq!(Shell::Cmd.escape("foo bar"), r#""foo bar""#);
    }

    #[test]
    fn test_cmd_escape_with_quote() {
        assert_eq!(Shell::Cmd.escape("foo'bar"), r#""foo'bar""#);
    }

    #[test]
    fn test_cmd_escape_with_double_quote() {
        assert_eq!(Shell::Cmd.escape(r#"foo"bar"#), r#""foo""bar""#);
    }

    #[test]
    fn test_powershell_escape_simple() {
        assert_eq!(Shell::PowerShell.escape("foo"), r#""foo""#);
    }

    #[test]
    fn test_powershell_escape_with_space() {
        assert_eq!(Shell::PowerShell.escape("foo bar"), r#""foo bar""#);
    }

    #[test]
    fn test_powershell_escape_with_quote() {
        assert_eq!(Shell::PowerShell.escape("foo'bar"), r#""foo'bar""#);
    }

    #[test]
    fn test_powershell_escape_with_double_quote() {
        assert_eq!(Shell::PowerShell.escape(r#"foo"bar"#), r#""foo""bar""#);
    }
}
