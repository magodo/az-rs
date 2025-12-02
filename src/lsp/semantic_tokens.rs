use std::fmt::Display;

use tree_sitter::Node;

pub enum SemanticTokenType {
    // structural tokens
    AttributeIdent,
    BlockIdent,

    // expressions
    Bool,
    String,
    Number,
}

impl TryFrom<Node<'_>> for SemanticTokenType {
    type Error = ();

    fn try_from(node: Node<'_>) -> Result<Self, Self::Error> {
        match node.kind() {
            "identifier" => {
                if let Some(parent) = node.parent() {
                    match parent.kind() {
                        "block" => return Ok(SemanticTokenType::BlockIdent),
                        "attribute" => return Ok(SemanticTokenType::AttributeIdent),
                        _ => {}
                    }
                }
                return Err(());
            }
            "string_lit" => Ok(SemanticTokenType::String),
            "numeric_lit" => Ok(SemanticTokenType::Number),
            "bool_lit" => Ok(SemanticTokenType::Bool),
            _ => Err(()),
        }
    }
}

impl Display for SemanticTokenType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            SemanticTokenType::AttributeIdent => "hcl-attribute",
            SemanticTokenType::BlockIdent => "hcl-block",
            SemanticTokenType::Bool => "hcl-type-bool",
            SemanticTokenType::String => "hcl-type-string",
            SemanticTokenType::Number => "hcl-type-number",
        })
    }
}

pub fn semantic_token_types() -> Vec<SemanticTokenType> {
    vec![
        SemanticTokenType::AttributeIdent,
        SemanticTokenType::BlockIdent,
        SemanticTokenType::Bool,
        SemanticTokenType::String,
        SemanticTokenType::Number,
    ]
}
