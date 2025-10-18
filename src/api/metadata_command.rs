use clap::ArgMatches;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Default, Serialize)]
pub struct Command {
    #[serde(rename = "argGroups")]
    pub arg_groups: Vec<ArgGroup>,
    pub conditions: Option<Vec<Condition>>,
    pub operations: Vec<Operation>,
    pub outputs: Option<Vec<Output>>,
    pub resources: Vec<Resource>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Condition {
    pub operator: ConditionOperator,
    pub var: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ConditionOperator {
    Operators {
        operators: Vec<ConditionOperator>,
        #[serde(rename = "type")]
        type_: ConditionOperatorType,
    },

    Operator {
        operator: Box<ConditionOperator>,
        #[serde(rename = "type")]
        type_: ConditionOperatorType,
    },

    Arg {
        arg: String,
        #[serde(rename = "type")]
        type_: ConditionOperatorType,
    },
}

#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
pub enum ConditionOperatorType {
    #[serde(rename = "hasValue")]
    HasValue,
    #[serde(rename = "not")]
    Not,
    #[serde(rename = "and")]
    And,
    #[serde(rename = "or")]
    Or,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Resource {
    pub id: String,
    pub plane: Plane,
}

#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
pub enum Plane {
    #[serde(rename = "mgmt-plane")]
    Mgmt,
    #[serde(rename = "data-plane")]
    Data,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ArgGroup {
    pub name: String,
    pub args: Vec<Arg>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Arg {
    #[serde(rename = "type")]
    pub type_: String,
    pub var: String,
    pub options: Vec<String>,
    pub group: Option<String>,
    pub help: Option<Help>,
    pub required: Option<bool>,
    #[serde(rename = "idPart")]
    pub id_part: Option<String>,
    #[serde(rename = "additionalProps")]
    pub additional_props: Option<AdditionalPropSchema>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Help {
    pub short: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Operation {
    #[serde(rename = "operationId")]
    pub operation_id: Option<String>,
    pub http: Option<Http>,
    pub when: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Http {
    pub path: String,
    pub request: Request,
    pub responses: Vec<Response>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Request {
    pub method: Method,
    pub path: RequestPath,
    pub query: RequestQuery,
    pub body: Option<Body>,
}

#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Method {
    Head,
    Get,
    Put,
    Patch,
    Post,
    Delete,
}

impl From<Method> for azure_core::http::Method {
    fn from(method: Method) -> Self {
        match method {
            Method::Head => azure_core::http::Method::Head,
            Method::Get => azure_core::http::Method::Get,
            Method::Put => azure_core::http::Method::Put,
            Method::Patch => azure_core::http::Method::Patch,
            Method::Post => azure_core::http::Method::Post,
            Method::Delete => azure_core::http::Method::Delete,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RequestPath {
    pub params: Vec<RequestPathParam>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RequestPathParam {
    #[serde(rename = "type")]
    pub type_: String,
    pub name: String,
    pub arg: String,
    pub required: Option<bool>,
    pub format: Option<RequestFormat>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RequestFormat {
    pub pattern: Option<String>,
    #[serde(rename = "maxLength")]
    pub max_length: Option<i64>,
    #[serde(rename = "minLength")]
    pub min_length: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ResponseFormat {
    pub template: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RequestQuery {
    pub consts: Vec<RequestQueryConst>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RequestQueryConst {
    pub name: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub required: Option<bool>,
    #[serde(rename = "readOnly")]
    pub read_only: Option<bool>,
    #[serde(rename = "const")]
    pub const_: bool,
    pub default: DefaultValue,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DefaultValue {
    pub value: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Body {
    pub json: BodyJSON,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BodyJSON {
    pub schema: Option<Schema>,
    // Only applies for response body
    pub var: Option<String>,
    #[serde(rename = "ref")]
    pub ref_: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Response {
    #[serde(rename = "statusCode")]
    pub status_code: Option<Vec<i64>>,
    pub body: Option<Body>,
    #[serde(rename = "isError")]
    pub is_error: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Output {
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(rename = "ref")]
    pub ref_: String,
    #[serde(rename = "clientFlatten")]
    pub client_flatten: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Schema {
    #[serde(rename = "type")]
    pub type_: String,
    pub name: Option<String>,
    pub required: Option<bool>,
    pub arg: Option<String>,
    #[serde(rename = "readOnly")]
    pub read_only: Option<bool>,
    pub props: Option<Vec<Schema>>,
    pub item: Option<Box<Schema>>,
    pub format: Option<ResponseFormat>,
    #[serde(rename = "clientFlatten")]
    pub client_flatten: Option<bool>,
    #[serde(rename = "additionalProps")]
    pub additional_props: Option<AdditionalPropSchema>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AdditionalPropSchema {
    pub item: AdditionalPropItemSchema,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AdditionalPropItemSchema {
    #[serde(rename = "type")]
    pub type_: String,
}

impl Command {
    pub fn select_operation(&self, matches: &ArgMatches) -> Option<&Operation> {
        if self.conditions.is_none() {
            return self.operations.first();
        }

        let matched_condition = self
            .conditions
            .as_ref()
            .unwrap()
            .iter()
            .find(|&c| self.match_operator(&c.operator, matches))
            .map(|c| c.var.clone())?;

        self.operations.iter().find(|op| {
            op.when
                .clone()
                .unwrap_or(vec![])
                .iter()
                .any(|w| w == &matched_condition)
        })
    }

    fn match_operator(&self, operator: &ConditionOperator, matches: &ArgMatches) -> bool {
        match operator {
            ConditionOperator::Operators { operators, type_ } => match type_ {
                ConditionOperatorType::Not | ConditionOperatorType::HasValue => unreachable!(
                    r#"operators' condition type can only be "and" or "or", got=%{type_:?}"#
                ),
                ConditionOperatorType::And => {
                    operators.iter().all(|o| self.match_operator(o, matches))
                }
                ConditionOperatorType::Or => {
                    operators.iter().any(|o| self.match_operator(o, matches))
                }
            },
            ConditionOperator::Operator { operator, type_ } => match type_ {
                ConditionOperatorType::Not => !self.match_operator(operator, matches),
                ConditionOperatorType::HasValue
                | ConditionOperatorType::And
                | ConditionOperatorType::Or => {
                    unreachable!(r#"operators' condition type can only be "not", got=%{type_:?}"#)
                }
            },
            ConditionOperator::Arg { arg, type_ } => match type_ {
                ConditionOperatorType::HasValue => matches.get_raw(arg).is_some(),
                ConditionOperatorType::Not
                | ConditionOperatorType::And
                | ConditionOperatorType::Or => unreachable!(
                    r#"operators' condition type can only be "hasValue", got=%{type_:?}"#
                ),
            },
        }
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
  "argGroups": [
    {
      "name": "",
      "args": [
        {
          "type": "ResourceGroupName",
          "var": "$Path.resourceGroupName",
          "options": [
            "g",
            "resource-group"
          ],
          "required": true,
          "idPart": "resource_group"
        },
        {
          "type": "SubscriptionId",
          "var": "$Path.subscriptionId",
          "options": [
            "subscription"
          ],
          "required": true,
          "idPart": "subscription"
        }
      ]
    },
    {
      "name": "Parameters",
      "args": [
        {
          "type": "ResourceLocation",
          "var": "$parameters.location",
          "options": [
            "l",
            "location"
          ],
          "required": true,
          "group": "Parameters",
          "help": {
            "short": "The location of the resource group. It cannot be changed after the resource group has been created. It must be one of the supported Azure locations."
          }
        },
        {
          "type": "string",
          "var": "$parameters.managedBy",
          "options": [
            "managed-by"
          ],
          "group": "Parameters",
          "help": {
            "short": "The ID of the resource that manages this resource group."
          }
        },
        {
          "type": "object",
          "var": "$parameters.tags",
          "options": [
            "tags"
          ],
          "group": "Parameters",
          "help": {
            "short": "The tags attached to the resource group."
          },
          "additionalProps": {
            "item": {
              "type": "string"
            }
          }
        }
      ]
    }
  ],
  "conditions": [
    {
      "operator": {
        "operators": [
          {
            "arg": "$Path.subscriptionId",
            "type": "hasValue"
          },
          {
            "operator": {
              "arg": "$Path.resourceGroupName",
              "type": "hasValue"
            },
            "type": "not"
          }
        ],
        "type": "and"
      },
      "var": "$Condition_VirtualNetworks_ListAll"
    },
    {
      "operator": {
        "operators": [
          {
            "arg": "$Path.resourceGroupName",
            "type": "hasValue"
          },
          {
            "arg": "$Path.subscriptionId",
            "type": "hasValue"
          }
        ],
        "type": "and"
      },
      "var": "$Condition_VirtualNetworks_List"
    }
  ],
  "operations": [
    {
      "operationId": "ResourceGroups_CreateOrUpdate",
      "http": {
        "path": "/subscriptions/{subscriptionId}/resourcegroups/{resourceGroupName}",
        "request": {
          "method": "put",
          "path": {
            "params": [
              {
                "type": "string",
                "name": "resourceGroupName",
                "arg": "$Path.resourceGroupName",
                "required": true,
                "format": {
                  "pattern": "^[-\\w\\._\\(\\)]+$",
                  "maxLength": 90,
                  "minLength": 1
                }
              },
              {
                "type": "string",
                "name": "subscriptionId",
                "arg": "$Path.subscriptionId",
                "required": true
              }
            ]
          },
          "query": {
            "consts": [
              {
                "readOnly": true,
                "const": true,
                "default": {
                  "value": "2024-11-01"
                },
                "type": "string",
                "name": "api-version",
                "required": true
              }
            ]
          },
          "body": {
            "json": {
              "schema": {
                "type": "object",
                "name": "parameters",
                "required": true,
                "props": [
                  {
                    "type": "ResourceLocation",
                    "name": "location",
                    "arg": "$parameters.location",
                    "required": true
                  },
                  {
                    "type": "string",
                    "name": "managedBy",
                    "arg": "$parameters.managedBy"
                  },
                  {
                    "type": "object",
                    "name": "tags",
                    "arg": "$parameters.tags",
                    "additionalProps": {
                      "item": {
                        "type": "string"
                      }
                    }
                  }
                ],
                "clientFlatten": true
              }
            }
          }
        },
        "responses": [
          {
            "statusCode": [
              200,
              201
            ],
            "body": {
              "json": {
                "var": "$Instance",
                "schema": {
                  "type": "object",
                  "props": [
                    {
                      "readOnly": true,
                      "type": "ResourceId",
                      "name": "id",
                      "format": {
                        "template": "/subscriptions/{}/resourcegroups/{}"
                      }
                    },
                    {
                      "type": "ResourceLocation",
                      "name": "location",
                      "required": true
                    },
                    {
                      "type": "string",
                      "name": "managedBy"
                    },
                    {
                      "readOnly": true,
                      "type": "string",
                      "name": "name"
                    },
                    {
                      "type": "object",
                      "name": "properties",
                      "props": [
                        {
                          "readOnly": true,
                          "type": "string",
                          "name": "provisioningState"
                        }
                      ]
                    },
                    {
                      "type": "object",
                      "name": "tags",
                      "additionalProps": {
                        "item": {
                          "type": "string"
                        }
                      }
                    },
                    {
                      "readOnly": true,
                      "type": "string",
                      "name": "type"
                    }
                  ]
                }
              }
            }
          },
          {
            "isError": true,
            "body": {
              "json": {
                "schema": {
                  "type": "@MgmtErrorFormat"
                }
              }
            }
          }
        ]
      }
    }
  ],
  "outputs": [
    {
      "type": "object",
      "ref": "$Instance",
      "clientFlatten": true
    }
  ],
  "resources": [
    {
      "id": "/subscriptions/{}/resourcegroups/{}",
      "plane": "mgmt-plane"
    }
  ]
}
"#;
        let command: Command = serde_json::from_str(input)?;
        let input_json: Value = serde_json::from_str(input)?;
        let output_json: Value = to_clean_json(&command);
        assert_eq!(input_json, output_json);
        Ok(())
    }
}
