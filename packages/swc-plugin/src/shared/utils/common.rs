use std::{
  any::type_name,
  collections::HashSet,
  hash::{DefaultHasher, Hash, Hasher},
  ops::Deref,
  path::PathBuf,
};

use radix_fmt::radix;
use swc_core::{
  common::{FileName, Span, DUMMY_SP},
  ecma::{
    ast::{
      BinExpr, BinaryOp, BindingIdent, Bool, Decl, Expr, ExprOrSpread, Ident, ImportDecl,
      ImportSpecifier, KeyValueProp, Lit, Module, ModuleDecl, ModuleExportName, ModuleItem, Number,
      ObjectLit, Pat, Prop, PropName, PropOrSpread, Stmt, Str, Tpl, UnaryExpr, UnaryOp,
      VarDeclarator,
    },
    visit::{Fold, FoldWith},
  },
};

use crate::shared::{
  constants::{self, messages::ILLEGAL_PROP_VALUE},
  enums::{TopLevelExpression, TopLevelExpressionKind, VarDeclAction},
  regex::{DASHIFY_REGEX, IDENT_PROP_REGEX},
  structures::{
    functions::{FunctionConfigType, FunctionMap, FunctionType},
    state_manager::StateManager,
  },
};

use super::{
  css::stylex::evaluate::{evaluate_cached, State},
  js::stylex::stylex_types::{BaseCSSType, CSSSyntax},
};

struct SpanReplacer;

impl Fold for SpanReplacer {
  fn fold_span(&mut self, _: Span) -> Span {
    DUMMY_SP
  }
}

fn _replace_spans(expr: &mut Expr) -> Expr {
  expr.clone().fold_children_with(&mut SpanReplacer)
}

pub fn prop_or_spread_expression_creator(key: String, value: Expr) -> PropOrSpread {
  PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
    key: string_to_prop_name(key).unwrap(),
    value: Box::new(value),
  })))
}

pub(crate) fn prop_or_spread_string_creator(key: String, value: String) -> PropOrSpread {
  let value = string_to_expression(value);

  match value {
    Some(value) => prop_or_spread_expression_creator(key, value),
    None => panic!("Value is not a string"),
  }
}

pub(crate) fn prop_or_spread_boolean_creator(key: String, value: Option<bool>) -> PropOrSpread {
  match value {
    Some(value) => prop_or_spread_expression_creator(
      key,
      Expr::Lit(Lit::Bool(Bool {
        span: DUMMY_SP,
        value,
      })),
    ),
    None => panic!("Value is not a boolean"),
  }
}

// Converts a string to an expression.
pub(crate) fn string_to_expression(value: String) -> Option<Expr> {
  Option::Some(Expr::Lit(Lit::Str(value.into())))
}

// Converts a string to an expression.
pub(crate) fn string_to_prop_name(value: String) -> Option<PropName> {
  if IDENT_PROP_REGEX.is_match(value.as_str()) && value.parse::<i64>().is_err() {
    Some(PropName::Ident(Ident::new(value.clone().into(), DUMMY_SP)))
  } else {
    Some(PropName::Str(Str {
      span: DUMMY_SP,
      value: value.clone().into(),
      raw: None,
    }))
  }
}

// Converts a number to an expression.
pub(crate) fn number_to_expression(value: f64) -> Option<Expr> {
  Option::Some(Expr::Lit(Lit::Num(Number {
    span: DUMMY_SP,
    value,
    raw: Option::None,
  })))
}

pub(crate) fn extract_filename_from_path(path: FileName) -> String {
  match path {
    FileName::Real(path_buf) => path_buf.file_stem().unwrap().to_str().unwrap().to_string(),
    _ => "UnknownFile".to_string(),
  }
}

pub(crate) fn extract_path(path: FileName) -> String {
  match path {
    FileName::Real(path_buf) => path_buf.to_str().unwrap().to_string(),
    _ => "UnknownFile".to_string(),
  }
}

pub(crate) fn extract_filename_with_ext_from_path(path: FileName) -> Option<String> {
  match path {
    FileName::Real(path_buf) => {
      Option::Some(path_buf.file_name().unwrap().to_str().unwrap().to_string())
    }
    _ => Option::None,
  }
}

pub fn create_hash(value: &str) -> String {
  radix(murmur2::murmur2(value.as_bytes(), 1), 36).to_string()
}

pub(crate) fn get_string_val_from_lit(value: &Lit) -> Option<String> {
  match value {
    Lit::Str(str) => Option::Some(format!("{}", str.value)),
    Lit::Num(num) => Option::Some(format!("{}", num.value)),
    Lit::BigInt(big_int) => Option::Some(format!("{}", big_int.value)),
    _ => Option::None, // _ => panic!("{}", ILLEGAL_PROP_VALUE),
  }
}

pub(crate) fn get_key_str(key_value: &KeyValueProp) -> String {
  let key = &key_value.key;
  let mut should_wrap_in_quotes = false;

  let key = match key {
    PropName::Ident(ident) => &*ident.sym,
    PropName::Str(str) => {
      should_wrap_in_quotes = false;

      &*str.value
    }
    _ => panic!("Key is not recognized"),
  };

  wrap_key_in_quotes(key, &should_wrap_in_quotes)
}

pub(crate) fn wrap_key_in_quotes(key: &str, should_wrap_in_quotes: &bool) -> String {
  if *should_wrap_in_quotes {
    format!("\"{}\"", key)
  } else {
    key.to_string()
  }
}

// pub(crate) fn push_css_anchor_prop(props: &mut Vec<PropOrSpread>) {
//     props.push(prop_or_spread_boolean_creator(
//         "$$css".to_string(),
//         Option::Some(true),
//     ))
// }

pub(crate) fn get_pat_as_string(pat: &Pat) -> String {
  match pat {
    Pat::Ident(ident) => ident.sym.to_string(),
    _ => todo!("get_pat_as_string: Pat"),
  }
}

// pub(crate) fn expr_or_spread_object_expression_creator(
//     key: String,
//     value: Box<Expr>,
// ) -> ExprOrSpread {
//     let expr = Box::new(Expr::Object(ObjectLit {
//         span: DUMMY_SP,
//         props: vec![prop_or_spread_box_expression_creator(key.as_ref(), value)],
//     }));

//     ExprOrSpread {
//         expr,
//         spread: Option::None,
//     }
// }

pub(crate) fn expr_or_spread_string_expression_creator(value: String) -> ExprOrSpread {
  let expr = Box::new(string_to_expression(value).expect(constants::messages::NON_STATIC_VALUE));

  ExprOrSpread {
    expr,
    spread: Option::None,
  }
}

pub(crate) fn expr_or_spread_number_expression_creator(value: f64) -> ExprOrSpread {
  let expr = Box::new(number_to_expression(value).unwrap());

  ExprOrSpread {
    expr,
    spread: Option::None,
  }
}

// pub(crate) fn prop_or_spread_box_expression_creator(key: &str, value: Box<Expr>) -> PropOrSpread {
//     PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
//         key: PropName::Ident(Ident::new(key.into(), DUMMY_SP)),
//         value,
//     })))
// }

pub fn reduce_ident_count<'a>(state: &'a mut StateManager, ident: &'a Ident) {
  *state.var_decl_count_map.entry(ident.to_id()).or_insert(0) -= 1;

  // eprintln!(
  //     "{} {:?} {:?}",
  //     Colorize::red("!!!! reduce_ident_count !!!!"),
  //     state
  //         .var_decl_count_map
  //         .get(&Ident::new("styles".into(), DUMMY_SP).to_id()),
  //     ident
  // );
}

pub fn increase_ident_count(state: &mut StateManager, ident: &Ident) {
  increase_ident_count_by_count(state, ident, 1);

  // eprintln!(
  //     "{} {:?} {:?}",
  //     Colorize::green("!!!! increase_ident_count !!!!"),
  //     state
  //         .var_decl_count_map
  //         .get(&Ident::new("styles".into(), DUMMY_SP).to_id()),
  //     ident
  // );
}

pub fn increase_ident_count_by_count(state: &mut StateManager, ident: &Ident, count: i8) {
  let ident_id = &ident.to_id();
  *state
    .var_decl_count_map
    .entry(ident_id.clone())
    .or_insert(0) += count;
}

pub fn get_var_decl_by_ident<'a>(
  ident: &'a Ident,
  state: &'a mut StateManager,
  functions: &'a FunctionMap,
  action: VarDeclAction,
) -> Option<VarDeclarator> {
  match action {
    VarDeclAction::Increase => increase_ident_count(state, ident),
    VarDeclAction::Reduce => reduce_ident_count(state, ident),
    VarDeclAction::None => {}
  };

  match get_var_decl_from(state, ident) {
    Some(var_decl) => Some(var_decl.clone()),
    None => {
      let func = functions.identifiers.get(&ident.to_id());

      match func {
        Some(func) => {
          let func = func.clone();
          match func {
            FunctionConfigType::Regular(func) => {
              match func.fn_ptr {
                FunctionType::Mapper(func) => {
                  // let arg = Expr::Ident(ident.clone());
                  let result = func();

                  println!("!!!!! ident: {:?}, result: {:?}", ident, result);

                  let var_decl = VarDeclarator {
                    span: DUMMY_SP,
                    name: Pat::Ident(BindingIdent {
                      id: ident.clone(),
                      type_ann: Option::None,
                    }),
                    init: Option::Some(Box::new(result)), // Clone the result
                    definite: false,
                  };

                  let var_declarator = var_decl.clone();
                  Option::Some(var_declarator)
                }
                _ => panic!("Function type not supported"),
              }
            }
            FunctionConfigType::Map(_) => todo!("FunctionConfigType::Map"),
          }
        }
        None => Option::None,
      }
    }
  }
}

pub fn get_import_by_ident<'a>(
  ident: &'a Ident,
  state: &'a mut StateManager,
) -> Option<ImportDecl> {
  get_import_from(state, ident).cloned()
}

pub(crate) fn get_var_decl_from<'a>(
  state: &'a StateManager,
  ident: &'a Ident,
) -> Option<&'a VarDeclarator> {
  state.declarations.iter().find(|var_declarator| {
    if let Pat::Ident(binding_indent) = &var_declarator.name {
      return binding_indent.sym == ident.sym;
    }

    false
  })
}

pub(crate) fn get_import_from<'a>(
  state: &'a StateManager,
  ident: &'a Ident,
) -> Option<&'a ImportDecl> {
  state.top_imports.iter().find(|import| {
    import.specifiers.iter().any(|specifier| match specifier {
      ImportSpecifier::Named(named_import) => {
        named_import.local.sym == ident.sym || {
          if let Some(imported) = &named_import.imported {
            match imported {
              ModuleExportName::Ident(export_ident) => export_ident.sym == ident.sym,
              ModuleExportName::Str(str) => str.value == ident.sym,
            }
          } else {
            false
          }
        }
      }
      ImportSpecifier::Default(default_import) => default_import.local.sym == ident.sym,
      ImportSpecifier::Namespace(namespace_import) => namespace_import.local.sym == ident.sym,
    })
  })
}

pub(crate) fn get_var_decl_by_ident_or_member<'a>(
  state: &'a StateManager,
  ident: &'a Ident,
) -> Option<&'a VarDeclarator> {
  state.declarations.iter().find(|var_declarator| {
    if let Pat::Ident(binding_indent) = &var_declarator.name {
      if binding_indent.sym == ident.sym {
        return true;
      }
    }

    var_declarator
      .init
      .as_ref()
      .and_then(|init| init.as_call())
      .and_then(|call| call.callee.as_expr())
      .and_then(|callee| callee.as_member())
      .and_then(|member| member.prop.as_ident())
      .map_or(false, |member_ident| member_ident.sym == ident.sym)
  })
}

pub fn get_expr_from_var_decl(var_decl: &VarDeclarator) -> Expr {
  match &var_decl.init {
    Some(var_decl_init) => unbox(var_decl_init.clone()),
    None => panic!("Variable declaration is not an expression"),
  }
}

pub fn _unbox_option<T>(item: Option<Box<T>>) -> T {
  match item {
    Some(item) => unbox(item),
    None => panic!("Item is undefined"),
  }
}

pub fn unbox<T>(value: Box<T>) -> T {
  *value
}

pub fn expr_to_num(expr_num: &Expr, traversal_state: &mut StateManager) -> f32 {
  match &expr_num {
    Expr::Ident(ident) => ident_to_number(ident, traversal_state, &FunctionMap::default()),
    Expr::Lit(lit) => lit_to_num(lit),
    Expr::Unary(unary) => unari_to_num(unary, traversal_state),
    Expr::Bin(lit) => {
      dbg!(&traversal_state.var_decl_count_map);

      let mut state = State::new(traversal_state);

      match binary_expr_to_num(lit, &mut state) {
        Some(result) => result,
        None => panic!("Binary expression is not a number"),
      }
    }
    _ => panic!("Expression in not a number {:?}", expr_num),
  }
}

fn ident_to_string(ident: &Ident, state: &mut StateManager, functions: &FunctionMap) -> String {
  let var_decl = get_var_decl_by_ident(ident, state, functions, VarDeclAction::Reduce);

  println!("var_decl: {:?}, ident: {:?}", var_decl, ident);

  match &var_decl {
    Some(var_decl) => {
      let var_decl_expr = get_expr_from_var_decl(var_decl);

      match &var_decl_expr {
        Expr::Lit(lit) => get_string_val_from_lit(lit).expect(ILLEGAL_PROP_VALUE),
        Expr::Ident(ident) => ident_to_string(ident, state, functions),
        _ => panic!("{}", ILLEGAL_PROP_VALUE),
      }
    }
    None => panic!("{}", ILLEGAL_PROP_VALUE),
  }
}

pub fn expr_to_str(
  expr_string: &Expr,
  state: &mut StateManager,
  functions: &FunctionMap,
) -> String {
  match &expr_string {
    Expr::Ident(ident) => ident_to_string(ident, state, functions),
    Expr::Lit(lit) => get_string_val_from_lit(lit).expect("Value is not a string"),
    _ => panic!("Expression in not a string {:?}", expr_string),
  }
}

pub fn unari_to_num(unary_expr: &UnaryExpr, state: &mut StateManager) -> f32 {
  let arg = unary_expr.arg.as_ref();
  let op = unary_expr.op;

  match &op {
    UnaryOp::Minus => expr_to_num(arg, state) * -1.0,
    UnaryOp::Plus => expr_to_num(arg, state),
    _ => panic!("Union operation '{}' is invalid", op),
  }
}

pub fn binary_expr_to_num(binary_expr: &BinExpr, state: &mut State) -> Option<f32> {
  let binary_expr = binary_expr.clone();

  let op = binary_expr.op;
  let Some(left) = evaluate_cached(&binary_expr.left, state) else {
    dbg!(binary_expr.left);

    if !state.confident {
      return Option::None;
    }

    panic!("Left expression is not a number")
  };

  let Some(right) = evaluate_cached(&binary_expr.right, state) else {
    dbg!(binary_expr.right);

    if !state.confident {
      return Option::None;
    }

    panic!("Right expression is not a number")
  };

  let result = match &op {
    BinaryOp::Add => {
      expr_to_num(left.as_expr()?, &mut state.traversal_state)
        + expr_to_num(right.as_expr()?, &mut state.traversal_state)
    }
    BinaryOp::Sub => {
      expr_to_num(left.as_expr()?, &mut state.traversal_state)
        - expr_to_num(right.as_expr()?, &mut state.traversal_state)
    }
    BinaryOp::Mul => {
      expr_to_num(left.as_expr()?, &mut state.traversal_state)
        * expr_to_num(right.as_expr()?, &mut state.traversal_state)
    }
    BinaryOp::Div => {
      expr_to_num(left.as_expr()?, &mut state.traversal_state)
        / expr_to_num(right.as_expr()?, &mut state.traversal_state)
    }
    BinaryOp::Mod => {
      expr_to_num(left.as_expr()?, &mut state.traversal_state)
        % expr_to_num(right.as_expr()?, &mut state.traversal_state)
    }
    BinaryOp::Exp => expr_to_num(left.as_expr()?, &mut state.traversal_state)
      .powf(expr_to_num(right.as_expr()?, &mut state.traversal_state)),
    BinaryOp::RShift => {
      ((expr_to_num(left.as_expr()?, &mut state.traversal_state) as i32)
        >> expr_to_num(right.as_expr()?, &mut state.traversal_state) as i32) as f32
    }
    BinaryOp::LShift => {
      ((expr_to_num(left.as_expr()?, &mut state.traversal_state) as i32)
        << expr_to_num(right.as_expr()?, &mut state.traversal_state) as i32) as f32
    }
    BinaryOp::BitAnd => {
      ((expr_to_num(left.as_expr()?, &mut state.traversal_state) as i32)
        & expr_to_num(right.as_expr()?, &mut state.traversal_state) as i32) as f32
    }
    BinaryOp::BitOr => {
      ((expr_to_num(left.as_expr()?, &mut state.traversal_state) as i32)
        | expr_to_num(right.as_expr()?, &mut state.traversal_state) as i32) as f32
    }
    BinaryOp::BitXor => {
      ((expr_to_num(left.as_expr()?, &mut state.traversal_state) as i32)
        ^ expr_to_num(right.as_expr()?, &mut state.traversal_state) as i32) as f32
    }
    BinaryOp::In => {
      if expr_to_num(right.as_expr()?, &mut state.traversal_state) == 0.0 {
        1.0
      } else {
        0.0
      }
    }
    BinaryOp::InstanceOf => {
      if expr_to_num(right.as_expr()?, &mut state.traversal_state) == 0.0 {
        1.0
      } else {
        0.0
      }
    }
    BinaryOp::EqEq => {
      if expr_to_num(left.as_expr()?, &mut state.traversal_state)
        == expr_to_num(right.as_expr()?, &mut state.traversal_state)
      {
        1.0
      } else {
        0.0
      }
    }
    BinaryOp::NotEq => {
      if expr_to_num(left.as_expr()?, &mut state.traversal_state)
        != expr_to_num(right.as_expr()?, &mut state.traversal_state)
      {
        1.0
      } else {
        0.0
      }
    }
    BinaryOp::EqEqEq => {
      if expr_to_num(left.as_expr()?, &mut state.traversal_state)
        == expr_to_num(right.as_expr()?, &mut state.traversal_state)
      {
        1.0
      } else {
        0.0
      }
    }
    BinaryOp::NotEqEq => {
      if expr_to_num(left.as_expr()?, &mut state.traversal_state)
        != expr_to_num(right.as_expr()?, &mut state.traversal_state)
      {
        1.0
      } else {
        0.0
      }
    }
    BinaryOp::Lt => {
      if expr_to_num(left.as_expr()?, &mut state.traversal_state)
        < expr_to_num(right.as_expr()?, &mut state.traversal_state)
      {
        1.0
      } else {
        0.0
      }
    }
    BinaryOp::LtEq => {
      if expr_to_num(left.as_expr()?, &mut state.traversal_state)
        <= expr_to_num(right.as_expr()?, &mut state.traversal_state)
      {
        1.0
      } else {
        0.0
      }
    }
    BinaryOp::Gt => {
      if expr_to_num(left.as_expr()?, &mut state.traversal_state)
        > expr_to_num(right.as_expr()?, &mut state.traversal_state)
      {
        1.0
      } else {
        0.0
      }
    }
    BinaryOp::GtEq => {
      if expr_to_num(left.as_expr()?, &mut state.traversal_state)
        >= expr_to_num(right.as_expr()?, &mut state.traversal_state)
      {
        1.0
      } else {
        0.0
      }
    }
    // #region Logical
    BinaryOp::LogicalOr => {
      println!("!!!!__ state.confident33333: {:#?}", state.confident);

      let was_confident = state.confident;

      let result = evaluate_cached(left.as_expr()?, state);

      let left = result.unwrap();
      let left = left.as_expr().unwrap();

      let left_confident = state.confident;

      state.confident = was_confident;

      let result = evaluate_cached(right.as_expr()?, state);

      let right = result.unwrap();
      let right = right.as_expr().unwrap();
      let right_confident = state.confident;

      let left = expr_to_num(left, &mut state.traversal_state);
      let right = expr_to_num(right, &mut state.traversal_state);

      state.confident = left_confident && (left != 0.0 || right_confident);
      println!("!!!!__ state.confident44444: {:#?}", state.confident);

      if !state.confident {
        return Option::None;
      }

      if left != 0.0 {
        left
      } else {
        right
      }
    }
    BinaryOp::LogicalAnd => {
      let was_confident = state.confident;

      let result = evaluate_cached(left.as_expr()?, state);

      let left = result.unwrap();
      let left = left.as_expr().unwrap();

      let left_confident = state.confident;

      state.confident = was_confident;

      let result = evaluate_cached(right.as_expr()?, state);

      let right = result.unwrap();
      let right = right.as_expr().unwrap();
      let right_confident = state.confident;

      let left = expr_to_num(left, &mut state.traversal_state);
      let right = expr_to_num(right, &mut state.traversal_state);

      state.confident = left_confident && (left == 0.0 || right_confident);

      if !state.confident {
        return Option::None;
      }

      if left != 0.0 {
        right
      } else {
        left
      }
    }
    BinaryOp::NullishCoalescing => {
      let was_confident = state.confident;

      let result = evaluate_cached(left.as_expr()?, state);

      let left = result.unwrap();
      let left = left.as_expr().unwrap();

      let left_confident = state.confident;

      state.confident = was_confident;

      let result = evaluate_cached(right.as_expr()?, state);

      let right = result.unwrap();
      let right = right.as_expr().unwrap();
      let right_confident = state.confident;

      let left = expr_to_num(left, &mut state.traversal_state);
      let right = expr_to_num(right, &mut state.traversal_state);

      state.confident = left_confident && !!(left == 0.0 || right_confident);

      if !state.confident {
        return Option::None;
      }

      if left == 0.0 {
        right
      } else {
        left
      }
    }
    // #endregion Logical
    BinaryOp::ZeroFillRShift => {
      ((expr_to_num(left.as_expr()?, &mut state.traversal_state) as i32)
        >> expr_to_num(right.as_expr()?, &mut state.traversal_state) as i32) as f32
    }
  };

  Option::Some(result)
}

pub fn ident_to_number(
  ident: &Ident,
  traveral_state: &mut StateManager,
  functions: &FunctionMap,
) -> f32 {
  // 1. Get the variable declaration
  let var_decl = get_var_decl_by_ident(ident, traveral_state, functions, VarDeclAction::Reduce);

  // 2. Check if it is a variable
  match &var_decl {
    Some(var_decl) => {
      // 3. Do the correct conversion according to the expression
      let var_decl_expr = get_expr_from_var_decl(var_decl);

      let mut state: State = State::new(traveral_state);

      match &var_decl_expr {
        Expr::Bin(bin_expr) => match binary_expr_to_num(bin_expr, &mut state) {
          Some(result) => result,
          None => panic!("Binary expression is not a number"),
        },
        Expr::Unary(unary_expr) => unari_to_num(unary_expr, traveral_state),
        Expr::Lit(lit) => lit_to_num(lit),
        _ => panic!("Varable {:?} is not a number", var_decl_expr),
      }
    }
    None => panic!("Variable {} is not declared", ident.sym),
  }
}

pub fn lit_to_num(lit_num: &Lit) -> f32 {
  match &lit_num {
    Lit::Bool(Bool { value, .. }) => {
      if value == &true {
        1.0
      } else {
        0.0
      }
    }
    Lit::Num(num) => num.value as f32,
    Lit::Str(str) => {
      let Result::Ok(num) = str.value.parse::<f32>() else {
        panic!("Value in not a number");
      };

      num
    }
    _ => {
      panic!("Value in not a number");
    }
  }
}

pub fn handle_tpl_to_expression(
  tpl: &swc_core::ecma::ast::Tpl,
  state: &mut StateManager,
  functions: &FunctionMap,
) -> Expr {
  // Clone the template, so we can work on it
  let mut tpl = tpl.clone();

  // Loop through each expression in the template
  for expr in tpl.exprs.iter_mut() {
    // Check if the expression is an identifier
    if let Expr::Ident(ident) = expr.as_ref() {
      // Find the variable declaration for this identifier in the AST
      let var_decl = get_var_decl_by_ident(ident, state, functions, VarDeclAction::Reduce);

      // If a variable declaration was found
      match &var_decl {
        Some(var_decl) => {
          // Swap the placeholder expression in the template with the variable declaration's initializer
          std::mem::swap(
            expr,
            &mut var_decl
              .init
              .clone()
              .expect("Variable declaration has no initializer"),
          );
        }
        None => {}
      }
    };
  }

  Expr::Tpl(tpl.clone())
}

pub fn expr_tpl_to_string(tpl: &Tpl, state: &mut StateManager, functions: &FunctionMap) -> String {
  let mut tpl_str: String = String::new();

  for (i, quasi) in tpl.quasis.iter().enumerate() {
    tpl_str.push_str(quasi.raw.as_ref());

    if i < tpl.exprs.len() {
      match &tpl.exprs[i].as_ref() {
        Expr::Ident(ident) => {
          let ident = get_var_decl_by_ident(ident, state, functions, VarDeclAction::Reduce);

          match ident {
            Some(var_decl) => {
              let var_decl_expr = get_expr_from_var_decl(&var_decl);

              let value = match &var_decl_expr {
                Expr::Lit(lit) => {
                  get_string_val_from_lit(lit).expect(constants::messages::ILLEGAL_PROP_VALUE)
                }
                _ => panic!("{}", constants::messages::ILLEGAL_PROP_VALUE),
              };

              tpl_str.push_str(value.as_str());
            }
            None => panic!("{}", constants::messages::NON_STATIC_VALUE),
          }
        }
        Expr::Bin(bin) => tpl_str.push_str(
          transform_bin_expr_to_number(bin, state)
            .to_string()
            .as_str(),
        ),
        Expr::Lit(lit) => tpl_str
          .push_str(&get_string_val_from_lit(lit).expect(constants::messages::ILLEGAL_PROP_VALUE)),
        _ => panic!("Value not suppported"), // Handle other expression types as needed
      }
    }
  }

  tpl_str
}

pub fn evaluate_bin_expr(op: BinaryOp, left: f32, right: f32) -> f32 {
  match &op {
    BinaryOp::Add => left + right,
    BinaryOp::Sub => left - right,
    BinaryOp::Mul => left * right,
    BinaryOp::Div => left / right,
    _ => panic!("Operator '{}' is not supported", op),
  }
}

pub fn transform_bin_expr_to_number(bin: &BinExpr, traversal_state: &mut StateManager) -> f32 {
  let mut state = State::new(traversal_state);
  let op = bin.op;
  let Some(left) = evaluate_cached(&bin.left, &mut state) else {
    panic!("Left expression is not a number")
  };

  let Some(right) = evaluate_cached(&bin.right, &mut state) else {
    panic!("Left expression is not a number")
  };
  let left = expr_to_num(left.as_expr().unwrap(), traversal_state);
  let right = expr_to_num(right.as_expr().unwrap(), traversal_state);

  evaluate_bin_expr(op, left, right)
}

pub(crate) fn type_of<T>(_: T) -> &'static str {
  type_name::<T>()
}

// pub fn get_value_as_string_from_ident(
//     value_ident: &Ident,
//     declarations: &Vec<VarDeclarator>,
//     var_dec_count_map: &mut HashMap<Id, i8>,
// ) -> String {
//     reduce_ident_count(var_dec_count_map, &value_ident);

//     let var_decl = get_var_decl_from(declarations, &value_ident);

//     match &var_decl {
//         Some(var_decl) => {
//             let var_decl_expr = get_expr_from_var_decl(var_decl);

//             match &var_decl_expr {
//                 Expr::Lit(lit) => get_string_val_from_lit(lit),
//                 Expr::Ident(ident) => {
//                     get_value_as_string_from_ident(ident)
//                 }
//                 _ => panic!("Value type not supported"),
//             }
//         }
//         None => {
//             println!("value_ident: {:?}", value_ident);
//             panic!("Variable not declared")
//         }
//     }
// }

fn prop_name_eq(a: &PropName, b: &PropName) -> bool {
  match (a, b) {
    (PropName::Ident(a), PropName::Ident(b)) => a.sym == b.sym,
    (PropName::Str(a), PropName::Str(b)) => a.value == b.value,
    (PropName::Num(a), PropName::Num(b)) => (a.value - b.value).abs() < std::f64::EPSILON,

    (PropName::BigInt(a), PropName::BigInt(b)) => a.value == b.value,
    // Add more cases as needed
    _ => false,
  }
}

pub(crate) fn remove_duplicates(props: Vec<PropOrSpread>) -> Vec<PropOrSpread> {
  let mut set = HashSet::new();
  let mut result = vec![];

  for prop in props.into_iter().rev() {
    let key = match &prop {
      PropOrSpread::Prop(prop) => match prop.as_ref().clone() {
        Prop::Shorthand(ident) => ident.sym.clone(),
        Prop::KeyValue(kv) => match kv.clone().key {
          PropName::Ident(ident) => ident.sym.clone(),
          PropName::Str(str_) => str_.value.clone(),
          _ => continue,
        },
        _ => continue,
      },
      _ => continue,
    };

    if set.insert(key) {
      result.push(prop);
    }
  }

  result.reverse();

  result
}

pub(crate) fn deep_merge_props(
  old_props: Vec<PropOrSpread>,
  mut new_props: Vec<PropOrSpread>,
) -> Vec<PropOrSpread> {
  for prop in old_props {
    match prop {
      PropOrSpread::Prop(prop) => match *prop {
        Prop::KeyValue(mut kv) => {
          if new_props.iter().any(|p| match p {
            PropOrSpread::Prop(p) => match **p {
              Prop::KeyValue(ref existing_kv) => prop_name_eq(&kv.key, &existing_kv.key),
              _ => false,
            },
            _ => false,
          }) {
            if let Expr::Object(ref mut obj1) = *kv.value {
              new_props.push(PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                key: kv.key.clone(),
                value: Box::new(Expr::Object(ObjectLit {
                  span: DUMMY_SP,
                  props: deep_merge_props(obj1.props.clone(), obj1.props.clone()),
                })),
              }))));
            }
          } else {
            new_props.push(PropOrSpread::Prop(Box::new(Prop::KeyValue(kv))));
          }
        }
        _ => new_props.push(PropOrSpread::Prop(Box::new(*prop))),
      },
      _ => new_props.push(prop),
    }
  }

  remove_duplicates(new_props.into_iter().rev().collect())
}

pub(crate) fn get_css_value(key_value: KeyValueProp) -> (Box<Expr>, Option<BaseCSSType>) {
  let Some(obj) = key_value.value.as_object() else {
    return (key_value.value, Option::None);
  };

  for prop in obj.props.clone().into_iter() {
    match prop {
      PropOrSpread::Spread(_) => todo!("Spread in not supported"),
      PropOrSpread::Prop(mut prop) => {
        transform_shorthand_to_key_values(&mut prop);

        match prop.deref() {
          Prop::KeyValue(key_value) => {
            if let Some(ident) = key_value.key.as_ident() {
              if ident.sym == "syntax" {
                let value = obj.props.iter().find(|prop| {
                  match prop {
                    PropOrSpread::Spread(_) => todo!("Spread in not supported"),
                    PropOrSpread::Prop(prop) => {
                      let mut prop = prop.clone();
                      transform_shorthand_to_key_values(&mut prop);

                      match prop.as_ref() {
                        Prop::KeyValue(key_value) => {
                          if let Some(ident) = key_value.key.as_ident() {
                            return ident.sym == "value";
                          }
                        }
                        _ => todo!(),
                      }
                    }
                  }

                  false
                });
                dbg!(&value);

                if let Some(value) = value {
                  dbg!(&key_value);
                  let result_key_value = value.as_prop().unwrap().clone().key_value().unwrap();

                  // let value = value.value.object().unwrap().props.first().unwrap().clone();

                  // let value = value.as_prop().unwrap().clone().key_value().unwrap();

                  return (result_key_value.value, Option::Some(obj.clone().into()));
                }
              }
            }
          }
          _ => todo!(),
        }
      }
    }
  }

  (key_value.value, Option::None)
}

pub(crate) fn get_key_values_from_object(object: &ObjectLit) -> Vec<KeyValueProp> {
  let mut key_values = vec![];

  for prop in object.props.iter() {
    assert!(prop.is_prop(), "Spread in not supported");

    match prop {
      PropOrSpread::Spread(_) => todo!("Spread in not supported"),
      PropOrSpread::Prop(prop) => {
        let mut prop = prop.clone();

        transform_shorthand_to_key_values(&mut prop);
        dbg!(&prop);

        match prop.as_ref() {
          Prop::KeyValue(key_value) => {
            key_values.push(key_value.clone());
          }
          _ => panic!("{}", constants::messages::ILLEGAL_PROP_VALUE),
        }
      }
    }
  }
  key_values
}

pub(crate) fn dashify(s: &str) -> String {
  let after = DASHIFY_REGEX.replace_all(s, "$1-$2");
  after.to_lowercase()
}

pub(crate) fn fill_top_level_expressions(module: &Module, state: &mut StateManager) {
  module.clone().body.iter().for_each(|item| match &item {
    ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(export_decl)) => {
      if let Decl::Var(decl_var) = &export_decl.decl {
        for decl in &decl_var.decls {
          if let Some(decl_init) = decl.init.as_ref() {
            state.top_level_expressions.push(TopLevelExpression(
              TopLevelExpressionKind::NamedExport,
              *decl_init.clone(),
              Option::Some(decl.name.as_ident().unwrap().to_id()),
            ));
            state.declarations.push(decl.clone());
          }
        }
      }
    }
    ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultExpr(export_decl)) => {
      if let Some(paren) = export_decl.expr.as_paren() {
        state.top_level_expressions.push(TopLevelExpression(
          TopLevelExpressionKind::DefaultExport,
          *paren.expr.clone(),
          None,
        ));
      } else {
        state.top_level_expressions.push(TopLevelExpression(
          TopLevelExpressionKind::DefaultExport,
          *export_decl.expr.clone(),
          None,
        ));
      }
    }
    ModuleItem::Stmt(Stmt::Decl(Decl::Var(var))) => {
      for decl in &var.decls {
        if let Some(decl_init) = decl.init.as_ref() {
          state.top_level_expressions.push(TopLevelExpression(
            TopLevelExpressionKind::Stmt,
            *decl_init.clone(),
            Option::Some(decl.name.as_ident().unwrap().to_id()),
          ));
          state.declarations.push(decl.clone());
        }
      }
    }
    _ => {}
  });
}

pub(crate) fn gen_file_based_identifier(
  file_name: &str,
  export_name: &str,
  key: Option<&str>,
) -> String {
  let key = key.map_or(String::new(), |k| format!(".{}", k));

  format!("{}//{}{}", file_name, export_name, key)
}

pub(crate) fn hash_f32(value: f32) -> u64 {
  let bits = value.to_bits();
  let mut hasher = DefaultHasher::new();
  bits.hash(&mut hasher);
  hasher.finish()
}

pub(crate) fn round_f64(value: f64, decimal_places: u32) -> f64 {
  let multiplier = 10f64.powi(decimal_places as i32);
  (value * multiplier).round() / multiplier
}

pub(crate) fn resolve_node_package_path(package_name: &str) -> Result<PathBuf, String> {
  match node_resolve::Resolver::default()
    .with_basedir(PathBuf::from("./cwd"))
    .preserve_symlinks(true)
    .with_extensions([".ts", ".tsx", ".js", ".jsx", ".json"])
    .with_main_fields(vec![String::from("main"), String::from("module")])
    .resolve(package_name)
  {
    Ok(path) => Ok(path),
    Err(error) => Err(format!(
      "Error resolving package {}: {:?}",
      package_name, error
    )),
  }
}

pub(crate) fn normalize_expr(expr: &Expr) -> &Expr {
  match expr {
    Expr::Paren(paren) => normalize_expr(paren.expr.as_ref()),
    _ => expr,
  }
}

pub(crate) fn transform_shorthand_to_key_values(prop: &mut Box<Prop>) {
  if let Some(ident) = prop.as_shorthand() {
    *prop = Box::new(Prop::KeyValue(KeyValueProp {
      key: PropName::Ident(ident.clone()),
      value: Box::new(Expr::Ident(ident.clone())),
    }));
  }
}