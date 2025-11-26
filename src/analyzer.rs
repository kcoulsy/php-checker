pub mod config;
pub mod fix;
mod parser;
mod project;
mod rules;

use std::{
    collections::BTreeMap,
    fmt,
    path::{Path, PathBuf},
};

use config::AnalyzerConfig;
use rules::psr4;
use serde::Serialize;

use anyhow::Result;
use project::ProjectContext;
use tree_sitter::Point;
use walkdir::WalkDir;

/// Represents the severity of a diagnostic.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
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
    pub rule_name: Option<String>,
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
            rule_name: None,
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
            rule_name: None,
        }
    }

    pub fn to_json(&self) -> DiagnosticJson {
        DiagnosticJson {
            file: self.file.display().to_string(),
            severity: self.severity.clone(),
            message: self.message.clone(),
            rule_name: self.rule_name.clone(),
            span: self.span.as_ref().map(|span| span.into()),
            snippet_before: self.snippet_before.clone(),
            snippet_line: self.snippet_line.clone(),
            snippet_after: self.snippet_after.clone(),
            caret_col: self.caret_col,
            caret_len: self.caret_len,
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
        let mut header = format!("{}{}{}", severity_color, self.severity, RESET);
        if let Some(rule) = &self.rule_name {
            header.push(' ');
            header.push('[');
            header.push_str(rule);
            header.push(']');
        }

        writeln!(f, "{}: {}", header, self.message)?;

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

#[derive(Serialize)]
pub struct DiagnosticJson {
    file: String,
    severity: Severity,
    message: String,
    rule_name: Option<String>,
    span: Option<SpanJson>,
    snippet_before: Option<String>,
    snippet_line: Option<String>,
    snippet_after: Option<String>,
    caret_col: Option<usize>,
    caret_len: usize,
}

#[derive(Serialize)]
pub struct SpanJson {
    start: PointJson,
    end: PointJson,
}

#[derive(Serialize)]
pub struct PointJson {
    row: usize,
    column: usize,
}

impl From<&Span> for SpanJson {
    fn from(span: &Span) -> Self {
        Self {
            start: span.start.into(),
            end: span.end.into(),
        }
    }
}

impl From<Point> for PointJson {
    fn from(point: Point) -> Self {
        Self {
            row: point.row,
            column: point.column,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tree_sitter::Point;

    #[test]
    fn diagnostic_to_json_includes_span_and_snippets() {
        let span = Span {
            start: Point { row: 1, column: 2 },
            end: Point { row: 1, column: 5 },
        };

        let diag = Diagnostic::with_span(
            PathBuf::from("example.php"),
            Severity::Warning,
            "example message",
            span,
            Some("before".into()),
            Some("line".into()),
            Some("after".into()),
            Some(4),
            3,
        );

        let json = diag.to_json();

        assert_eq!(json.file, "example.php");
        assert_eq!(json.severity, Severity::Warning);
        let span_json = json.span.as_ref().expect("span should be set");
        assert_eq!(span_json.start.row, 1);
        assert_eq!(span_json.start.column, 2);
        assert_eq!(span_json.end.column, 5);
        assert_eq!(json.snippet_before.as_deref(), Some("before"));
        assert_eq!(json.snippet_line.as_deref(), Some("line"));
        assert_eq!(json.snippet_after.as_deref(), Some("after"));
        assert_eq!(json.caret_col, Some(4));
        assert_eq!(json.caret_len, 3);
    }
}

/// Lightweight analyzer that drives future passes.
pub struct Analyzer {
    parser: Box<dyn parser::PhpParser>,
    rules: Vec<Box<dyn rules::DiagnosticRule>>,
    config: AnalyzerConfig,
}

impl Analyzer {
    pub fn new(config: Option<AnalyzerConfig>) -> Result<Self> {
        let parser = Box::new(parser::TreeSitterPhpParser::new()?);
        let mut rules: Vec<Box<dyn rules::DiagnosticRule>> = vec![
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
            Box::new(rules::IncludeUserInputRule::new()),
            Box::new(rules::HardCodedCredentialsRule::new()),
        ];

        let config = config.unwrap_or_default();
        rules.retain(|rule| config.enabled(rule.name()));

        Ok(Self {
            parser,
            rules,
            config,
        })
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
        let canonical_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
        let paths = collect_php_files(&canonical_root)?;
        let mut context = ProjectContext::new();

        for path in paths {
            let parsed = self.parser.parse_file(&path)?;
            context.insert(parsed);
        }

        let mut diagnostics = Vec::new();
        for parsed in context.iter() {
            diagnostics.extend(self.collect_diagnostics(parsed, &context));
        }

        if self.config.psr4.enabled {
            diagnostics.extend(psr4::run_namespace_checks(
                &canonical_root,
                &context,
                &self.config,
            ));
        }

        Ok(diagnostics)
    }

    pub fn fix_root(&mut self, root: &Path) -> Result<BTreeMap<PathBuf, Vec<fix::TextEdit>>> {
        let canonical_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
        let paths = collect_php_files(&canonical_root)?;
        let mut context = ProjectContext::new();

        for path in paths {
            let parsed = self.parser.parse_file(&path)?;
            context.insert(parsed);
        }

        let mut edits: BTreeMap<PathBuf, Vec<fix::TextEdit>> = BTreeMap::new();
        for parsed in context.iter() {
            for rule in &self.rules {
                let mut rule_edits = rule.fix(parsed, &context);
                if rule_edits.is_empty() {
                    continue;
                }
                edits
                    .entry(parsed.path.clone())
                    .or_default()
                    .append(&mut rule_edits);
            }
        }

        Ok(edits)
    }

    fn collect_diagnostics(
        &self,
        parsed: &parser::ParsedSource,
        context: &ProjectContext,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        for rule in &self.rules {
            let rule_name = rule.name().to_string();
            let mut rule_diagnostics = rule.run(parsed, context);
            for diag in rule_diagnostics.iter_mut() {
                diag.rule_name = Some(rule_name.clone());
            }
            diagnostics.extend(rule_diagnostics);
        }
        diagnostics
    }

    // run_psr4_checks moved to `rules::psr4`.
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
