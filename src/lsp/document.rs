use anyhow::Result;
use hcl_edit::{parser, structure};
use lsp_document::{IndexedText, Pos, TextAdapter, TextMap};
use tower_lsp::lsp_types::{
    Diagnostic, DiagnosticSeverity, NumberOrString, Position, TextDocumentContentChangeEvent,
};
use tree_sitter::{Parser, Tree};

use crate::api::metadata_command::Operation;

pub struct Document {
    text: IndexedText<String>,
    parser_ts: Parser,

    // This is strict syntax, used for diagnositics
    syntax_hcl: Result<structure::Body, parser::Error>,

    // This is lossy tolerant syntax, used for other features
    syntax_ts: Option<Tree>,
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
            syntax_ts,
            parser_ts,
            text,
        }
    }

    pub fn apply_change(&mut self, change: &TextDocumentContentChangeEvent) {
        if change.range.is_some() {
            panic!("Incremental change is not supported");
        }
        self.text = IndexedText::new(change.text.clone());
        self.syntax_hcl = parser::parse_body(self.text.text());
        self.syntax_ts = self.parser_ts.parse(self.text.text(), None);
    }

    pub fn hover(&self, operation: &Operation, position: &Position) -> Option<String> {
        let Some(syntax_ts) = &self.syntax_ts else {
            return None;
        };
        let Some(pos) = self.text.lsp_pos_to_pos(position) else {
            return None;
        };
        let Some(offset) = self.text.pos_to_offset(&pos) else {
            return None;
        };

        let Some(paths) = hcl_path_by_offset(self.text.text().as_bytes(), offset, syntax_ts) else {
            return None;
        };

        tracing::debug!("grammar path: {:#?}", paths);

        if paths.is_empty() {
            return None;
        }

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
        tracing::debug!("Hover result: {:#?}", schema.name);

        return schema.name.clone();
    }

    pub fn get_diagnostics(&self) -> Vec<Diagnostic> {
        if self.syntax_hcl.is_ok() {
            return Vec::new();
        }
        let Err(ref err) = self.syntax_hcl else {
            return Vec::new();
        };
        tracing::debug!("parse error: {:#?}", err);
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
        tracing::debug!("diag: {diag:#?}");
        return vec![diag];
    }
}

fn hcl_path_by_offset<'a, 'b>(
    text: &'a [u8],
    offset: usize,
    syntax_ts: &'b Tree,
) -> Option<Vec<&'a str>> {
    let Some(node) = syntax_ts
        .root_node()
        .descendant_for_byte_range(offset, offset)
    else {
        return None;
    };

    // If the focused node is not an identifier, return early.
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
    let mut paths = vec![];
    for node in nodes {
        if let Some(ident) = match node.kind() {
            "block" | "attribute" | "ERROR" => node.child(0),
            "object_elem" => node
                .child_by_field_name("key")
                .and_then(|expr| expr.child(0))
                .filter(|vexpr| vexpr.kind() == "variable_expr")
                .and_then(|vexpr| vexpr.child(0))
                .filter(|ident| ident.kind() == "identifier"),
            _ => None,
        } {
            paths.push(ident.utf8_text(text).ok()?);
        }
    }
    Some(paths)
}
