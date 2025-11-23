use hcl::{
    expr::{BinaryOperator, UnaryOperator},
    Number,
};
use hcl_edit::{
    expr::{
        Array, BinaryOp, Conditional, Expression, ForCond, ForExpr, ForIntro, FuncArgs, FuncCall,
        FuncName, Null, Object, ObjectKey, ObjectValue, Parenthesis, Splat, Traversal,
        TraversalOperator, UnaryOp,
    },
    structure::{Attribute, Block, BlockLabel, Body, Structure},
    template::{
        Directive, Element, ElseTemplateExpr, EndforTemplateExpr, EndifTemplateExpr, ForDirective,
        ForTemplateExpr, HeredocTemplate, IfDirective, IfTemplateExpr, Interpolation,
        StringTemplate, Template,
    },
    visit::{visit_attr, visit_block, visit_body, visit_structure, Visit},
    Decorated, Formatted, Ident, Span, Spanned,
};

#[derive(Debug)]
struct Locater {
    offset: usize,
    paths: Vec<String>,
    ident: Option<Decorated<Ident>>,
}

impl Visit for Locater {
    fn visit_body(&mut self, node: &Body) {
        let Some(span) = node.span() else {
            return;
        };
        if !span.contains(&self.offset) {
            return;
        }
        visit_body(self, node);
    }

    fn visit_structure(&mut self, node: &Structure) {
        visit_structure(self, node);
    }

    fn visit_attr(&mut self, node: &Attribute) {
        let Some(span) = node.span() else {
            return;
        };
        if !span.contains(&self.offset) {
            return;
        }
        self.paths.push(node.key.to_string());
        visit_attr(self, node);
    }

    fn visit_block(&mut self, node: &Block) {
        let Some(span) = node.span() else {
            return;
        };
        if !span.contains(&self.offset) {
            return;
        }
        self.paths.push(node.ident.to_string());
        visit_block(self, node);
    }

    fn visit_ident(&mut self, node: &Decorated<Ident>) {
        if let Some(span) = node.span() {
            if span.contains(&self.offset) {
                self.ident = Some(node.clone());
            }
        }
    }

    fn visit_object(&mut self, _: &Object) {
        return;
    }

    fn visit_object_item(
        &mut self,
        _: &hcl_edit::expr::ObjectKey,
        _: &hcl_edit::expr::ObjectValue,
    ) {
        return;
    }

    fn visit_object_key(&mut self, _: &ObjectKey) {
        return;
    }

    fn visit_object_value(&mut self, _: &ObjectValue) {
        return;
    }

    fn visit_array(&mut self, _: &Array) {
        return;
    }

    fn visit_block_label(&mut self, _: &BlockLabel) {
        return;
    }

    fn visit_expr(&mut self, _: &Expression) {
        return;
    }

    fn visit_parenthesis(&mut self, _: &Parenthesis) {
        return;
    }

    fn visit_conditional(&mut self, _: &Conditional) {
        return;
    }

    fn visit_unary_op(&mut self, _: &UnaryOp) {
        return;
    }

    fn visit_binary_op(&mut self, _: &BinaryOp) {
        return;
    }

    fn visit_traversal(&mut self, _: &Traversal) {
        return;
    }

    fn visit_traversal_operator(&mut self, _: &TraversalOperator) {
        return;
    }

    fn visit_func_call(&mut self, _: &FuncCall) {
        return;
    }

    fn visit_func_name(&mut self, _: &FuncName) {
        return;
    }

    fn visit_func_args(&mut self, _: &FuncArgs) {
        return;
    }

    fn visit_for_expr(&mut self, _: &ForExpr) {
        return;
    }

    fn visit_for_intro(&mut self, _: &ForIntro) {
        return;
    }

    fn visit_for_cond(&mut self, _: &ForCond) {
        return;
    }

    fn visit_string_template(&mut self, _: &StringTemplate) {
        return;
    }

    fn visit_heredoc_template(&mut self, _: &HeredocTemplate) {
        return;
    }

    fn visit_template(&mut self, _: &Template) {
        return;
    }

    fn visit_element(&mut self, _: &Element) {
        return;
    }

    fn visit_interpolation(&mut self, _: &Interpolation) {
        return;
    }

    fn visit_directive(&mut self, _: &Directive) {
        return;
    }

    fn visit_if_directive(&mut self, _: &IfDirective) {
        return;
    }

    fn visit_for_directive(&mut self, _: &ForDirective) {
        return;
    }

    fn visit_if_template_expr(&mut self, _: &IfTemplateExpr) {
        return;
    }

    fn visit_else_template_expr(&mut self, _: &ElseTemplateExpr) {
        return;
    }

    fn visit_for_template_expr(&mut self, _: &ForTemplateExpr) {
        return;
    }
}
