use tree_sitter::Node;

// AnchorNode is one of "config_file", "block", "ERROR"
#[derive(Clone, Debug)]
pub struct AnchorNode<'a>(pub Node<'a>);

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

    // containing_ident_nodes returns the containing identifier nodes of this anchor node.
    pub fn containing_ident_nodes(&self) -> Vec<Node<'_>> {
        let mut cursor = self.0.walk();
        match self.0.kind() {
            "block" => self
                .0
                .children(&mut cursor)
                .filter(|child| ["attribute", "block"].contains(&child.kind()))
                .filter_map(|node| node.child(0))
                .collect(),
            "config_file" => {
                if let Some(body) = self.0.child(0) {
                    body.children(&mut cursor)
                        .filter(|child| ["attribute", "block"].contains(&child.kind()))
                        .filter_map(|node| node.child(0))
                        .collect()
                } else {
                    vec![]
                }
            }
            _ => vec![],
        }
    }
}

// ident_node returns the direct identifier node of one of: attribute, block, object_elem.
pub fn ident_node(node: Node<'_>) -> Option<Node<'_>> {
    match node.kind() {
        "attribute" | "block" => node.child(0),
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
