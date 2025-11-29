use crate::lsp::hcl;
use anyhow::Result;
use hcl_edit::{parser, structure};
use lsp_document::{IndexedText, Pos, TextAdapter, TextMap};
use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, Diagnostic, DiagnosticSeverity, Documentation,
    NumberOrString, Position, TextDocumentContentChangeEvent,
};
use tree_sitter::{Node, Parser, Tree};

use crate::api::metadata_command::{Operation, Schema};

pub struct Document {
    text: IndexedText<String>,
    parser_ts: Parser,

    // This is strict syntax, used for diagnositics
    syntax_hcl: Result<structure::Body, parser::Error>,

    // This is lossy tolerant syntax, used for other features
    syntax_ts: Option<Tree>,

    // Following are the text and ts syntax of the last change.
    last_text: IndexedText<String>,
    last_syntax_ts: Option<Tree>,
}

impl Document {
    pub fn new(text: &str) -> Self {
        let text = IndexedText::new(text.to_string());
        let mut parser_ts = Parser::new();
        parser_ts
            .set_language(&tree_sitter_hcl::LANGUAGE.into())
            .expect("error loading HCL grammar");
        let syntax_ts = parser_ts.parse(text.text(), None);
        let syntax = parser::parse_body(text.text());
        Self {
            syntax_hcl: syntax,
            parser_ts,
            last_text: text.clone(),
            last_syntax_ts: None,
            text,
            syntax_ts,
        }
    }

    pub fn apply_change(&mut self, change: &TextDocumentContentChangeEvent) {
        if change.range.is_some() {
            panic!("Incremental change is not supported");
        }
        self.last_text = self.text.clone();
        self.last_syntax_ts = self.syntax_ts.clone();

        self.text = IndexedText::new(change.text.clone());
        self.syntax_ts = self.parser_ts.parse(self.text.text(), None);
        self.syntax_hcl = parser::parse_body(self.text.text());
    }

    pub fn hover(&self, operation: &Operation, position: &Position) -> Option<String> {
        let syntax_ts = self.syntax_ts.as_ref()?;
        let pos = self.text.lsp_pos_to_pos(position)?;
        let offset = self.text.pos_to_offset(&pos)?;
        let paths = hcl_identifier_path_by_offset(self.text.text().as_bytes(), offset, syntax_ts)?;
        tracing::debug!("grammar path: {:#?}", paths);
        if paths.is_empty() {
            return None;
        }
        let schema = api_schema_by_path(operation, &paths)?;
        tracing::debug!("Hover result: {:#?}", schema.name);
        return schema.name.clone();
    }

    pub fn complete(
        &self,
        operation: &Operation,
        position: &Position,
    ) -> Option<Vec<CompletionItem>> {
        let syntax_ts = self.syntax_ts.as_ref()?;
        let last_syntax_ts = self.last_syntax_ts.as_ref()?;
        let pos = self.text.lsp_pos_to_pos(position)?;
        let offset = self.text.pos_to_offset(&pos)?;
        let comp_info = hcl_completion_info_by_offset(
            self.text.text().as_bytes(),
            offset,
            syntax_ts,
            last_syntax_ts,
        )?;
        tracing::info!("comp_info: {comp_info:#?}");
        let schema = api_schema_by_path(operation, &comp_info.path)?;
        let props = &schema.props.as_ref()?;
        Some(
            props
                .iter()
                .filter(|prop| {
                    if let Some(name) = &prop.name {
                        !comp_info.exist_idents.contains(&name.as_str())
                    } else {
                        false
                    }
                })
                .map(|prop| CompletionItem {
                    label: prop.name.as_ref().unwrap().clone(),
                    kind: Some(CompletionItemKind::FIELD),
                    detail: Some("<detail>".to_string()),
                    documentation: Some(Documentation::String("<documentation>".to_string())),
                    ..Default::default()
                })
                .collect(),
        )
    }

    pub fn get_diagnostics(&self) -> Vec<Diagnostic> {
        if self.syntax_hcl.is_ok() {
            return Vec::new();
        }
        let Err(ref err) = self.syntax_hcl else {
            return Vec::new();
        };
        // Parse error location of hcl-rs (i.e. loc) starts from (1,1).
        // The LSP range below is zero indexed, hence needs to minus 1 from loc.
        let loc = err.location();
        let range = std::ops::Range {
            start: Pos {
                line: (loc.line() - 1) as u32,
                col: (loc.column() - 1) as u32,
            },
            end: Pos {
                line: (loc.line() - 1) as u32,
                col: (err.line().len()) as u32,
            },
        };
        let range = self.text.range_to_lsp_range(&range).unwrap();
        let diag = Diagnostic {
            range,
            severity: Some(DiagnosticSeverity::ERROR),
            code: Some(NumberOrString::String("parse".to_string())),
            source: Some("az-rs".to_string()),
            message: err.message().to_string(),
            ..Default::default()
        };
        //tracing::debug!("diag: {diag:#?}");
        return vec![diag];
    }
}

fn api_schema_by_path<'a>(operation: &'a Operation, paths: &[&str]) -> Option<&'a Schema> {
    let Some(mut schema) = operation
        .http
        .as_ref()
        .and_then(|http| http.request.body.as_ref())
        .and_then(|body| body.json.schema.as_ref())
    else {
        return None;
    };

    let mut found = true;
    for path in paths {
        if let Some(next_schema) = schema.props.as_ref().and_then(|props| {
            props.iter().find(|prop| {
                if let Some(name) = prop.name.as_ref() {
                    name == path
                } else {
                    false
                }
            })
        }) {
            schema = next_schema;
        } else {
            found = false;
            break;
        };
    }
    if !found {
        return None;
    }
    return Some(schema);
}

fn hcl_parent_nodes(node: Node<'_>) -> Option<Vec<Node<'_>>> {
    let mut nodes = vec![];
    let mut n = node;
    loop {
        nodes.push(n);
        if let Some(p) = n.parent() {
            n = p;
        } else {
            break;
        }
    }
    nodes.reverse();
    Some(nodes)
}

// hcl_node_by_offset returns the node from top config_file node down to the offset identifier node.
fn hcl_nodes_by_offset<'a, 'b>(offset: usize, syntax_ts: &'b Tree) -> Option<Vec<Node<'b>>> {
    let Some(node) = syntax_ts
        .root_node()
        .descendant_for_byte_range(offset, offset)
    else {
        return None;
    };

    if node.kind() != "identifier" {
        return None;
    }

    // Look up the path of identifiers from top to this node, regardless if it is a block,
    // attribute or key of an object element.
    let mut nodes = vec![];
    let mut n = node;
    loop {
        nodes.push(n);
        if let Some(p) = n.parent() {
            n = p;
        } else {
            break;
        }
    }
    nodes.reverse();
    Some(nodes)
}

// hcl_identifier_path_by_offset returns the path from top config_file node down to the identifier offset node.
fn hcl_identifier_path_by_offset<'a, 'b>(
    text: &'a [u8],
    offset: usize,
    syntax_ts: &'b Tree,
) -> Option<Vec<&'a str>> {
    let Some(nodes) = hcl_nodes_by_offset(offset, syntax_ts) else {
        return None;
    };
    hcl_identifier_path_of_nodes(text, &nodes).ok()
}

// hcl_identifier_path_of_nodes returns the path determined by the `nodes`.
// The path segments are identifiers of any block, attribute, ERROR, object key (i.e. object_elem) along the way.
fn hcl_identifier_path_of_nodes<'a>(text: &'a [u8], nodes: &[Node<'_>]) -> Result<Vec<&'a str>> {
    let mut paths = vec![];
    for node in nodes {
        if let Some(ident) = hcl_identifier_of_node(*node) {
            paths.push(ident.utf8_text(text)?);
        }
    }
    Ok(paths)
}

fn hcl_identifier_of_node(node: Node<'_>) -> Option<Node<'_>> {
    match node.kind() {
        "block" | "attribute" => node.child(0),
        "ERROR" => node.child(0).filter(|child| child.kind() == "identifier"),
        "object_elem" => node
            .child_by_field_name("key")
            .and_then(|expr| expr.child(0))
            .filter(|vexpr| vexpr.kind() == "variable_expr")
            .and_then(|vexpr| vexpr.child(0))
            .filter(|ident| ident.kind() == "identifier"),
        _ => None,
    }
}

#[derive(Clone, Debug)]
struct CompletionInfo<'a> {
    // The identifier path from the top to the parent identifier node, if any
    path: Vec<&'a str>,
    // The existing sibling identifiers
    // TODO: we shall allow repeatable indentities
    exist_idents: Vec<&'a str>,
}

// hcl_completion_info_by_offset returns the completion info.
// TODO: Complete dones't work correctly for object type. Hence suggest to use block instead.
fn hcl_completion_info_by_offset<'a, 'b>(
    text: &'a [u8],
    offset: usize,
    syntax_ts: &'b Tree,
    last_syntax_ts: &'b Tree,
) -> Option<CompletionInfo<'a>> {
    let mut anchor_node;

    // The offset represents the next token's position.
    // Here minus one to focus on the selected node.
    let offset = if offset != 0 { offset - 1 } else { offset };

    // Retrieve the node and the anchor node of the insertion position.
    let node = syntax_ts
        .root_node()
        .descendant_for_byte_range(offset, offset)?;
    anchor_node = hcl::AnchorNode::from_node(node)?;

    if anchor_node.0.is_error() {
        // Error anchor node implies an insert into the body or object, fallback to using
        // the last syntax tree, assuming it is error free. The offset in this case works as there
        // is only one char diff.
        let node = last_syntax_ts
            .root_node()
            .descendant_for_byte_range(offset, offset)?;
        anchor_node = hcl::AnchorNode::from_node(node)?;
        if anchor_node.0.is_error() {
            return None;
        }
    } else {
        // If the anchor node is not an ERROR node. It can implies one of:
        // 1. Triggering inside a block or config_file body.
        // 2. Modifying an existing identifier.
        if node.kind() != "identifier" {
            // Case 1, the existing anchor node is already correctly set, nothing to do.
        } else {
            // Case 2, we only support block and attribute identifier can be modified.
            // In this case, we need to move the anchor node one level up.
            match node.parent() {
                Some(parent) if ["block", "attribute"].contains(&parent.kind()) => {
                    anchor_node = hcl::AnchorNode::from_node(parent.parent()?)?;
                }
                _ => {
                    return None;
                }
            }
        }
    }

    let parent_nodes = hcl_parent_nodes(anchor_node.0)?;
    let path = hcl_identifier_path_of_nodes(text, &parent_nodes).ok()?;

    Some(CompletionInfo {
        path,
        exist_idents: vec![], // TODO
    })
}
