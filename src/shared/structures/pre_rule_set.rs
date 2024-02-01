use std::{any::Any, fmt::Debug, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::shared::{
    structures::pre_rule::{CompiledResult, ComputedStyle},
    utils::common::type_of,
};

use super::{
    injectable_style::InjectableStyle,
    null_pre_rule::NullPreRule,
    pre_rule::{PreRule, PreRules, Styles},
};
#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct PreRuleSet {
    rules: Vec<PreRules>,
}

impl PreRuleSet {
    pub(crate) fn new() -> Self {
        PreRuleSet { rules: vec![] }
    }
    pub(crate) fn create(rules: Vec<PreRules>) -> PreRules {
        let flat_rules = rules
            .into_iter()
            .flat_map(|rule| match rule {
                PreRules::PreRuleSet(rule_set) => rule_set.rules,
                _ => vec![rule],
            })
            .collect::<Vec<PreRules>>();

        match flat_rules.len() {
            0 => PreRules::NullPreRule(NullPreRule::new()),
            1 => flat_rules.get(0).unwrap().clone(),
            _ => PreRules::PreRuleSet(PreRuleSet { rules: flat_rules }),
        }
    }
}

impl PreRule for PreRuleSet {
    fn equals(&self, other: &dyn PreRule) -> bool {
        true
    }
    fn compiled(&mut self, prefix: &str) -> CompiledResult {
        let style_tuple = self
            .rules
            .iter()
            .flat_map(|rule| {
                let compiled_rule = match rule {
                    PreRules::PreRuleSet(rule_set) => rule_set.clone().compiled(prefix),
                    PreRules::StylesPreRule(styles_pre_rule) => {
                        styles_pre_rule.clone().compiled(prefix)
                    }
                    PreRules::NullPreRule(null_pre_rule) => null_pre_rule.clone().compiled(prefix),
                };

                println!(
                    "!!!!__ rule: {:#?}, compiled_rule: {:#?}",
                    rule, compiled_rule
                );

                match compiled_rule {
                    CompiledResult::ComputedStyles(styles) => styles,
                    _ => vec![],
                }
            })
            // .filter(|style| style.is_some())
            .collect::<Vec<ComputedStyle>>();

        println!("!!!!__ style_tuple: {:#?}", style_tuple);

        // if style_tuple.is_empty() {
        //     vec![]
        // } else {
        //     style_tuple
        // }

        CompiledResult::ComputedStyles(style_tuple)
    }
    fn get_value(&self) -> Option<String> {
        let rule = self.rules.get(0).unwrap();

        match &rule {
            PreRules::PreRuleSet(rule_set) => rule_set.get_value(),
            PreRules::StylesPreRule(styles_pre_rule) => styles_pre_rule.get_value(),
            PreRules::NullPreRule(null_pre_rule) => null_pre_rule.get_value(),
        }
        // self.rules
        //     .iter()
        //     .map(|rule| {
        //         let value = match &rule {
        //             PreRules::PreRuleSet(rule_set) => rule_set.get_value(),
        //             PreRules::StylesPreRule(styles_pre_rule) => styles_pre_rule.get_value(),
        //             PreRules::NullPreRule(null_pre_rule) => null_pre_rule.get_value(),
        //         };

        //         value.unwrap()
        //     })
        //     .collect::<Vec<_>>()
        //     .join(" ")
        //     .into()
    }
}