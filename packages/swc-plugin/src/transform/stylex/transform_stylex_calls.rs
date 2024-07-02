use swc_core::{
  common::comments::Comments,
  ecma::ast::{CallExpr, Callee, Expr, MemberProp},
};
use swc_ecma_ast::Id;

use crate::shared::enums::core::ModuleCycle;
use crate::ModuleTransformVisitor;

impl<C> ModuleTransformVisitor<C>
where
  C: Comments,
{
  pub(crate) fn transform_call_expression_to_stylex_expr(&mut self, ex: &mut CallExpr) -> Option<Expr> {
    if let Callee::Expr(callee) = &ex.callee {
      match callee.as_ref() {
        Expr::Member(member) => {
          if let MemberProp::Ident(ident) = &member.prop {
            return self.transform_stylex_fns(&ident.to_id(), ex);
          }
        }
        Expr::Ident(ident) => return self.transform_stylex_fns(&ident.to_id(), ex),
        _ => {}
      }
    }

    None
  }

  fn transform_stylex_fns(&mut self, id: &Id, call_expr: &mut CallExpr) -> Option<Expr> {
    if self.cycle == ModuleCycle::TransformEnter {
      let (_, parent_var_decl) = &self.get_call_var_name(call_expr);

      if let Some(parent_var_decl) = parent_var_decl {
        if let Some(value) = self.transform_stylex_keyframes_call(parent_var_decl) {
          return Some(value);
        }
      }

      if let Some(value) = self.transform_stylex_define_vars(call_expr) {
        return Some(value);
      }

      if let Some(value) = self.transform_stylex_create_theme_call(call_expr) {
        return Some(value);
      }

      if let Some(value) = self.transform_stylex_create(call_expr) {
        return Some(value);
      }

      if let Some(value) = self.transform_stylex_create(call_expr) {
        return Some(value);
      }
    }

    if self.cycle == ModuleCycle::TransformExit {
      if self.state.stylex_props_import.contains(id) {
        if let Some(value) = self.transform_stylex_props_call(call_expr) {
          return Some(value);
        }
      }

      if self.state.stylex_attrs_import.contains(id) {
        if let Some(value) = self.transform_stylex_attrs_call(call_expr) {
          return Some(value);
        }
      }

      if let Some(value) = self.transform_stylex_call(call_expr) {
        return Some(value);
      }

      if let Some(value) = self.transform_stylex_attrs_call(call_expr) {
        return Some(value);
      }

      if let Some(value) = self.transform_stylex_props_call(call_expr) {
        return Some(value);
      }
    }

    None
  }
}
