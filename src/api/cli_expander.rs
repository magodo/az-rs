use std::str::FromStr;

use crate::{
    arg::{self, CliInput},
    cmd::ID_OPTION,
};

use super::metadata_command::ArgGroup;
use anyhow::{anyhow, bail, Context, Result};
use clap::builder::PossibleValue;

pub struct CLIExpander {
    shell: Shell,
    arg_groups: Vec<ArgGroup>,
    arg_input: CliInput,
    body: Option<serde_json::Value>,
    id: Option<String>,
}

impl CLIExpander {
    pub fn new(
        shell: &Shell,
        arg_groups: &Vec<ArgGroup>,
        cli_input: &CliInput,
        body: Option<serde_json::Value>,
        id: Option<String>,
    ) -> Self {
        Self {
            shell: shell.clone(),
            arg_groups: arg_groups.clone(),
            arg_input: cli_input.clone(),
            body,
            id,
        }
    }

    pub fn expand(&self) -> Result<Vec<String>> {
        let mut cli_inputs = vec![];
        for (k, v) in &self.arg_input.opt_args() {
            if ["print-cli", "stdin", "edit", "e", "file", "f"].contains(k) {
                continue;
            }
            cli_inputs.push(arg::Arg::Optional(k.to_string(), v.map(String::from)));
        }
        if let Some(ref id) = self.id {
            cli_inputs.push(arg::Arg::Optional(ID_OPTION.to_string(), Some(id.clone())));
        }

        if let Some(ref body) = self.body {
            // Expand the root level parameters (except "properties"), plus the top level properties of the "properties".
            self.arg_groups
                .iter()
                .skip_while(|ag| ag.name.is_empty())
                .for_each(|ag| {
                    ag.args.iter().for_each(|arg| {
                        if let Some(prefix) = arg.var.split('.').next() {
                            if prefix.starts_with("$")
                                && prefix.to_lowercase().contains("parameters")
                            {
                                let paths: Vec<_> = arg.var.split('.').skip(1).collect();
                                if let Ok(val) = self.find_value(&paths, body) {
                                    if let Some(option_name) = arg.options.first() {
                                        cli_inputs.push(arg::Arg::Optional(
                                            option_name.clone(),
                                            Some(self.shell.escape(val)),
                                        ));
                                    }
                                }
                            }
                        }
                    });
                });
        }
        Ok(cli_inputs
            .iter()
            .map(|arg| format!("{}", arg))
            .collect::<Vec<_>>())
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

    fn escape(&self, arg: &serde_json::Value) -> String {
        let chars: Vec<_> = if arg.is_string() {
            arg.as_str().unwrap().chars().collect()
        } else {
            arg.to_string().chars().collect()
        };
        match self {
            Shell::Unix => {
                let mut out = String::new();
                out.push('"');
                for c in chars {
                    if c == '"' {
                        out.push('\\'); // escape `"` by `\"`
                    }
                    out.push(c);
                }
                out.push('"');
                out
            }
            Shell::PowerShell | Shell::Cmd => {
                let mut out = String::new();
                out.push('"');
                for c in chars {
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
        assert_eq!(Shell::Unix.escape(&serde_json::json!("foo")), "\"foo\"");
    }

    #[test]
    fn test_unix_escape_with_space() {
        assert_eq!(
            Shell::Unix.escape(&serde_json::json!("foo bar")),
            r#""foo bar""#
        );
    }

    #[test]
    fn test_unix_escape_with_quote() {
        assert_eq!(
            Shell::Unix.escape(&serde_json::json!("foo'bar")),
            r#""foo'bar""#
        );
    }

    #[test]
    fn test_unix_escape_with_double_quote() {
        assert_eq!(
            Shell::Unix.escape(&serde_json::json!(r#"foo"bar"#)),
            r#""foo\"bar""#
        );
    }

    #[test]
    fn test_unix_escape_with_double_quote_in_object() {
        assert_eq!(
            Shell::Unix.escape(&serde_json::json!({"a\"b": 123})),
            r#""{\"a\\"b\":123}""#
        );
    }

    #[test]
    fn test_cmd_escape_simple() {
        assert_eq!(Shell::Cmd.escape(&serde_json::json!("foo")), r#""foo""#);
    }

    #[test]
    fn test_cmd_escape_with_space() {
        assert_eq!(
            Shell::Cmd.escape(&serde_json::json!("foo bar")),
            r#""foo bar""#
        );
    }

    #[test]
    fn test_cmd_escape_with_quote() {
        assert_eq!(
            Shell::Cmd.escape(&serde_json::json!("foo'bar")),
            r#""foo'bar""#
        );
    }

    #[test]
    fn test_cmd_escape_with_double_quote() {
        assert_eq!(
            Shell::Cmd.escape(&serde_json::json!(r#"foo"bar"#)),
            r#""foo""bar""#
        );
    }

    #[test]
    fn test_powershell_escape_simple() {
        assert_eq!(
            Shell::PowerShell.escape(&serde_json::json!("foo")),
            r#""foo""#
        );
    }

    #[test]
    fn test_powershell_escape_with_space() {
        assert_eq!(
            Shell::PowerShell.escape(&serde_json::json!("foo bar")),
            r#""foo bar""#
        );
    }

    #[test]
    fn test_powershell_escape_with_quote() {
        assert_eq!(
            Shell::PowerShell.escape(&serde_json::json!("foo'bar")),
            r#""foo'bar""#
        );
    }

    #[test]
    fn test_powershell_escape_with_double_quote() {
        assert_eq!(
            Shell::PowerShell.escape(&serde_json::json!(r#"foo"bar"#)),
            r#""foo""bar""#
        );
    }
}
