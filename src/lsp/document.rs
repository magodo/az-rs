use hcl_edit::{parser, structure};
use lsp_document::{IndexedText, Pos, TextAdapter, TextMap};
use tower_lsp::lsp_types::{
    Diagnostic, DiagnosticSeverity, NumberOrString, TextDocumentContentChangeEvent,
};

pub struct Document {
    hcl_body: Result<structure::Body, parser::Error>,
    text: IndexedText<String>,
}

impl Document {
    pub fn new(text: &str) -> Self {
        let text = IndexedText::new(text.to_string());
        let hcl_body = parser::parse_body(text.text());
        Self { hcl_body, text }
    }

    pub fn apply_change(&mut self, change: &TextDocumentContentChangeEvent) {
        if change.range.is_some() {
            panic!("Incremental change is not supported");
        }
        self.text = IndexedText::new(change.text.clone());
        self.hcl_body = parser::parse_body(self.text.text());
        dbg!(&self.hcl_body);
    }

    pub fn get_diagnostics(&self) -> Vec<Diagnostic> {
        if self.hcl_body.is_ok() {
            return Vec::new();
        }
        let Err(ref err) = self.hcl_body else {
            return Vec::new();
        };
        tracing::debug!("parse error: {:#?}", err);
        let loc = err.location();

        // The range below is zero indexed, hence needs to minus 1 from loc.
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
