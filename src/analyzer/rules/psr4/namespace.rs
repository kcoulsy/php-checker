use crate::analyzer::project::ProjectContext;
use crate::analyzer::{
    Diagnostic, Severity,
    config::{AnalyzerConfig, StrictnessLevel},
};
use std::path::{Path, PathBuf};

const RULE_NAME: &str = "psr4/namespace";

pub fn run_namespace_checks(
    root: &Path,
    context: &ProjectContext,
    config: &AnalyzerConfig,
) -> Vec<Diagnostic> {
    if !config.psr4.enabled || !config.enabled("psr4") || !config.enabled("psr4/namespace") {
        return Vec::new();
    }

    let namespace_root = resolve_namespace_root(root, &config.psr4.namespace_root);
    let mut diagnostics = Vec::new();

    for parsed in context.iter() {
        let relative = match parsed.path.strip_prefix(&namespace_root) {
            Ok(relative) => relative,
            Err(_) => continue,
        };

        let expected_namespace = namespace_from_relative_path(relative);
        let scope = match context.scope_for(&parsed.path) {
            Some(scope) => scope,
            None => continue,
        };

        if scope.namespace.as_deref() == expected_namespace.as_deref() {
            continue;
        }

        let expected_dir = describe_directory(relative);
        let severity = match config.strictness {
            StrictnessLevel::Strict => Severity::Error,
            _ => Severity::Warning,
        };

        let actual_description = describe_namespace(scope.namespace.as_deref());
        let expected_description = describe_namespace(expected_namespace.as_deref());
        let message = format!(
            "{} does not match PSR-4 directory \"{expected_dir}\" (expected {expected_description})",
            actual_description
        );

        let mut diagnostic = Diagnostic::new(parsed.path.clone(), severity, message);
        diagnostic.rule_name = Some(RULE_NAME.to_string());
        diagnostics.push(diagnostic);
    }

    diagnostics
}

fn resolve_namespace_root(root: &Path, override_root: &Option<PathBuf>) -> PathBuf {
    match override_root {
        Some(custom_root) => {
            let candidate = if custom_root.is_absolute() {
                custom_root.clone()
            } else {
                root.join(custom_root)
            };
            candidate.canonicalize().unwrap_or(candidate)
        }
        None => root.to_path_buf(),
    }
}

fn namespace_from_relative_path(relative: &Path) -> Option<String> {
    let parent = relative.parent()?;
    let mut segments = Vec::new();

    for component in parent.components() {
        let literal = component.as_os_str().to_string_lossy();
        let trimmed = literal.trim();
        if !trimmed.is_empty() {
            segments.push(trimmed.to_string());
        }
    }

    if segments.is_empty() {
        None
    } else {
        Some(segments.join("\\"))
    }
}

fn describe_directory(relative: &Path) -> String {
    match relative.parent() {
        Some(parent) => {
            let dir = parent.display().to_string();
            if dir.is_empty() { ".".to_string() } else { dir }
        }
        None => ".".to_string(),
    }
}

fn describe_namespace(namespace: Option<&str>) -> String {
    match namespace {
        Some(ns) => format!("namespace `{ns}`"),
        None => "no namespace".to_string(),
    }
}
