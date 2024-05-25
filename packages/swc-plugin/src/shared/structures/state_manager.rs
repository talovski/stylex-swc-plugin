use core::panic;
use std::option::Option;
use std::path::Path;
use std::{
  collections::{HashMap, HashSet},
  path::PathBuf,
};

use indexmap::{IndexMap, IndexSet};
use swc_core::common::{EqIgnoreSpan, FileName, DUMMY_SP};
use swc_core::ecma::ast::{
  BindingIdent, CallExpr, Callee, Decl, Expr, ExprStmt, Id, Ident, ImportDecl,
  ImportDefaultSpecifier, ImportNamedSpecifier, ImportPhase, ImportSpecifier, ModuleDecl,
  ModuleExportName, ModuleItem, Pat, Stmt, Str, VarDecl, VarDeclKind, VarDeclarator,
};

use crate::shared::utils::common::{
  extract_filename_from_path, extract_filename_with_ext_from_path, extract_path, round_f64,
};
use crate::shared::{
  constants::common::DEFAULT_INJECT_PATH,
  utils::ast::factories::{
    expr_or_spread_number_expression_factory, expr_or_spread_string_expression_factory,
  },
};
use crate::shared::{
  enums::data_structures::{
    import_path_resolution::{ImportPathResolution, ImportPathResolutionType},
    style_vars_to_keep::StyleVarsToKeep,
    top_level_expression::{TopLevelExpression, TopLevelExpressionKind},
  },
  utils::common::resolve_file_path,
};

use super::plugin_pass::PluginPass;
use super::stylex_options::{CheckModuleResolution, StyleXOptions};
use super::stylex_state_options::StyleXStateOptions;
use super::uid_generator::UidGenerator;
use super::{injectable_style::InjectableStyle, stylex_options::ModuleResolution};
use super::{meta_data::MetaData, types::StylesObjectMap};
use super::{
  named_import_source::{ImportSources, NamedImportSource, RuntimeInjectionState},
  seen_value::SeenValue,
};

#[derive(Clone, Debug)]
pub struct StateManager {
  pub(crate) _state: Box<PluginPass>,

  // Imports
  pub(crate) import_paths: HashSet<String>,
  pub(crate) stylex_import: HashSet<Box<ImportSources>>,
  pub(crate) stylex_props_import: HashSet<Box<Id>>,
  pub(crate) stylex_attrs_import: HashSet<Box<Id>>,
  pub(crate) stylex_create_import: HashSet<Box<Id>>,
  pub(crate) stylex_include_import: HashSet<Box<Id>>,
  pub(crate) stylex_first_that_works_import: HashSet<Box<Id>>,
  pub(crate) stylex_keyframes_import: HashSet<Box<Id>>,
  pub(crate) stylex_define_vars_import: HashSet<Box<Id>>,
  pub(crate) stylex_create_theme_import: HashSet<Box<Id>>,
  pub(crate) stylex_types_import: HashSet<Box<Id>>,
  pub(crate) inject_import_inserted: Option<(Box<Ident>, Box<Ident>)>,
  pub(crate) theme_name: Option<String>,

  pub(crate) declarations: Vec<VarDeclarator>,
  pub(crate) top_level_expressions: Vec<TopLevelExpression>,
  pub(crate) all_call_expressions: Vec<CallExpr>,
  pub(crate) var_decl_count_map: HashMap<Id, i8>,
  pub(crate) seen: HashMap<Box<Expr>, Box<SeenValue>>,

  // `stylex.create` calls
  pub(crate) style_map: HashMap<String, Box<StylesObjectMap>>,
  pub(crate) style_vars: HashMap<String, Box<VarDeclarator>>,

  // results of `stylex.create` calls that should be kept
  pub(crate) style_vars_to_keep: HashSet<Box<StyleVarsToKeep>>,
  pub(crate) member_object_ident_count_map: HashMap<Id, i8>,

  pub(crate) in_stylex_create: bool,

  pub(crate) styles_vars_to_inject: Vec<String>,

  pub(crate) options: Box<StyleXStateOptions>,
  pub(crate) metadata: IndexMap<String, Vec<MetaData>>,
  pub(crate) styles_to_inject: IndexMap<Box<Expr>, Vec<ModuleItem>>,
  pub(crate) prepend_include_module_items: Vec<ModuleItem>,
  pub(crate) prepend_import_module_items: Vec<ModuleItem>,

  pub(crate) injected_keyframes: IndexMap<String, Box<InjectableStyle>>,
  pub(crate) top_imports: Vec<ImportDecl>,
}

impl Default for StateManager {
  fn default() -> Self {
    StateManager::new(StyleXOptions::default())
  }
}

impl StateManager {
  pub fn new(stylex_options: StyleXOptions) -> Self {
    let options = Box::new(StyleXStateOptions::from(stylex_options));

    Self {
      _state: Box::<PluginPass>::default(),
      import_paths: HashSet::new(),
      stylex_import: HashSet::new(),
      stylex_props_import: HashSet::new(),
      stylex_attrs_import: HashSet::new(),
      stylex_create_import: HashSet::new(),
      stylex_include_import: HashSet::new(),
      stylex_first_that_works_import: HashSet::new(),
      stylex_keyframes_import: HashSet::new(),
      stylex_define_vars_import: HashSet::new(),
      stylex_create_theme_import: HashSet::new(),
      stylex_types_import: HashSet::new(),
      inject_import_inserted: None,
      style_map: HashMap::new(),
      style_vars: HashMap::new(),
      style_vars_to_keep: HashSet::new(),
      member_object_ident_count_map: HashMap::new(),
      theme_name: Option::None,

      seen: HashMap::new(),

      top_imports: vec![],

      declarations: vec![],
      top_level_expressions: vec![],
      all_call_expressions: vec![],
      var_decl_count_map: HashMap::new(),

      in_stylex_create: false,
      options,

      styles_vars_to_inject: vec![],
      metadata: IndexMap::new(),
      styles_to_inject: IndexMap::new(),
      prepend_include_module_items: vec![],
      prepend_import_module_items: vec![],

      injected_keyframes: IndexMap::new(),
    }
  }

  pub fn import_as(&self, import: &str) -> Option<String> {
    for import_source in &self.options.import_sources {
      match import_source {
        ImportSources::Regular(_) => {}
        ImportSources::Named(named) => {
          if named.from.eq(import) {
            return Option::Some(named.r#as.clone());
          }
        }
      }
    }

    Option::None
  }

  pub fn import_sources(&self) -> Vec<ImportSources> {
    self.options.import_sources.clone()
  }

  pub fn import_sources_stringified(&self) -> Vec<String> {
    self
      .options
      .import_sources
      .clone()
      .into_iter()
      .map(|import_source| match import_source {
        ImportSources::Regular(regular) => regular,
        ImportSources::Named(named) => named.from,
      })
      .collect()
  }

  pub fn stylex_import_stringified(&self) -> Vec<String> {
    self
      .stylex_import
      .clone()
      .into_iter()
      .map(|import_source| match import_source.as_ref() {
        ImportSources::Regular(regular) => regular.clone(),
        ImportSources::Named(named) => named.clone().r#as,
      })
      .collect()
  }

  pub(crate) fn is_test(&self) -> bool {
    self.options.test
  }

  pub(crate) fn is_dev(&self) -> bool {
    self.options.dev
  }

  pub(crate) fn gen_conditional_classes(&self) -> bool {
    self.options.gen_conditional_classes
  }

  pub(crate) fn get_short_filename(&self) -> String {
    extract_filename_from_path(self._state.filename.clone())
  }
  pub(crate) fn get_filename(&self) -> String {
    extract_path(self._state.filename.clone())
  }
  pub(crate) fn get_filename_for_hashing(&self) -> Option<String> {
    let filename = self.get_filename();

    let unstable_module_resolution = self
      .options
      .unstable_module_resolution
      .clone()
      .unwrap_or_default();

    let theme_file_extension = match unstable_module_resolution.clone() {
      CheckModuleResolution::CommonJS(ModuleResolution {
        theme_file_extension,
        ..
      }) => theme_file_extension,
      CheckModuleResolution::Haste(ModuleResolution {
        theme_file_extension,
        ..
      }) => theme_file_extension,
      CheckModuleResolution::CrossFileParsing(ModuleResolution {
        theme_file_extension,
        ..
      }) => theme_file_extension,
    }
    .unwrap_or(".stylex".to_string());

    if filename.is_empty()
      || !matches_file_suffix(theme_file_extension.as_str(), &filename)
      || self.options.unstable_module_resolution.is_none()
    {
      return Option::None;
    }

    match unstable_module_resolution {
      CheckModuleResolution::Haste(_) => {
        let filename = FileName::Real(filename.into());
        extract_filename_with_ext_from_path(filename)
      }
      CheckModuleResolution::CommonJS(module_resolution)
      | CheckModuleResolution::CrossFileParsing(module_resolution) => {
        let root_dir = module_resolution
          .root_dir
          .expect("root_dir is required for CommonJS");

        let root_dir = Path::new(root_dir.as_str());

        let filename = Path::new(&filename);

        let filename_for_hashing = relative_path(root_dir, filename).display().to_string();

        Option::Some(filename_for_hashing)
      }
    }
  }

  pub(crate) fn import_path_resolver(&self, import_path: &str) -> ImportPathResolution {
    let source_file_path = self.get_filename();

    if source_file_path.is_empty() {
      return ImportPathResolution::False;
    }

    let Some(unstable_module_resolution) = &self.options.unstable_module_resolution else {
      return ImportPathResolution::False;
    };

    match unstable_module_resolution {
      CheckModuleResolution::CommonJS(module_resolution) => {
        let root_dir = module_resolution
          .clone()
          .root_dir
          .expect("root_dir is required for CommonJS");

        let root_dir_path = Path::new(root_dir.as_str());

        let theme_file_extension = module_resolution
          .theme_file_extension
          .clone()
          .unwrap_or(".stylex".to_string());

        if !matches_file_suffix(theme_file_extension.as_str(), import_path) {
          return ImportPathResolution::False;
        }

        let resolved_file_path =
          file_path_resolver(import_path, source_file_path, root_dir.as_str());

        ImportPathResolution::Tuple(
          ImportPathResolutionType::ThemeNameRef,
          relative_path(
            Path::new(root_dir_path),
            Path::new(resolved_file_path.as_str()),
          )
          .display()
          .to_string(),
        )
      }
      CheckModuleResolution::Haste(module_resolution) => {
        let theme_file_extension = module_resolution
          .theme_file_extension
          .clone()
          .unwrap_or(".stylex".to_string());

        if !matches_file_suffix(theme_file_extension.as_str(), import_path) {
          return ImportPathResolution::False;
        }

        ImportPathResolution::Tuple(
          ImportPathResolutionType::ThemeNameRef,
          add_file_extension(import_path, &source_file_path),
        )
      }
      _ => unimplemented!("Module resolution is not supported"),
    }
  }

  pub(crate) fn get_top_level_expr(
    &self,
    kind: &TopLevelExpressionKind,
    call: &CallExpr,
  ) -> Option<TopLevelExpression> {
    self
      .top_level_expressions
      .clone()
      .into_iter()
      .find(|tpe| kind.eq(&tpe.0) && tpe.1.eq(&Box::new(Expr::Call(call.clone()))))
  }

  pub(crate) fn register_styles(
    &mut self,
    call: &CallExpr,
    style: &IndexMap<String, Box<InjectableStyle>>,
    ast: &Expr,
    var_name: &Option<String>,
  ) {
    if style.is_empty() {
      return;
    }

    let metadatas = MetaData::convert_from_injected_styles_map(style.clone());

    let uid_generator_inject = UidGenerator::new("inject");

    let runtime_injection = self
      .options
      .runtime_injection
      .as_ref()
      .unwrap_or(&RuntimeInjectionState::Regular(String::default()))
      .clone();

    let (inject_module_ident, inject_var_ident) = match self.inject_import_inserted.as_ref() {
      Some(idents) => idents.clone(),
      None => {
        let inject_module_ident = Box::new(uid_generator_inject.generate_ident());

        let inject_var_ident = Box::new(match runtime_injection.clone() {
          RuntimeInjectionState::Regular(_) => uid_generator_inject.generate_ident(),
          RuntimeInjectionState::Named(NamedImportSource { r#as, .. }) => {
            let uid_generator_inject = UidGenerator::new(&r#as);

            uid_generator_inject.generate_ident()
          }
        });

        self.inject_import_inserted =
          Option::Some((inject_module_ident.clone(), inject_var_ident.clone()));

        (inject_module_ident, inject_var_ident)
      }
    };

    if !metadatas.is_empty() && self.prepend_include_module_items.is_empty() {
      let first_module_items = match runtime_injection {
        RuntimeInjectionState::Regular(_) => vec![
          add_inject_default_import_expression(&inject_module_ident),
          add_inject_var_decl_expression(&inject_var_ident, &inject_module_ident),
        ],
        RuntimeInjectionState::Named(_) => {
          vec![
            add_inject_named_import_expression(&inject_module_ident, &inject_var_ident),
            add_inject_var_decl_expression(&inject_var_ident, &inject_module_ident),
          ]
        }
      };

      self.prepend_include_module_items.extend(first_module_items);
    }

    for metadata in metadatas {
      self.add_style(
        var_name.clone().unwrap_or("default".to_string()),
        metadata.clone(),
      );

      self.add_style_to_inject(&metadata, &inject_var_ident, ast);
    }

    if self.options.runtime_injection.is_none() {
      return;
    }

    if let Some(item) = self.declarations.iter_mut().find(|decl| {
      decl
        .init
        .as_ref()
        .unwrap()
        .eq(&Box::new(Expr::Call(call.clone())))
    }) {
      item.init = Option::Some(Box::new(ast.clone()));

      let var_id = item.name.as_ident().unwrap().sym.to_string();

      if !self.styles_vars_to_inject.contains(&var_id) {
        self.styles_vars_to_inject.push(var_id);
      }
    };

    if let Some((_, item)) = self.style_vars.iter_mut().find(|(_, decl)| {
      decl
        .init
        .as_ref()
        .unwrap()
        .eq(&Box::new(Expr::Call(call.clone())))
    }) {
      item.init = Option::Some(Box::new(ast.clone()));
    };

    if let Some(TopLevelExpression(_, item, _)) = self
      .top_level_expressions
      .iter_mut()
      .find(|TopLevelExpression(_, decl, _)| decl.eq(&Expr::Call(call.clone())))
    {
      *item = ast.clone();
    };

    if let Some(index) = self
      .all_call_expressions
      .iter()
      .position(|expr| expr.eq_ignore_span(&Box::new(call.clone())))
    {
      if let Some(call_expr) = ast.as_call() {
        self.all_call_expressions[index] = call_expr.clone();
      } else {
        self.all_call_expressions.remove(index);
      }
    }
  }

  fn add_style(&mut self, var_name: String, metadata: MetaData) {
    let value = self.metadata.entry(var_name).or_default();

    if !value
      .iter()
      .any(|item| item.get_class_name() == metadata.get_class_name())
    {
      value.push(metadata);
    }
  }

  fn add_style_to_inject(&mut self, metadata: &MetaData, inject_var_ident: &Ident, ast: &Expr) {
    let priority = &metadata.get_priority();

    let css = &metadata.get_css();
    let css_rtl = &metadata.get_css_rtl();

    let mut stylex_inject_args = vec![
      expr_or_spread_string_expression_factory(css.as_str()),
      expr_or_spread_number_expression_factory(round_f64(**priority, 1)),
    ];

    if let Some(rtl) = css_rtl {
      stylex_inject_args.push(expr_or_spread_string_expression_factory(rtl.as_str()));
    }

    let _inject = Expr::Ident(inject_var_ident.clone());

    let stylex_call_expr = CallExpr {
      span: DUMMY_SP,
      type_args: Option::None,
      callee: Callee::Expr(Box::new(_inject.clone())),
      args: stylex_inject_args,
    };

    let stylex_call = Expr::Call(stylex_call_expr);

    let module = ModuleItem::Stmt(Stmt::Expr(ExprStmt {
      span: DUMMY_SP,
      expr: Box::new(stylex_call),
    }));

    self
      .styles_to_inject
      .entry(Box::new(ast.clone()))
      .or_default()
      .push(module);
  }

  pub(crate) fn _get_css_vars(&self) -> HashMap<String, String> {
    self.options.defined_stylex_css_variables.clone()
  }

  pub(crate) fn get_treeshake_compensation(&self) -> bool {
    self.options.treeshake_compensation.unwrap_or(false)
  }

  pub fn combine(self, other: Self) -> Self {
    // Now you can use these helper functions to simplify your function
    StateManager {
      _state: self._state,
      import_paths: union_hash_set(&self.import_paths, &other.import_paths),
      stylex_import: union_hash_set(&self.stylex_import, &other.stylex_import),
      stylex_props_import: union_hash_set(&self.stylex_props_import, &other.stylex_props_import),
      stylex_attrs_import: union_hash_set(&self.stylex_attrs_import, &other.stylex_attrs_import),
      stylex_create_import: union_hash_set(&self.stylex_create_import, &other.stylex_create_import),
      stylex_include_import: union_hash_set(
        &self.stylex_include_import,
        &other.stylex_include_import,
      ),
      stylex_first_that_works_import: union_hash_set(
        &self.stylex_first_that_works_import,
        &other.stylex_first_that_works_import,
      ),
      stylex_keyframes_import: union_hash_set(
        &self.stylex_keyframes_import,
        &other.stylex_keyframes_import,
      ),
      stylex_define_vars_import: union_hash_set(
        &self.stylex_define_vars_import,
        &other.stylex_define_vars_import,
      ),
      stylex_create_theme_import: union_hash_set(
        &self.stylex_create_theme_import,
        &other.stylex_create_theme_import,
      ),
      stylex_types_import: union_hash_set(&self.stylex_types_import, &other.stylex_types_import),
      inject_import_inserted: self.inject_import_inserted.or(other.inject_import_inserted),
      theme_name: self.theme_name.or(other.theme_name),
      declarations: chain_collect(self.declarations, other.declarations),
      top_level_expressions: chain_collect(self.top_level_expressions, other.top_level_expressions),
      all_call_expressions: chain_collect(self.all_call_expressions, other.all_call_expressions),
      var_decl_count_map: chain_collect_hash_map(self.var_decl_count_map, other.var_decl_count_map),
      style_map: chain_collect_hash_map(self.style_map, other.style_map),
      style_vars: chain_collect_hash_map(self.style_vars, other.style_vars),
      style_vars_to_keep: union_hash_set(&self.style_vars_to_keep, &other.style_vars_to_keep),
      member_object_ident_count_map: chain_collect_hash_map(
        self.member_object_ident_count_map,
        other.member_object_ident_count_map,
      ),
      in_stylex_create: self.in_stylex_create || other.in_stylex_create,
      styles_vars_to_inject: chain_collect(self.styles_vars_to_inject, other.styles_vars_to_inject),
      options: self.options,
      metadata: chain_collect_index_map(self.metadata, other.metadata),
      seen: chain_collect_hash_map(self.seen, other.seen),
      styles_to_inject: chain_collect_index_map(self.styles_to_inject, other.styles_to_inject),
      prepend_include_module_items: chain_collect(
        self.prepend_include_module_items,
        other.prepend_include_module_items,
      ),
      prepend_import_module_items: chain_collect(
        self.prepend_import_module_items,
        other.prepend_import_module_items,
      ),
      injected_keyframes: chain_collect_index_map(
        self.injected_keyframes,
        other.injected_keyframes,
      ),
      top_imports: chain_collect(self.top_imports, other.top_imports),
    }
  }
}

fn add_inject_default_import_expression(ident: &Ident) -> ModuleItem {
  ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
    span: DUMMY_SP,
    specifiers: vec![ImportSpecifier::Default(ImportDefaultSpecifier {
      span: DUMMY_SP,
      local: ident.clone(),
    })],
    src: Box::new(Str {
      span: DUMMY_SP,
      raw: Option::None,
      value: DEFAULT_INJECT_PATH.into(),
    }),
    type_only: false,
    with: Option::None,
    phase: ImportPhase::Evaluation,
  }))
}

pub(crate) fn add_import_expression(path: &str) -> ModuleItem {
  ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
    span: DUMMY_SP,
    specifiers: vec![],
    src: Box::new(Str {
      span: DUMMY_SP,
      raw: Option::None,
      value: path.into(),
    }),
    type_only: false,
    with: Option::None,
    phase: ImportPhase::Evaluation,
  }))
}

fn add_inject_named_import_expression(ident: &Ident, imported_ident: &Ident) -> ModuleItem {
  ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
    span: DUMMY_SP,
    specifiers: vec![ImportSpecifier::Named(ImportNamedSpecifier {
      span: DUMMY_SP,
      local: ident.clone(),
      imported: Option::Some(ModuleExportName::Ident(imported_ident.clone())),
      is_type_only: false,
    })],
    src: Box::new(Str {
      span: DUMMY_SP,
      raw: Option::None,
      value: DEFAULT_INJECT_PATH.into(),
    }),
    type_only: false,
    with: Option::None,
    phase: ImportPhase::Evaluation,
  }))
}

fn add_inject_var_decl_expression(decl_ident: &Ident, value_ident: &Ident) -> ModuleItem {
  ModuleItem::Stmt(Stmt::Decl(Decl::Var(Box::new(VarDecl {
    declare: false,
    decls: vec![VarDeclarator {
      definite: true,
      span: DUMMY_SP,
      name: Pat::Ident(BindingIdent {
        id: decl_ident.clone(),
        type_ann: None,
      }),
      init: Option::Some(Box::new(Expr::Ident(value_ident.clone()))),
    }],
    kind: VarDeclKind::Var,
    span: DUMMY_SP,
  }))))
}

pub(crate) fn matches_file_suffix(allowed_suffix: &str, filename: &str) -> bool {
  filename.ends_with(&format!("{}.js", allowed_suffix))
    || filename.ends_with(&format!("{}.ts", allowed_suffix))
    || filename.ends_with(&format!("{}.tsx", allowed_suffix))
    || filename.ends_with(&format!("{}.jsx", allowed_suffix))
    || filename.ends_with(&format!("{}.mdx", allowed_suffix))
    || filename.ends_with(&format!("{}.md", allowed_suffix))
    || filename.ends_with(&format!("{}.mjs", allowed_suffix))
    || filename.ends_with(&format!("{}.cjs", allowed_suffix))
    || filename.ends_with(allowed_suffix)
}

pub(crate) const EXTENSIONS: [&str; 8] =
  [".tsx", ".ts", ".jsx", ".js", ".mjs", ".cjs", ".mdx", ".md"];

fn add_file_extension(imported_file_path: &str, source_file: &str) -> String {
  if EXTENSIONS
    .iter()
    .any(|ext| imported_file_path.ends_with(ext))
  {
    return imported_file_path.to_string();
  }

  let file_extension = Path::new(source_file)
    .extension()
    .and_then(std::ffi::OsStr::to_str)
    .unwrap_or("");

  format!("{}.{}", imported_file_path, file_extension)
}

fn chain_collect<T: Clone>(vec1: Vec<T>, vec2: Vec<T>) -> Vec<T> {
  vec1.into_iter().chain(vec2).collect()
}

fn union_hash_set<T: Clone + Eq + std::hash::Hash>(
  set1: &HashSet<T>,
  set2: &HashSet<T>,
) -> HashSet<T> {
  set1.union(set2).cloned().collect()
}

fn chain_collect_hash_map<K: Eq + std::hash::Hash, V: Clone>(
  map1: HashMap<K, V>,
  map2: HashMap<K, V>,
) -> HashMap<K, V> {
  map1.into_iter().chain(map2).collect()
}

fn _union_index_set<T: Clone + Eq + std::hash::Hash>(
  set1: &IndexSet<T>,
  set2: &IndexSet<T>,
) -> IndexSet<T> {
  set1.union(set2).cloned().collect()
}

fn chain_collect_index_map<K: Eq + std::hash::Hash, V: Clone>(
  map1: IndexMap<K, V>,
  map2: IndexMap<K, V>,
) -> IndexMap<K, V> {
  map1.into_iter().chain(map2).collect()
}

fn file_path_resolver(
  relative_file_path: &str,
  source_file_path: String,
  root_path: &str,
) -> String {
  let file_to_look_for = relative_file_path;

  if EXTENSIONS.iter().any(|ext| file_to_look_for.ends_with(ext)) {
    unimplemented!("file_path_resolver")
  }

  for ext in EXTENSIONS.iter() {
    let import_path_str = if file_to_look_for.starts_with('.') {
      format!("{}{}", file_to_look_for, ext)
    } else {
      file_to_look_for.to_string()
    };

    let resolved_file_path = resolve_file_path(&import_path_str, &source_file_path, ext, root_path);

    if let Ok(resolved_path) = resolved_file_path {
      return resolved_path
        .display()
        .to_string()
        .replace("/app/@", "/node_modules1/@");
    }
  }

  panic!("Cannot resolve file path: {}", relative_file_path)
}

fn relative_path(root: &Path, file_path: &Path) -> PathBuf {
  let rel_path = file_path
    .strip_prefix(root)
    .ok()
    .map_or(file_path.to_path_buf(), PathBuf::from);

  rel_path
}
