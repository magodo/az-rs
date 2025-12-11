use crate::{api::metadata_command::Method, cmd};

use super::metadata_command::{Operation, Schema};
use anyhow::{bail, Result};
use clap::ArgMatches;
use core::unreachable;
use std::collections::HashMap;

pub struct OperationInvocation {
    operation: Operation,
    matches: ArgMatches,
    body: Option<serde_json::Value>,
}

impl OperationInvocation {
    pub fn new(
        operation: &Operation,
        matches: &ArgMatches,
        body: &Option<serde_json::Value>,
    ) -> Self {
        Self {
            operation: operation.clone(),
            matches: matches.clone(),
            body: body.clone(),
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
        let mut path;
        // In case the "--id" is specified, we validate and use it.
        if let Some(id) = self.matches.get_one::<String>(cmd::ID_OPTION) {
            let id = if id == "-" {
                cmd::ResourceId::from_stdin()?
            } else {
                cmd::ResourceId::from(id)
            };
            id.validate_pattern(&http.path, &http.request.method)?;

            path = id.id();
            if http.request.method == Method::Post {
                if let Some(last_seg) = http.path.split("/").last() {
                    path += format!("/{}", last_seg).as_str();
                }
            }
        } else {
            path = http.path.clone();
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
        }
        let mut query_pairs = HashMap::new();
        for param in &http.request.query.consts {
            // Only handle api-version const query so far.
            if param.name == "api-version" {
                if let Some(value) = self.matches.get_one::<String>("api-version") {
                    query_pairs.insert(param.name.clone(), value.clone());
                } else {
                    query_pairs.insert(param.name.clone(), param.default.value.clone());
                }
            }
        }
        if let Some(params) = http.request.query.params.as_ref() {
            for param in params {
                if let Some(value) = self.matches.get_one::<String>(&param.arg) {
                    query_pairs.insert(param.name.clone(), value.clone());
                }
            }
        }

        let body = if self.body.is_some() {
            self.body.clone()
        } else if let Some(body_meta) = &http.request.body {
            let bb = BodyBuilder(&self.matches);
            if let Some(schema) = &body_meta.json.schema {
                Some(bb.build_body(schema)?)
            } else {
                None
            }
        } else {
            None
        }
        .map(|v| bytes::Bytes::from(v.to_string()));

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
}

struct BodyBuilder<'a>(&'a ArgMatches);

impl<'a> BodyBuilder<'a> {
    pub fn build_body(&self, schema: &Schema) -> Result<serde_json::Value> {
        if let Some(props) = &schema.props {
            let mut map = serde_json::Map::new();
            for prop in props {
                if let Some(prop_name) = &prop.name {
                    let value = self.build_value(prop)?;
                    if let Some(value) = value {
                        map.insert(prop_name.clone(), value);
                    }
                } else {
                    bail!(r#"property {prop:#?} lacks the "name" in the schema"#,);
                }
            }
            return Ok(serde_json::Value::Object(map));
        }
        bail!(r#"schema lacks the top level "props" in the schema"#);
    }

    fn build_value(&self, schema: &Schema) -> Result<Option<serde_json::Value>> {
        match schema.type_.as_str() {
            "object" => {
                if let Some(arg) = &schema.arg {
                    if let Some(value) = self.0.get_one::<String>(arg) {
                        Ok(Some(serde_json::from_str(value)?))
                    } else {
                        Ok(None)
                    }
                } else if let Some(props) = &schema.props {
                    let mut map = serde_json::Map::new();
                    for prop in props {
                        if let Some(prop_name) = &prop.name {
                            let value = self.build_value(prop)?;
                            if let Some(value) = value {
                                map.insert(prop_name.clone(), value);
                            }
                        } else {
                            bail!(r#"property {prop:#?} lacks the "name" in the schema"#,);
                        }
                    }
                    if map.is_empty() {
                        Ok(None)
                    } else {
                        Ok(Some(serde_json::Value::Object(map)))
                    }
                } else {
                    bail!(r#"schema {schema:#?} lacks both the "arg" and "props""#);
                }
            }
            "string" => {
                if let Some(arg) = &schema.arg {
                    if let Some(value) = self.0.get_one::<String>(arg) {
                        Ok(Some((value.clone()).into()))
                    } else {
                        Ok(None)
                    }
                } else {
                    bail!(r#"schema "{schema:#?}" lacks the "arg" in the schema"#);
                }
            }
            _ => {
                // The other types are all passed in its json form, hence can be directly decoded
                if let Some(arg) = &schema.arg {
                    if let Some(value) = self.0.get_one::<String>(arg) {
                        Ok(serde_json::from_str(value)?)
                    } else {
                        Ok(None)
                    }
                } else {
                    bail!(r#"schema "{schema:#?}" lacks the "arg" in the schema"#);
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use clap::{Arg, Command};
    use serde_json::Value;

    #[test]
    fn build_value() {
        let m = Command::new("test")
            .arg(Arg::new("bool").long("bool"))
            .arg(Arg::new("int").long("int"))
            .arg(Arg::new("str").long("str"))
            .arg(Arg::new("array-of-str").long("array-of-str"))
            .arg(Arg::new("obj").long("obj"))
            .get_matches_from(vec![
                "test",
                "--bool",
                "true",
                "--int",
                "123",
                "--str",
                "abc",
                "--array-of-str",
                r#"["a", "b", "c"]"#,
                "--obj",
                r#"{"bool": true, "int": 123, "str": "abc", "array": ["a"]}"#,
            ]);

        let schema = Schema {
            type_: "object".to_string(),
            props: Some(vec![
                Schema {
                    type_: "boolean".to_string(),
                    arg: Some("bool".to_string()),
                    name: Some("bool".to_string()),
                    ..Schema::default()
                },
                Schema {
                    type_: "integer32".to_string(),
                    arg: Some("int".to_string()),
                    name: Some("int".to_string()),
                    ..Schema::default()
                },
                Schema {
                    type_: "string".to_string(),
                    arg: Some("str".to_string()),
                    name: Some("str".to_string()),
                    ..Schema::default()
                },
                Schema {
                    type_: "array".to_string(),
                    arg: Some("array-of-str".to_string()),
                    name: Some("array-of-str".to_string()),
                    item: Some(Box::new(Schema {
                        type_: "string".to_string(),
                        ..Schema::default()
                    })),
                    ..Schema::default()
                },
                Schema {
                    type_: "object".to_string(),
                    arg: Some("obj".to_string()),
                    name: Some("obj".to_string()),
                    props: Some(vec![
                        Schema {
                            type_: "boolean".to_string(),
                            arg: Some("bool".to_string()),
                            name: Some("bool".to_string()),
                            ..Schema::default()
                        },
                        Schema {
                            type_: "integer32".to_string(),
                            arg: Some("int".to_string()),
                            name: Some("int".to_string()),
                            ..Schema::default()
                        },
                        Schema {
                            type_: "string".to_string(),
                            arg: Some("str".to_string()),
                            name: Some("str".to_string()),
                            ..Schema::default()
                        },
                        Schema {
                            type_: "array".to_string(),
                            arg: Some("array-of-str".to_string()),
                            name: Some("array-of-str".to_string()),
                            item: Some(Box::new(Schema {
                                type_: "string".to_string(),
                                ..Schema::default()
                            })),
                            ..Schema::default()
                        },
                    ]),
                    ..Schema::default()
                },
            ]),
            ..Schema::default()
        };
        let bb = BodyBuilder(&m);
        let value = bb.build_body(&schema).unwrap();
        let expect: Value = serde_json::from_str(
            r#"
{
  "array-of-str": [
    "a",
    "b",
    "c"
  ],
  "bool": true,
  "int": 123,
  "obj": {
    "array": [
      "a"
    ],
    "bool": true,
    "int": 123,
    "str": "abc"
  },
  "str": "abc"
}
"#,
        )
        .unwrap();
        assert_eq!(value, expect);
    }
}
