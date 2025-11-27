use crate::analyzer::{Span, parser};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tree_sitter::Node;

/// Stores parsed PHP sources and derived symbol data for the whole workspace.
pub struct ProjectContext {
    sources: HashMap<PathBuf, parser::ParsedSource>,
    file_scopes: HashMap<PathBuf, FileScope>,
    function_symbols: HashMap<String, Vec<FunctionSymbol>>,
}

pub(crate) struct FileMetadata {
    pub namespace: Option<String>,
    pub uses: HashMap<String, UseInfo>,
    pub symbols: Vec<FunctionSymbol>,
}

/// Namespace and symbol information for a single file.
#[allow(dead_code)]
pub struct FileScope {
    pub namespace: Option<String>,
    pub functions: Vec<FunctionSymbol>,
    pub uses: HashMap<String, UseInfo>,
}

#[derive(Clone)]
pub struct UseInfo {
    pub target: String,
    pub span: Span,
    pub clause_start: usize,
    pub clause_end: usize,
    pub declaration_has_multiple_clauses: bool,
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct FunctionSymbol {
    pub name: String,
    pub fq_name: String,
    pub file: PathBuf,
    pub span: Span,
    pub required_params: usize,
}

impl ProjectContext {
    pub fn new() -> Self {
        Self {
            sources: HashMap::new(),
            file_scopes: HashMap::new(),
            function_symbols: HashMap::new(),
        }
    }

    pub fn insert(&mut self, parsed: parser::ParsedSource) {
        let metadata = collect_file_metadata(&parsed);
        self.insert_with_metadata(parsed, metadata);
    }

    pub(crate) fn insert_with_metadata(
        &mut self,
        parsed: parser::ParsedSource,
        metadata: FileMetadata,
    ) {
        let path = parsed.path.clone();
        let FileMetadata {
            namespace,
            uses,
            symbols,
        } = metadata;

        for symbol in &symbols {
            self.function_symbols
                .entry(symbol.fq_name.clone())
                .or_default()
                .push(symbol.clone());
        }

        self.file_scopes.insert(
            path.clone(),
            FileScope {
                namespace,
                functions: symbols.clone(),
                uses,
            },
        );

        self.sources.insert(path, parsed);
    }

    pub fn get(&self, path: &Path) -> Option<&parser::ParsedSource> {
        self.sources.get(path)
    }

    pub fn iter(&self) -> impl Iterator<Item = &parser::ParsedSource> {
        self.sources.values()
    }

    pub fn len(&self) -> usize {
        self.sources.len()
    }

    pub fn scope_for(&self, path: &Path) -> Option<&FileScope> {
        self.file_scopes.get(path)
    }

    pub fn resolve_function_symbol<'a>(
        &'a self,
        name: &str,
        parsed: &parser::ParsedSource,
    ) -> Option<&'a FunctionSymbol> {
        let scope = self.scope_for(&parsed.path)?;
        for candidate in candidate_function_names(name, scope) {
            if let Some(symbols) = self.function_symbols.get(&candidate) {
                if let Some(symbol) = symbols.first() {
                    return Some(symbol);
                }
            }
        }
        None
    }

    #[allow(dead_code)]
    pub fn function_symbols(&self) -> &HashMap<String, Vec<FunctionSymbol>> {
        &self.function_symbols
    }
}

fn collect_namespace(parsed: &parser::ParsedSource) -> Option<String> {
    let mut namespace = None;

    walk_node(parsed.tree.root_node(), &mut |node| {
        if namespace.is_some() {
            return;
        }

        if node.kind() == "namespace_definition" {
            if let Some(name_node) = child_by_kind(node, "namespace_name") {
                if let Some(name) = node_text(name_node, parsed) {
                    namespace = Some(name);
                }
            }
        }
    });

    namespace
}

fn collect_use_aliases(parsed: &parser::ParsedSource) -> HashMap<String, UseInfo> {
    let mut uses = HashMap::new();

    walk_node(parsed.tree.root_node(), &mut |node| {
        if node.kind() != "namespace_use_declaration" {
            return;
        }

        let clause_count = (0..node.named_child_count())
            .filter_map(|idx| node.named_child(idx))
            .filter(|child| child.kind() == "namespace_use_clause")
            .count();
        let declaration_has_multiple_clauses = clause_count > 1;

        for idx in 0..node.named_child_count() {
            if let Some(child) = node.named_child(idx) {
                if child.kind() != "namespace_use_clause" {
                    continue;
                }

                if let Some(qualified) = child_by_kind(child, "qualified_name") {
                    if let Some(fq_name) = node_text(qualified, parsed) {
                        if let Some(alias_node) = alias_node_from_clause(child) {
                            if let Some(alias) = node_text(alias_node, parsed) {
                                uses.insert(
                                    alias.clone(),
                                    UseInfo {
                                        target: fq_name,
                                        span: span_from_node(alias_node),
                                        clause_start: child.start_byte(),
                                        clause_end: child.end_byte(),
                                        declaration_has_multiple_clauses,
                                    },
                                );
                            }
                        }
                    }
                }
            }
        }
    });

    uses
}

fn alias_node_from_clause<'a>(clause: Node<'a>) -> Option<Node<'a>> {
    if let Some(alias_clause) = child_by_kind(clause, "namespace_aliasing_clause") {
        if let Some(alias_name) = child_by_kind(alias_clause, "name") {
            return Some(alias_name);
        }
    }

    if let Some(qualified) = child_by_kind(clause, "qualified_name") {
        return last_name_in_node(qualified);
    }

    None
}

fn last_name_in_node<'a>(node: Node<'a>) -> Option<Node<'a>> {
    let mut last = None;
    for idx in 0..node.named_child_count() {
        if let Some(child) = node.named_child(idx) {
            if child.kind() == "name" {
                last = Some(child);
            }
        }
    }
    last
}

fn span_from_node(node: Node) -> Span {
    Span {
        start: node.start_position(),
        end: node.end_position(),
    }
}

fn collect_function_symbols(
    parsed: &parser::ParsedSource,
    namespace: Option<&str>,
) -> Vec<FunctionSymbol> {
    let mut symbols = Vec::new();

    walk_node(parsed.tree.root_node(), &mut |node| {
        if node.kind() != "function_definition" {
            return;
        }

        if let Some(name_node) = child_by_kind(node, "name") {
            if let Some(name) = node_text(name_node, parsed) {
                let fq = qualify_name(namespace, &name);
                symbols.push(FunctionSymbol {
                    name,
                    fq_name: fq,
                    file: parsed.path.clone(),
                    span: Span {
                        start: node.start_position(),
                        end: node.end_position(),
                    },
                    required_params: child_by_kind(node, "formal_parameters")
                        .map(count_required_parameters)
                        .unwrap_or(0),
                });
            }
        }
    });

    symbols
}

pub(crate) fn collect_file_metadata(parsed: &parser::ParsedSource) -> FileMetadata {
    let namespace = collect_namespace(parsed);
    let uses = collect_use_aliases(parsed);
    let symbols = collect_function_symbols(parsed, namespace.as_deref());

    FileMetadata {
        namespace,
        uses,
        symbols,
    }
}

fn qualify_name(namespace: Option<&str>, name: &str) -> String {
    match namespace {
        Some(ns) => format!("{ns}\\{name}"),
        None => name.to_owned(),
    }
}

fn count_required_parameters<'a>(formal: Node<'a>) -> usize {
    (0..formal.named_child_count())
        .filter_map(|idx| formal.named_child(idx))
        .filter(|param| param.kind() == "simple_parameter")
        .filter(|param| !parameter_has_default(*param))
        .count()
}

fn parameter_has_default<'a>(param: Node<'a>) -> bool {
    for idx in 0..param.named_child_count() {
        if let Some(child) = param.named_child(idx) {
            if child.kind() == "default_value" {
                return true;
            }
        }
    }

    false
}

fn candidate_function_names(name: &str, scope: &FileScope) -> Vec<String> {
    let mut candidates = Vec::new();
    let normalized = name.trim_start_matches('\\');
    let segments: Vec<&str> = normalized.split('\\').collect();
    let first = segments.get(0).copied().unwrap_or("");
    let remainder = if segments.len() > 1 {
        segments[1..].join("\\")
    } else {
        String::new()
    };

    if name.starts_with('\\') {
        candidates.push(normalized.to_owned());
    } else {
        if let Some(use_info) = scope.uses.get(first) {
            if remainder.is_empty() {
                candidates.push(use_info.target.clone());
            } else {
                candidates.push(format!("{}\\{}", use_info.target, remainder));
            }
        }

        if let Some(ns) = &scope.namespace {
            candidates.push(format!("{ns}\\{normalized}"));
        }
    }

    candidates.push(normalized.to_owned());
    candidates
}

fn walk_node<'a, F>(node: Node<'a>, callback: &mut F)
where
    F: FnMut(Node<'a>),
{
    callback(node);
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            walk_node(cursor.node(), callback);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

fn child_by_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    for idx in 0..node.named_child_count() {
        if let Some(child) = node.named_child(idx) {
            if child.kind() == kind {
                return Some(child);
            }
        }
    }

    None
}

fn node_text<'a>(node: Node<'a>, parsed: &parser::ParsedSource) -> Option<String> {
    node.utf8_text(parsed.source.as_bytes())
        .ok()
        .map(|text| text.trim().to_owned())
}
