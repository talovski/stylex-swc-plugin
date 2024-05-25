use core::panic;
use std::collections::HashMap;

use indexmap::IndexMap;
use swc_core::{
  common::{comments::Comments, DUMMY_SP},
  ecma::ast::{CallExpr, Expr, Ident},
};

use crate::shared::structures::{functions::FunctionMap, types::FunctionMapIdentifiers};
use crate::shared::{
  constants::messages::{NON_OBJECT_FOR_STYLEX_CALL, NON_STATIC_VALUE},
  utils::{
    core::js_to_expr::{convert_object_to_ast, NestedStringObject},
    js::evaluate::evaluate,
  },
};
use crate::shared::{
  structures::functions::FunctionConfigType,
  transformers::{stylex_keyframes::get_keyframes_fn, stylex_types::get_types_fn},
};
use crate::shared::{
  structures::types::FunctionMapMemberExpression,
  utils::{
    core::dev_class_name::convert_theme_to_dev_styles,
    validators::{
      is_create_theme_call, validate_stylex_create_theme_indent, validate_theme_variables,
    },
  },
};
use crate::shared::{
  transformers::stylex_create_theme::stylex_create_theme,
  utils::core::dev_class_name::convert_theme_to_test_styles,
};
use crate::ModuleTransformVisitor;

impl<C> ModuleTransformVisitor<C>
where
  C: Comments,
{
  pub(crate) fn transform_stylex_create_theme_call(&mut self, call: &CallExpr) -> Option<Expr> {
    let is_create_theme_call = is_create_theme_call(call, &self.state);

    let result = if is_create_theme_call {
      let (_, parent_var_decl) = &self.get_call_var_name(call);

      validate_stylex_create_theme_indent(parent_var_decl, call, &mut self.state);

      let first_arg = call.args.first();
      let second_arg = call.args.get(1);

      let first_arg = first_arg.map(|first_arg| match &first_arg.spread {
        Some(_) => unimplemented!("Spread"),
        None => first_arg.expr.clone(),
      })?;

      let second_arg = second_arg.map(|second_arg| match &second_arg.spread {
        Some(_) => unimplemented!("Spread"),
        None => second_arg.expr.clone(),
      })?;

      let mut identifiers: FunctionMapIdentifiers = HashMap::new();
      let mut member_expressions: FunctionMapMemberExpression = HashMap::new();

      let keyframes_fn = get_keyframes_fn();
      let types_fn = get_types_fn();

      for name in &self.state.stylex_keyframes_import {
        identifiers.insert(
          name.clone(),
          Box::new(FunctionConfigType::Regular(keyframes_fn.clone())),
        );
      }

      for name in &self.state.stylex_types_import {
        identifiers.insert(
          name.clone(),
          Box::new(FunctionConfigType::Regular(types_fn.clone())),
        );
      }

      for name in &self.state.stylex_import {
        let member_expression = member_expressions.entry(name.clone()).or_default();

        member_expression.insert(
          Box::new(Ident::new("keyframes".into(), DUMMY_SP).to_id()),
          Box::new(FunctionConfigType::Regular(keyframes_fn.clone())),
        );

        let identifier = identifiers
          .entry(Box::new(
            Ident::new(name.get_import_str().into(), DUMMY_SP).to_id(),
          ))
          .or_insert(Box::new(FunctionConfigType::Map(HashMap::default())));

        if let Some(identifier_map) = identifier.as_map_mut() {
          identifier_map.insert(
            Ident::new("types".into(), DUMMY_SP).to_id(),
            types_fn.clone(),
          );
        }
      }

      let function_map: Box<FunctionMap> = Box::new(FunctionMap {
        identifiers,
        member_expressions,
      });

      let evaluated_arg1 = evaluate(&first_arg, &mut self.state, &function_map);

      assert!(evaluated_arg1.confident, "{}", NON_STATIC_VALUE);

      let evaluated_arg2 = evaluate(&second_arg, &mut self.state, &function_map);

      assert!(evaluated_arg2.confident, "{}", NON_STATIC_VALUE);

      let variables = match evaluated_arg1.value {
        Some(value) => {
          validate_theme_variables(&value, &mut self.state);

          value
        }
        None => {
          panic!("Can only override variables theme created with stylex.defineVars().")
        }
      };
      let overrides = match evaluated_arg2.value {
        Some(value) => {
          assert!(
            value
              .as_expr()
              .map(|expr| expr.is_object())
              .unwrap_or(false),
            "{}",
            NON_OBJECT_FOR_STYLEX_CALL
          );
          value
        }
        None => {
          panic!("{}", NON_OBJECT_FOR_STYLEX_CALL)
        }
      };

      let (mut overrides_obj, inject_styles) = stylex_create_theme(
        &variables,
        &overrides,
        &mut self.state,
        &mut IndexMap::default(),
      );

      let (var_name, _) = self.get_call_var_name(call);

      if self.state.is_test() {
        overrides_obj =
          convert_theme_to_test_styles(&var_name, &overrides_obj, &self.state.get_filename());
      } else if self.state.is_dev() {
        overrides_obj =
          convert_theme_to_dev_styles(&var_name, &overrides_obj, &self.state.get_filename());
      }

      let result_ast =
        convert_object_to_ast(&NestedStringObject::FlatCompiledStylesValues(overrides_obj));

      self
        .state
        .register_styles(call, &inject_styles, &result_ast, &var_name);

      return Option::Some(result_ast);
    } else {
      None
    };

    result
  }
}
