use swc_ecma_ast::Expr;

use crate::shared::utils::core::js_to_expr::NestedStringObject;

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum FnResult {
  Attrs(NestedStringObject),
  Props(NestedStringObject),
  Stylex(Expr),
}

impl FnResult {
  pub(crate) fn as_props(&self) -> Option<&NestedStringObject> {
    match self {
      FnResult::Props(props) => Some(props),
      _ => None,
    }
  }

  #[cfg(test)]
  pub(crate) fn as_stylex(&self) -> Option<&Expr> {
    match self {
      FnResult::Stylex(expr) => Some(expr),
      _ => None,
    }
  }

  pub(crate) fn _as_attrs(&self) -> Option<&NestedStringObject> {
    match self {
      FnResult::Attrs(attrs) => Some(attrs),
      _ => None,
    }
  }
}
