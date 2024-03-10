use std::collections::HashMap;

use indexmap::IndexMap;
use swc_core::ecma::ast::{Expr, Id, VarDeclarator};

use crate::shared::{
    structures::{
        evaluate_result::EvaluateResultValue,
        flat_compiled_styles::{FlatCompiledStyles, FlatCompiledStylesValue},
        functions::FunctionMap,
        injectable_style::InjectableStyle,
        pre_rule::{CompiledResult, PreRule, PreRules},
        stylex_state_options::StyleXStateOptions,
    },
    utils::{
        common::expr_to_str, css::stylex::flatten_raw_style_object::flatten_raw_style_object,
        validators::validate_namespace,
    },
};

pub(crate) fn stylex_create_set(
    namespaces: &EvaluateResultValue,
    prefix: &str,
    declarations: &Vec<VarDeclarator>,
    var_dec_count_map: &mut HashMap<Id, i8>,
    options: &StyleXStateOptions,
    functions: &FunctionMap,
) -> (
    IndexMap<String, FlatCompiledStyles>,
    IndexMap<String, InjectableStyle>,
) {
    let mut resolved_namespaces: IndexMap<String, FlatCompiledStyles> = IndexMap::new();
    let mut injected_styles_map: IndexMap<String, InjectableStyle> = IndexMap::new();

    for (namespace_name, namespace) in namespaces.as_map().unwrap() {
        validate_namespace(&namespace, &vec![]);

        let mut pseudos = vec![];
        let mut at_rules = vec![];

        let flattened_namespace = flatten_raw_style_object(
            namespace,
            declarations,
            var_dec_count_map,
            &mut pseudos,
            &mut at_rules,
            options,
            functions,
        );

        dbg!(&flattened_namespace);

        let compiled_namespace_tuples = flattened_namespace
            .iter()
            .map(|(key, value)| match value {
                PreRules::PreRuleSet(rule_set) => {
                    (key.to_string(), rule_set.clone().compiled(prefix))
                }
                PreRules::StylesPreRule(styles_pre_rule) => {
                    (key.to_string(), styles_pre_rule.clone().compiled(prefix))
                }
                PreRules::NullPreRule(rule_set) => {
                    (key.to_string(), rule_set.clone().compiled(prefix))
                }
                PreRules::PreIncludedStylesRule(pre_included_tyles_rule) => (
                    key.to_string(),
                    pre_included_tyles_rule.clone().compiled(prefix),
                ),
            })
            .collect::<Vec<(String, CompiledResult)>>();

        let compiled_namespace = compiled_namespace_tuples
            .iter()
            .map(|(key, value)| {
                (
                    key.to_string(),
                    match value {
                        CompiledResult::ComputedStyles(styles) => {
                            CompiledResult::ComputedStyles(styles.clone())
                        }
                        CompiledResult::Null => CompiledResult::Null,
                        CompiledResult::IncludedStyle(include_styles) => {
                            CompiledResult::IncludedStyle(include_styles.clone())
                        }
                    },
                )
            })
            .collect::<IndexMap<String, CompiledResult>>();

        dbg!(&compiled_namespace);

        let mut namespace_obj: FlatCompiledStyles = IndexMap::new();

        for key in compiled_namespace.keys() {
            let value = compiled_namespace.get(key).unwrap();

            if let Some(included_styles) = value.as_included_style() {
                namespace_obj.insert(
                    key.clone(),
                    FlatCompiledStylesValue::IncludedStyle(included_styles.clone()),
                );
            } else if let Some(styles) = value.as_computed_styles() {
                let class_name_tuples = styles.clone();

                let class_name = &class_name_tuples
                    .iter()
                    .map(|computed_style| {
                        let class_name = computed_style.0.clone();

                        class_name
                    })
                    .collect::<Vec<String>>()
                    .join(" ");

                namespace_obj.insert(
                    key.clone(),
                    FlatCompiledStylesValue::String(class_name.clone()),
                );

                for item in &class_name_tuples {
                    let class_name = item.0.clone();
                    let injectable_styles = item.1.clone();
                    if !injected_styles_map.contains_key(class_name.as_str()) {
                        injected_styles_map.insert(class_name.clone(), injectable_styles.clone());
                    }
                }
            } else {
                namespace_obj.insert(key.clone(), FlatCompiledStylesValue::Null);
            }
        }

        let resolved_namespace_name =
            expr_to_str(namespace_name, declarations, var_dec_count_map, functions);

        namespace_obj.insert("$$css".to_string(), FlatCompiledStylesValue::Bool(true));

        dbg!(&namespace_obj);

        resolved_namespaces.insert(resolved_namespace_name.clone(), namespace_obj);
    }

    (resolved_namespaces, injected_styles_map)
}