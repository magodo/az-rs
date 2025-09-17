use core::unreachable;
use std::collections::HashMap;

use crate::client::Client;

use super::metadata::{Command, Operation, Schema, VersionCommand};
use anyhow::{anyhow, bail, Result};
use clap::ArgMatches;

pub struct CommandInvocation {
    command: VersionCommand,
    matches: ArgMatches,
}

impl CommandInvocation {
    pub fn new(command: &Command, matches: &ArgMatches) -> Result<Self> {
        // TODO: Honor the api-version from the matches and choose the correct VersionCommand.

        // By default, we use the latest api version.
        let version = command
            .versions
            .iter()
            .max_by_key(|(v, _)| (*v).clone())
            .ok_or(anyhow!("no versioned command defined"))?
            .0;
        let command = command.versions.get(version).ok_or(anyhow!(
            r#"no versioned command found for version "{}""#,
            version
        ))?;
        Ok(Self {
            command: command.clone(),
            matches: matches.clone(),
        })
    }

    pub async fn invoke(&self, client: &Client) -> Result<String> {
        if self.command.operations.is_empty() {
            bail!("no operation found for this command");
        }
        let operation = self.command.operations.first().unwrap();
        let operation_ionvocation = OperationInvocation::new(operation, &self.matches);
        operation_ionvocation.invoke(client).await
    }
}

struct OperationInvocation {
    operation: Operation,
    matches: ArgMatches,
}

impl OperationInvocation {
    pub fn new(operation: &Operation, matches: &ArgMatches) -> Self {
        Self {
            operation: operation.clone(),
            matches: matches.clone(),
        }
    }

    pub async fn invoke(&self, client: &crate::client::Client) -> Result<String> {
        if self.operation.http.is_none() {
            bail!(
                r#"HTTP information not found for operation "{}""#,
                self.operation
                    .operation_id
                    .clone()
                    .unwrap_or("".to_string()),
            );
        }

        let http = self.operation.http.as_ref().unwrap();
        let mut path = http.path.clone();
        for param in &http.request.path.params {
            if let Some(value) = self.matches.get_one::<String>(&param.arg) {
                path = path.replace(&format!("{{{}}}", param.name), value);
            } else if let Some(true) = param.required {
                bail!("missing required path parameter: {}", param.name);
            } else {
                unreachable!(
                    r#"optional path parameter "{}" not supported yet!"#,
                    param.name
                )
            }
        }
        let mut query_pairs = HashMap::new();
        // TODO: handle query parameters (query.params)
        for param in &http.request.query.consts {
            query_pairs.insert(param.name.clone(), param.default.value.clone());
        }
        let body: Option<bytes::Bytes> = if let Some(body_meta) = &http.request.body {
            if let Some(schema) = &body_meta.json.schema {
                self.build_body(schema.clone())?
                    .map(|v| bytes::Bytes::from(v.to_string()))
            } else {
                None
            }
        } else {
            None
        };
        let response = client
            .run(
                http.request.method.into(),
                path.as_str(),
                &query_pairs["api-version"],
                body,
                None,
            )
            .await?;
        for response_meta in &http.responses {
            if let Some(status_codes) = &response_meta.status_code {
                if status_codes.contains(&(u16::from(response.status_code) as i64)) {
                    return Ok(String::from_utf8(response.body.to_vec())?);
                }
            }
        }
        bail!(
            "error response: {}\n\n{}",
            response.status_code,
            String::from_utf8_lossy(&response.body)
        );
    }

    fn build_body(&self, schema: Schema) -> Result<Option<serde_json::Value>> {
        if let Some(props) = &schema.props {
            let mut map = serde_json::Map::new();
            for prop in props {
                if let Some(prop_name) = &prop.name {
                    let value = self.build_value(prop.clone())?;
                    if let Some(value) = value {
                        map.insert(prop_name.clone(), value);
                    }
                } else {
                    bail!(r#"property lacks the "name" in the schema"#,);
                }
            }
            return Ok(Some(serde_json::Value::Object(map)));
        }
        bail!(r#"schema lacks the "props" in the schema"#);
    }

    fn build_value(&self, schema: Schema) -> Result<Option<serde_json::Value>> {
        match schema.type_.as_str() {
            "object" => {
                if let Some(arg) = &schema.arg {
                    if let Some(value) = self.matches.get_one::<String>(arg) {
                        Ok(Some(serde_json::from_str(value)?))
                    } else {
                        Ok(None)
                    }
                } else if let Some(props) = &schema.props {
                    let mut map = serde_json::Map::new();
                    for prop in props {
                        if let Some(prop_name) = &prop.name {
                            let value = self.build_value(prop.clone())?;
                            if let Some(value) = value {
                                map.insert(prop_name.clone(), value);
                            }
                        } else {
                            bail!(r#"property lacks the "name" in the schema"#,);
                        }
                    }
                    if map.is_empty() {
                        Ok(None)
                    } else {
                        Ok(Some(serde_json::Value::Object(map)))
                    }
                } else {
                    bail!(r#"object schema lacks both the "arg" and "props" in the schema"#);
                }
            }
            s if s.starts_with("array") || s == "string" => {
                if let Some(arg) = &schema.arg {
                    if let Some(value) = self.matches.get_one::<String>(arg) {
                        Ok(Some(serde_json::from_str(value)?))
                    } else {
                        Ok(None)
                    }
                } else {
                    bail!(
                        r#"schema "{}" lacks the "arg" in the schema"#,
                        schema.name.unwrap_or("".to_string())
                    );
                }
            }
            s if s.starts_with("integer") => {
                if let Some(arg) = &schema.arg {
                    if let Some(value) = self.matches.get_one::<i32>(arg) {
                        Ok(Some((*value).into()))
                    } else {
                        Ok(None)
                    }
                } else {
                    bail!(
                        r#"schema "{}" lacks the "arg" in the schema"#,
                        schema.name.unwrap_or("".to_string())
                    );
                }
            }
            "boolean" => {
                if let Some(arg) = &schema.arg {
                    if let Some(value) = self.matches.get_one::<bool>(arg) {
                        Ok(Some((*value).into()))
                    } else {
                        Ok(None)
                    }
                } else {
                    bail!(
                        r#"schema "{}" lacks the "arg" in the schema"#,
                        schema.name.unwrap_or("".to_string())
                    );
                }
            }
            // TODO: We shall handle float and other potential types in the metadata.
            //       Then fail the other cases.
            _ => {
                // We suppose any other type as a json value first, if failed, try to parse it as a string
                if let Some(arg) = &schema.arg {
                    if let Some(value) = self.matches.get_one::<String>(arg) {
                        match serde_json::from_str(value) {
                            Ok(v) => Ok(Some(v)),
                            Err(_) => Ok(Some(serde_json::Value::String(value.clone()))),
                        }
                    } else {
                        Ok(None)
                    }
                } else {
                    bail!(
                        r#"schema "{}" lacks the "arg" in the schema"#,
                        schema.name.unwrap_or("".to_string())
                    );
                }
            }
        }
    }
}
