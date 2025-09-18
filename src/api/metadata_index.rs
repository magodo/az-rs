use std::collections::HashMap;

use anyhow::{anyhow, bail, Result};
use serde::Deserialize;

use crate::arg::CliInput;

#[cfg_attr(test, derive(serde::Serialize))]
#[derive(Debug, Clone, Deserialize)]
pub struct Index {
    pub help: Option<Help>,
    #[serde(rename = "commandGroups")]
    pub command_groups: HashMap<String, CommandGroup>,
}

#[cfg_attr(test, derive(serde::Serialize))]
#[derive(Debug, Clone, Deserialize, Default)]
pub struct CommandGroup {
    #[serde(rename = "commandGroups")]
    pub command_groups: Option<HashMap<String, CommandGroup>>,
    pub commands: Option<HashMap<String, Command>>,
    pub help: Option<Help>,
}

#[cfg_attr(test, derive(serde::Serialize))]
#[derive(Debug, Clone, Deserialize, Default)]
pub struct Command {
    pub help: Option<Help>,
    pub versions: Vec<String>,
}

#[cfg_attr(test, derive(serde::Serialize))]
#[derive(Debug, Clone, Deserialize)]
pub struct Help {
    pub short: String,
}

impl Index {
    // Locate the command file based on the raw CLI input.
    // Especially, if the api-version is not specified, it defaults to the latest version.
    pub fn locate_command_file(
        &self,
        api_version: Option<String>,
        input: &CliInput,
    ) -> Result<String> {
        if input.is_empty() {
            bail!("empty CLI input");
        }

        // The command metadata files are flattened to the same level at the FS.
        // The name follows: <rp>_<res>+_<version>.json
        let mut parts = vec![];

        let args = input.pos_args();
        let mut args = args.iter();

        // rp
        parts.push(args.next().unwrap().to_string());

        let mut cg = CommandGroup {
            command_groups: Some(self.command_groups.clone()),
            help: None,
            commands: None,
        };

        while let Some(arg) = args.next() {
            parts.push(arg.to_string());
            if let Some(v) = cg
                .command_groups
                .as_ref()
                .and_then(|cg| cg.get(*arg).cloned())
            {
                cg = v;
            } else if let Some(v) = cg.commands.as_ref().and_then(|c| c.get(*arg).cloned()) {
                // Command must be the last positional argument
                if let Some(arg) = args.next() {
                    return Err(anyhow!("unknown argument {}", arg));
                } else {
                    let ver = if let Some(ref version) = api_version {
                        v.versions
                            .iter()
                            .find(|&v| v == version)
                            .ok_or(anyhow!(r#"api version {} not available"#, version))?
                    } else {
                        v.versions
                            .iter()
                            .max()
                            .ok_or(anyhow!("no api version defined"))?
                    };
                    parts.push(ver.to_string());

                    return Ok(parts.join("_") + ".json");
                }
            } else {
                return Err(anyhow!("unknown argument {}", arg));
            }
        }

        return Err(anyhow!("this isn't a command"));
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions::assert_eq;
    use serde::Serialize;
    use serde_json::Value;
    use std::error::Error;

    // This is used to skip optional fields when they are None.
    fn to_clean_json<T: Serialize>(value: &T) -> Value {
        fn strip_nulls(value: Value) -> Value {
            match value {
                Value::Object(map) => {
                    let cleaned = map
                        .into_iter()
                        .filter_map(|(k, v)| {
                            let v = strip_nulls(v);
                            if v.is_null() {
                                None
                            } else {
                                Some((k, v))
                            }
                        })
                        .collect();
                    Value::Object(cleaned)
                }
                Value::Array(arr) => {
                    let cleaned = arr
                        .into_iter()
                        .map(strip_nulls)
                        .filter(|v| !v.is_null())
                        .collect();
                    Value::Array(cleaned)
                }
                other => other,
            }
        }

        let raw = serde_json::to_value(value).expect("Serialization failed");
        strip_nulls(raw)
    }

    #[test]
    fn deserialize() -> Result<(), Box<dyn Error>> {
        let input = r#"
{
  "help": {
      "short": "rp"
  },
  "commandGroups": {
    "group": {
      "commands": {
        "show": {
          "help": {
            "short": "show"
          },
          "versions": [
            "2024-11-01"
          ]
        },
        "create": {
          "help": {
            "short": "create"
          },
          "versions": [
            "2024-11-01"
          ]
        }
      },
      "help": {
          "short": "command group"
      }
    }
  }
}
"#;
        let index: Index = serde_json::from_str(input)?;
        let input_json: Value = serde_json::from_str(input)?;
        let output_json: Value = to_clean_json(&index);
        assert_eq!(input_json, output_json);
        Ok(())
    }
}
