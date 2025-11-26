mod parser;
mod rules;

use std::{
    fmt,
    path::{Path, PathBuf},
};

use anyhow::Result;
use walkdir::WalkDir;

/// Represents the severity of a diagnostic.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Info => write!(f, "info"),
            Severity::Warning => write!(f, "warning"),
            Severity::Error => write!(f, "error"),
        }
    }
}

/// A diagnostic that can be emitted during analysis.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub file: PathBuf,
    pub severity: Severity,
    pub message: String,
}

impl Diagnostic {
    pub fn new(file: PathBuf, severity: Severity, message: impl Into<String>) -> Self {
        Self {
            file,
            severity,
            message: message.into(),
        }
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] {}: {}",
            self.severity,
            self.file.display(),
            self.message
        )
    }
}

/// Lightweight analyzer that drives future passes.
pub struct Analyzer {
    parser: Box<dyn parser::PhpParser>,
    rules: Vec<Box<dyn rules::DiagnosticRule>>,
}

impl Analyzer {
    pub fn new() -> Result<Self> {
        let parser = Box::new(parser::TreeSitterPhpParser::new()?);
        let rules: Vec<Box<dyn rules::DiagnosticRule>> = vec![
            Box::new(rules::UndefinedVariableRule::new()),
            Box::new(rules::MissingReturnRule::new()),
            Box::new(rules::TypeMismatchRule::new()),
            Box::new(rules::UnreachableCodeRule::new()),
        ];

        Ok(Self { parser, rules })
    }

    pub fn analyse_file(&mut self, path: &Path) -> Result<Vec<Diagnostic>> {
        let parsed = self.parser.parse_file(path)?;
        let mut diagnostics = Vec::new();

        for rule in &self.rules {
            let _rule_name = rule.name();
            diagnostics.extend(rule.run(&parsed));
        }

        Ok(diagnostics)
    }
}

pub fn collect_php_files(root: &Path) -> Result<Vec<PathBuf>> {
    if root.is_file() {
        return Ok(if is_php_file(root) {
            vec![root.to_path_buf()]
        } else {
            vec![]
        });
    }

    let mut php_files = Vec::new();

    for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if entry.file_type().is_file() && is_php_file(path) {
            php_files.push(path.to_path_buf());
        }
    }

    Ok(php_files)
}

fn is_php_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map_or(false, |ext| ext.eq_ignore_ascii_case("php"))
}
