use anyhow::Result;
use tree_sitter::Node;

// AnchorNode is one of "config_file", "block", "ERROR"
#[derive(Clone, Debug)]
pub struct AnchorNode<'a>(Node<'a>);

impl<'a> AnchorNode<'a> {
    // from_node returns the nearest anchor node up from itself.
    pub fn from_node(node: Node<'a>) -> Option<Self> {
        const TARGETS: &[&str] = &["config_file", "block", "ERROR"];

        let mut n = node;
        loop {
            if TARGETS.contains(&n.kind()) {
                return Some(AnchorNode(n));
            }
            match n.parent() {
                Some(p) => n = p,
                None => return None,
            }
        }
    }

    pub fn inner(&self) -> Node<'_> {
        return self.0;
    }
}

// identifier_path_of_nodes returns the path determined by the `nodes`.
// The path segments are identifiers of any block, attribute, ERROR, object key (i.e. object_elem) along the way.
pub fn identifier_path_of_nodes<'a>(text: &'a [u8], nodes: &[Node<'_>]) -> Result<Vec<&'a str>> {
    let mut paths = vec![];
    for node in nodes {
        if let Some(ident) = match node.kind() {
            "attribute" | "block" => node.child(0),
            "ERROR" => node.child(0).filter(|child| child.kind() == "identifier"),
            "object_elem" => node
                .child_by_field_name("key")
                .and_then(|expr| expr.child(0))
                .filter(|vexpr| vexpr.kind() == "variable_expr")
                .and_then(|vexpr| vexpr.child(0))
                .filter(|ident| ident.kind() == "identifier"),
            _ => None,
        } {
            paths.push(ident.utf8_text(text)?);
        }
    }
    Ok(paths)
}

// nodes_to_node returns all the parent nodes together with this node.
pub fn nodes_to_node(node: Node<'_>) -> Vec<Node<'_>> {
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
    nodes
}
