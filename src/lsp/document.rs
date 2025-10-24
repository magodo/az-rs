use lsp_document::{IndexedText, TextMap};
use tower_lsp::lsp_types::{
    Diagnostic, DiagnosticSeverity, NumberOrString, Position, Range, TextDocumentContentChangeEvent,
};
use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator, Tree};

pub struct Document {
    parser: Parser,
    query_error: Query,
    query_miss: Query,
    tree: Option<Tree>,
    text: IndexedText<String>,
}

impl Document {
    pub fn new(text: &str) -> Self {
        let lang = tree_sitter_hcl::LANGUAGE;
        let mut parser = Parser::new();
        parser
            .set_language(&lang.into())
            .expect("Error loading HCL grammer");
        let text = IndexedText::new(text.to_string());
        let tree = parser.parse(text.text(), None);
        let query_error = Query::new(&lang.into(), "(ERROR) @error").unwrap();
        let query_miss = Query::new(&lang.into(), "(MISSING) @miss").unwrap();
        Self {
            parser,
            query_error,
            query_miss,
            tree,
            text,
        }
    }

    pub fn apply_change(&mut self, change: &TextDocumentContentChangeEvent) {
        if change.range.is_some() {
            panic!("Incremental change is not supported");
        }
        self.tree = self.parser.parse(&change.text, None);
        self.text = IndexedText::new(change.text.clone());
        dbg!(self.tree.as_ref().unwrap().root_node().to_sexp());
    }

    pub fn get_diagnostics(&self) -> Vec<Diagnostic> {
        let Some(ref tree) = self.tree else {
            return Vec::new();
        };
        let root_node = tree.root_node();

        let mut diags = Vec::new();
        let mut add_diag = |query: &Query, severity: DiagnosticSeverity, code: String| {
            let mut query_cursor = QueryCursor::new();
            query_cursor
                .matches(query, root_node, self.text.text().as_bytes())
                .for_each(|m| {
                    m.captures.iter().for_each(|capture| {
                        let node = capture.node;
                        let range = self.text.offset_range_to_range(node.byte_range()).unwrap();
                        let range = Range::new(
                            Position::new(range.start.line, range.start.col),
                            Position::new(range.end.line, range.end.col),
                        );
                        diags.push(Diagnostic {
                            range,
                            severity: Some(severity),
                            code: Some(NumberOrString::String(code.clone())),
                            message: code.clone(),
                            source: Some("azure".to_string()),
                            ..Default::default()
                        });
                    });
                });
        };

        add_diag(
            &self.query_error,
            DiagnosticSeverity::ERROR,
            "ERROR".to_string(),
        );
        add_diag(
            &self.query_miss,
            DiagnosticSeverity::WARNING,
            "MISSING".to_string(),
        );
        tracing::debug!("Diagnostics: {:#?}", diags);
        diags
    }
}
