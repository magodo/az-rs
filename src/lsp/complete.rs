use crate::{api::metadata_command::Operation, lsp::hcl};
use tower_lsp::lsp_types::CompletionItem;
use tree_sitter::Tree;

#[derive(Clone, Debug)]
pub struct CompletionInfo<'a> {
    // The identifier path from the top to the parent identifier node, if any
    pub path: Vec<&'a str>,
}

impl<'a> CompletionInfo<'a> {
    fn new(path: Vec<&'a str>) -> Self {
        Self { path }
    }

    fn build_completion_items(&self, operation: &Operation) -> Option<Vec<CompletionItem>> {
        let schema = operation.schema_by_path(&self.path)?;
        let props = &schema.props.as_ref()?;
        Some(props.iter().map(|prop| prop.to_completion_item()).collect())
    }
}

pub fn get_completion_items<'a, 'b>(
    text: &'a [u8],
    offset: usize,
    syntax_ts: &'a Tree,
    last_syntax_ts: &'a Tree,
    operation: &'b Operation,
) -> Option<Vec<CompletionItem>> {
    let comp_info = completion_info_by_offset(text, offset, syntax_ts, last_syntax_ts)?;
    comp_info.build_completion_items(operation)
}

// completion_info_by_offset returns the completion info.
// TODO: Complete dones't work correctly for object type. Hence suggest to use block instead.
fn completion_info_by_offset<'a>(
    text: &'a [u8],
    offset: usize,
    syntax_ts: &'a Tree,
    last_syntax_ts: &'a Tree,
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

    if anchor_node.inner().is_error() {
        // Error anchor node implies an insert into the body or object, fallback to using
        // the last syntax tree, assuming it is error free. The offset in this case works as there
        // is only "one char diff".
        //
        // Note: The "one char diff" assumption is weak as the client's did_change can introduce
        // multiple bytes. While nvim seems to always send to completion right after the did_change
        // with one char change.
        let node = last_syntax_ts
            .root_node()
            .descendant_for_byte_range(offset, offset)?;
        anchor_node = hcl::AnchorNode::from_node(node)?;

        // If the old syntax is still error, then just quit.
        // This can happen when the user editing a just typed attribute name (without the "=val" part).
        if anchor_node.inner().is_error() {
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
    let path =
        hcl::identifier_path_of_nodes(text, &hcl::nodes_to_node(anchor_node.inner())).ok()?;

    Some(CompletionInfo::new(path))
}
