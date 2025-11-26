mod parser;
mod project;
mod rules;

use std::{
    fmt,
    path::{Path, PathBuf},
};

use anyhow::Result;
use project::ProjectContext;
use tree_sitter::Point;
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
pub struct Span {
    pub start: Point,
    pub end: Point,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub file: PathBuf,
    pub severity: Severity,
    pub message: String,
    pub span: Option<Span>,
    pub snippet_before: Option<String>,
    pub snippet_line: Option<String>,
    pub snippet_after: Option<String>,
    pub caret_col: Option<usize>,
    pub caret_len: usize,
}

impl Diagnostic {
    #[allow(dead_code)]
    pub fn new(file: PathBuf, severity: Severity, message: impl Into<String>) -> Self {
        Self {
            file,
            severity,
            message: message.into(),
            span: None,
            snippet_before: None,
            snippet_line: None,
            snippet_after: None,
            caret_col: None,
            caret_len: 1,
        }
    }

    pub fn with_span(
        file: PathBuf,
        severity: Severity,
        message: impl Into<String>,
        span: Span,
        snippet_before: Option<String>,
        snippet_line: Option<String>,
        snippet_after: Option<String>,
        caret_col: Option<usize>,
        caret_len: usize,
    ) -> Self {
        Self {
            file,
            severity,
            message: message.into(),
            span: Some(span),
            snippet_before,
            snippet_line,
            snippet_after,
            caret_col,
            caret_len: caret_len.max(1),
        }
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const RESET: &str = "\x1b[0m";
        const DIM: &str = "\x1b[2m";
        const BOLD_RED: &str = "\x1b[1;31m";
        const BOLD_YELLOW: &str = "\x1b[1;33m";
        const BLUE: &str = "\x1b[34m";

        let severity_color = match self.severity {
            Severity::Warning | Severity::Info => BOLD_YELLOW,
            _ => BOLD_RED,
        };
        writeln!(
            f,
            "{}{}{}: {}",
            severity_color, self.severity, RESET, self.message
        )?;

        if let Some(span) = &self.span {
            writeln!(
                f,
                " --> {}:{}:{}",
                self.file.display(),
                span.start.row + 1,
                span.start.column + 1
            )?;
            writeln!(f, "{BLUE}    |{RESET}")?;
            let prefix_line =
                |line_num: usize| format!("{BLUE}{:>3}{RESET} {BLUE}|{RESET}", line_num);
            let blank_prefix = format!("{BLUE}    |{RESET}");

            if let Some(line_before) = &self.snippet_before {
                writeln!(
                    f,
                    "{} {}{}{}",
                    prefix_line(span.start.row),
                    DIM,
                    line_before,
                    RESET
                )?;
            }

            if let Some(line) = &self.snippet_line {
                writeln!(f, "{} {}", prefix_line(span.start.row + 1), line)?;

                let caret_col = self.caret_col.unwrap_or(0);
                let caret_color = match self.severity {
                    Severity::Warning => BOLD_YELLOW,
                    _ => BOLD_RED,
                };

                writeln!(
                    f,
                    "{} {}{}{}{}",
                    blank_prefix,
                    " ".repeat(caret_col),
                    caret_color,
                    "^".repeat(self.caret_len),
                    RESET
                )?;
            }

            if let Some(line_after) = &self.snippet_after {
                writeln!(
                    f,
                    "{} {}{}{}",
                    prefix_line(span.start.row + 2),
                    DIM,
                    line_after,
                    RESET
                )?;
            }
        } else {
            writeln!(f, " --> {}", self.file.display())?;
        }

        Ok(())
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
            Box::new(rules::ArrayKeyNotDefinedRule::new()),
            Box::new(rules::MissingReturnRule::new()),
            Box::new(rules::MissingArgumentRule::new()),
            Box::new(rules::TypeMismatchRule::new()),
            Box::new(rules::DuplicateDeclarationRule::new()),
            Box::new(rules::ImpossibleComparisonRule::new()),
            Box::new(rules::RedundantConditionRule::new()),
            Box::new(rules::UnreachableCodeRule::new()),
            Box::new(rules::UnusedVariableRule::new()),
            Box::new(rules::UnusedUseRule::new()),
            Box::new(rules::InvalidThisRule::new()),
            Box::new(rules::DeprecatedApiRule::new()),
            Box::new(rules::MutatingLiteralRule::new()),
            Box::new(rules::StrictTypesRule::new()),
        ];

        Ok(Self { parser, rules })
    }

    pub fn analyse_file(&mut self, path: &Path) -> Result<Vec<Diagnostic>> {
        let parsed = self.parser.parse_file(path)?;
        let mut context = ProjectContext::new();
        context.insert(parsed);

        let parsed_ref = context
            .get(path)
            .expect("parsed file should exist in context");

        Ok(self.collect_diagnostics(parsed_ref, &context))
    }

    pub fn analyse_root(&mut self, root: &Path) -> Result<Vec<Diagnostic>> {
        let paths = collect_php_files(root)?;
        let mut context = ProjectContext::new();

        for path in paths {
            let parsed = self.parser.parse_file(&path)?;
            context.insert(parsed);
        }

        let mut diagnostics = Vec::new();
        for parsed in context.iter() {
            diagnostics.extend(self.collect_diagnostics(parsed, &context));
        }

        Ok(diagnostics)
    }

    fn collect_diagnostics(
        &self,
        parsed: &parser::ParsedSource,
        context: &ProjectContext,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        for rule in &self.rules {
            let _ = rule.name();
            diagnostics.extend(rule.run(parsed, context));
        }
        diagnostics
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
