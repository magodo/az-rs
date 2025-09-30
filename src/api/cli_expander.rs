use super::metadata_command::Operation;
use anyhow::Result;

pub struct CLIExpander {
    operation: Operation,
    raw_input: Vec<String>,
    body: Option<serde_json::Value>,
}

impl CLIExpander {
    pub fn new(
        operation: &Operation,
        raw_input: &Vec<String>,
        body: Option<serde_json::Value>,
    ) -> Self {
        Self {
            operation: operation.clone(),
            raw_input: raw_input.clone(),
            body,
        }
    }

    pub fn expand(&self) -> Result<String> {
        todo!();
    }
}
