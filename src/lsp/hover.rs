use std::ops;

use crate::{api::metadata_command::Operation, lsp::hcl};
use lsp_document::{IndexedText, Pos, TextAdapter, TextMap};
use tower_lsp::lsp_types::Range;
use tree_sitter::Tree;

pub struct HoverInfo {
    pub content: String,
    pub range: Option<Range>,
}

pub fn get_hover_info(
    text: &IndexedText<String>,
    offset: usize,
    syntax_ts: &Tree,
    operation: &Operation,
) -> Option<HoverInfo> {
    let node = syntax_ts
        .root_node()
        .descendant_for_byte_range(offset, offset)?;
    if node.kind() != "identifier" {
        return None;
    }
    let paths =
        hcl::identifier_path_of_nodes(text.text().as_bytes(), &hcl::nodes_to_node(node)).ok()?;
    let schema = operation.schema_by_path(&paths)?;

    let range = node.range();
    let range = text.range_to_lsp_range(&ops::Range {
        start: Pos {
            line: range.start_point.row as u32,
            col: range.start_point.column as u32,
        },
        end: Pos {
            line: range.end_point.row as u32,
            col: range.end_point.column as u32,
        },
    });
    Some(HoverInfo {
        content: schema.to_hover_content(),
        range: range,
    })
}
