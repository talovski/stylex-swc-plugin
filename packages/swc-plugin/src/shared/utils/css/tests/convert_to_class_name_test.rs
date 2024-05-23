#[cfg(test)]
mod convert_style_to_class_name {
  use crate::shared::structures::{pre_rule::PreRuleValue, state_manager::StateManager};
  use crate::shared::utils::css::utils::convert_style_to_class_name;

  fn convert(styles: (&str, &PreRuleValue)) -> String {
    let result =
      convert_style_to_class_name(styles, &mut [], &mut [], "", &StateManager::default());

    extract_body(result.2.ltr)
  }

  fn extract_body(s: String) -> String {
    let start = s.find('{').unwrap_or(0) + 1;
    let end = s.len() - 1;
    s[start..end].to_string()
  }

  #[test]
  fn converts_style_to_class_name() {
    let result = convert(("margin", &PreRuleValue::String("10".to_string())));

    assert_eq!(result, "margin:10px")
  }

  #[test]
  fn converts_margin_number_to_px() {
    let result = convert(("margin", &PreRuleValue::String("10".to_string())));

    assert_eq!(result, "margin:10px")
  }

  #[test]
  fn keeps_number_for_z_index() {
    let result = convert(("zIndex", &PreRuleValue::String("10".to_string())));

    assert_eq!(result, "z-index:10")
  }

  #[test]
  fn keeps_fr_for_zero_fraction_values() {
    let result = convert(("gridTemplateRows", &PreRuleValue::String("0fr".to_string())));

    assert_eq!(result, "grid-template-rows:0")
  }

  #[test]
  fn keeps_fr_for_zero_percentage_values() {
    let result = convert(("flexBasis", &PreRuleValue::String("0%".to_string())));

    assert_eq!(result, "flex-basis:0%")
  }

  #[test]
  fn keeps_number_for_opacity() {
    let result = convert(("opacity", &PreRuleValue::String("0.25".to_string())));

    assert_eq!(result, "opacity:.25")
  }

  #[test]
  fn handles_array_of_values() {
    let result = convert((
      "height",
      &PreRuleValue::Vec(vec![
        "500".to_string(),
        "100vh".to_string(),
        "100dvh".to_string(),
      ]),
    ));

    assert_eq!(result, "height:500px;height:100vh;height:100dvh")
  }

  #[test]
  fn handles_array_of_values_with_var() {
    let result = convert((
      "height",
      &PreRuleValue::Vec(vec![
        "500".to_string(),
        "var(--height)".to_string(),
        "100dvh".to_string(),
      ]),
    ));

    assert_eq!(result, "height:var(--height,500px);height:100dvh")
  }

  #[test]
  fn handles_array_with_multiple_vars() {
    let result = convert((
      "height",
      &PreRuleValue::Vec(vec![
        "500".to_string(),
        "var(--x)".to_string(),
        "var(--y)".to_string(),
        "100dvh".to_string(),
      ]),
    ));

    assert_eq!(result, "height:var(--y,var(--x,500px));height:100dvh")
  }

  #[test]
  fn handles_array_with_multiple_vars_and_multiple_fallbacks() {
    let result = convert((
      "height",
      &PreRuleValue::Vec(vec![
        "500".to_string(),
        "100vh".to_string(),
        "var(--x)".to_string(),
        "var(--y)".to_string(),
        "100dvh".to_string(),
      ]),
    ));

    assert_eq!(
      result,
      "height:var(--y,var(--x,500px));height:var(--y,var(--x,100vh));height:100dvh"
    )
  }
}
