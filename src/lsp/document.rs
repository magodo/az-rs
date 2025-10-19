use tower_lsp::lsp_types::TextDocumentContentChangeEvent;
use tree_sitter::{InputEdit, Parser, Point, Tree};

pub struct Document {
    parser: Parser,
    text: String,
    tree: Option<Tree>,
    //diags: Vec<Diagnostic>,
}

impl Document {
    pub fn new(text: &str) -> Self {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_hcl::LANGUAGE.into())
            .expect("Error loading HCL grammer");
        let tree = parser.parse(text, None);
        Self {
            parser,
            text: text.to_string(),
            tree,
        }
    }

    pub fn apply_change(&mut self, change: &TextDocumentContentChangeEvent) {
        let Some(tree) = &mut self.tree else {
            return;
        };

        let new_text = &change.text;

        // Convert the Point to byte offsets
        let point_to_byte_offset = |text: &str, point: &Point| -> usize {
            text.lines()
                .take(point.row)
                .map(|l| l.len() + 1) // +1 for "/n"
                .sum::<usize>()
                + point.column
        };

        //
        let compute_end_position = |start: &Point, new_text: &str| -> Point {
            let row = start.row;
            let column = start.column;
            match new_text.lines().count() {
                0 => Point { row, column },
                other => {
                    let last = new_text.lines().last().unwrap();
                    Point {
                        row: row + other - 1,
                        // TODO: If the server picks a position_encoding other than UTF8, change needed.
                        column: column + last.len(),
                    }
                }
            }
        };

        let edit = if let Some(range) = change.range {
            // Convert LSP position to Tree-sitter Point
            let start_position = Point {
                row: range.start.line as usize,
                // Tree-sitter Point.column counts UTF8 bytes in the row for incremental parsing.
                // TODO: If the server picks a position_encoding other than UTF8, change needed.
                column: range.start.character as usize,
            };

            let old_end_position = Point {
                row: range.end.line as usize,
                // TODO: If the server picks a position_encoding other than UTF8, change needed.
                column: range.end.character as usize,
            };

            let new_end_position = compute_end_position(&start_position, &new_text);

            let start_byte = point_to_byte_offset(&self.text, &start_position);
            let old_end_byte = point_to_byte_offset(&self.text, &old_end_position);
            let new_end_byte = start_byte + new_text.len();
            InputEdit {
                start_byte,
                old_end_byte,
                new_end_byte,
                start_position,
                old_end_position,
                new_end_position,
            }
        } else {
            InputEdit {
                start_byte: 0,
                old_end_byte: 0,
                new_end_byte: new_text.len(),
                start_position: Point::new(0, 0),
                old_end_position: Point::new(0, 0),
                new_end_position: compute_end_position(&Point::new(0, 0), &new_text),
            }
        };

        // Edit the old tree.
        tree.edit(&edit);

        // Incrementally parse for the new tree.
        self.tree = self.parser.parse(new_text, Some(&tree));
    }
}
