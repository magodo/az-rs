use crate::lsp::hcl;
use tree_sitter::Tree;

#[derive(Clone, Debug)]
pub struct CompletionInfo<'a> {
    // The identifier path from the top to the parent identifier node, if any
    pub path: Vec<&'a str>,
    // The existing sibling identifiers
    // TODO: we shall allow repeatable indentities
    pub exist_idents: Vec<&'a str>,
}

// completion_info_by_offset returns the completion info.
// TODO: Complete dones't work correctly for object type. Hence suggest to use block instead.
pub fn completion_info_by_offset<'a, 'b>(
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

    if anchor_node.inner().is_error() {
        // Error anchor node implies an insert into the body or object, fallback to using
        // the last syntax tree, assuming it is error free. The offset in this case works as there
        // is only one char diff.
        let node = last_syntax_ts
            .root_node()
            .descendant_for_byte_range(offset, offset)?;
        anchor_node = hcl::AnchorNode::from_node(node)?;
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

    Some(CompletionInfo {
        path,
        exist_idents: vec![], // TODO
    })
}
