use crate::lsp::{complete, hover};
use anyhow::Result;
use hcl_edit::{parser, structure};
use lsp_document::{IndexedText, Pos, TextAdapter, TextMap};
use tower_lsp::lsp_types::{
    CompletionItem, Diagnostic, DiagnosticSeverity, Hover, HoverContents, MarkupContent,
    MarkupKind, NumberOrString, Position, TextDocumentContentChangeEvent,
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

    pub fn hover(&self, operation: &Operation, position: &Position) -> Option<Hover> {
        let syntax_ts = self.syntax_ts.as_ref()?;
        let pos = self.text.lsp_pos_to_pos(position)?;
        let offset = self.text.pos_to_offset(&pos)?;
        let hover_info = hover::get_hover_info(&self.text, offset, syntax_ts, operation)?;
        return Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: hover_info.content,
            }),
            range: hover_info.range,
        });
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
        complete::get_completion_items(
            self.text.text().as_bytes(),
            offset,
            syntax_ts,
            last_syntax_ts,
            operation,
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
