pub mod config;
pub mod fix;
pub mod ignore;
mod parser;
pub mod phpdoc;
mod project;
mod rules;
pub mod test_config;

use std::{
    collections::BTreeMap,
    fmt,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use config::AnalyzerConfig;
use ignore::IgnoreState;
use parser::PhpParser;
use rayon::prelude::*;
use rules::psr4;
use serde::Serialize;
use test_config::TestConfig;

use anyhow::Result;
use project::{ProjectContext, collect_file_metadata};
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
    rules: Vec<Arc<dyn rules::DiagnosticRule>>,
    config: AnalyzerConfig,
}

impl Analyzer {
    pub fn new(config: Option<AnalyzerConfig>) -> Result<Self> {
        let parser = Box::new(parser::TreeSitterPhpParser::new()?);
        let mut rules: Vec<Arc<dyn rules::DiagnosticRule>> = vec![
            Arc::new(rules::UndefinedVariableRule::new()),
            Arc::new(rules::ArrayKeyNotDefinedRule::new()),
            Arc::new(rules::MissingReturnRule::new()),
            Arc::new(rules::MissingArgumentRule::new()),
            Arc::new(rules::TypeMismatchRule::new()),
            Arc::new(rules::ConsistentReturnRule::new()),
            Arc::new(rules::ForceReturnTypeRule::new()),
            Arc::new(rules::DuplicateDeclarationRule::new()),
            Arc::new(rules::ImpossibleComparisonRule::new()),
            Arc::new(rules::RedundantConditionRule::new()),
            Arc::new(rules::DuplicateSwitchCaseRule::new()),
            Arc::new(rules::FallthroughRule::new()),
            Arc::new(rules::UnreachableCodeRule::new()),
            Arc::new(rules::UnreachableStatementRule::new()),
            Arc::new(rules::UnusedVariableRule::new()),
            Arc::new(rules::UnusedUseRule::new()),
            Arc::new(rules::InvalidThisRule::new()),
            Arc::new(rules::DeprecatedApiRule::new()),
            Arc::new(rules::MutatingLiteralRule::new()),
            Arc::new(rules::StrictTypesRule::new()),
            Arc::new(rules::IncludeUserInputRule::new()),
            Arc::new(rules::HardCodedCredentialsRule::new()),
            Arc::new(rules::WeakHashingRule::new()),
            Arc::new(rules::HardCodedKeysRule::new()),
            Arc::new(rules::PhpDocVarCheckRule::new()),
            Arc::new(rules::PhpDocParamCheckRule::new()),
            Arc::new(rules::PhpDocReturnCheckRule::new()),
            Arc::new(rules::PhpDocReturnValueCheckRule::new()),
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
        self.analyse_root_with_progress(root, None)
    }

    pub fn analyse_root_with_progress(
        &mut self,
        root: &Path,
        progress: Option<&indicatif::ProgressBar>,
    ) -> Result<Vec<Diagnostic>> {
        let canonical_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
        let paths = collect_php_files(&canonical_root)?;
        self.analyse_files_with_progress(&paths, &canonical_root, progress)
    }

    pub fn analyse_files_with_progress(
        &mut self,
        paths: &[PathBuf],
        root: &Path,
        progress: Option<&indicatif::ProgressBar>,
    ) -> Result<Vec<Diagnostic>> {
        if paths.is_empty() {
            return Ok(Vec::new());
        }

        if let Some(pb) = progress {
            pb.set_length(paths.len() as u64);
            pb.set_message("Parsing files");
        }

        let context = parse_files(paths, progress)?;
        let file_count = context.len();

        if let Some(pb) = progress {
            pb.set_message("Analyzing");
            pb.set_length(file_count as u64);
            pb.set_position(0);
        }

        let context = Arc::new(context);
        let parsed_files: Vec<&parser::ParsedSource> = context.iter().collect();
        let rules = self.rules.clone();
        let pb_for_diag = progress.map(|p| p.clone());
        let context_for_diag = context.clone();

        let diagnostics: Vec<_> = parsed_files
            .par_iter()
            .flat_map_iter(move |parsed| {
                if let Some(ref pb) = pb_for_diag {
                    pb.inc(1);
                }
                let mut diags =
                    collect_diagnostics_with_rules(&rules, parsed, context_for_diag.as_ref());
                if let Some(ref pb) = pb_for_diag {
                    for diag in &diags {
                        pb.println(format!("{diag}"));
                    }
                }
                diags
            })
            .collect();

        let mut all_diagnostics = diagnostics;

        if self.config.psr4.enabled {
            all_diagnostics.extend(psr4::run_namespace_checks(
                root,
                context.as_ref(),
                &self.config,
            ));
        }

        Ok(all_diagnostics)
    }

    pub fn fix_root(&mut self, root: &Path) -> Result<BTreeMap<PathBuf, Vec<fix::TextEdit>>> {
        let canonical_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
        let paths = collect_php_files(&canonical_root)?;
        self.fix_files(&paths)
    }

    pub fn fix_files(
        &mut self,
        paths: &[PathBuf],
    ) -> Result<BTreeMap<PathBuf, Vec<fix::TextEdit>>> {
        if paths.is_empty() {
            return Ok(BTreeMap::new());
        }

        let context = parse_files(paths, None)?;
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
        collect_diagnostics_with_rules(&self.rules, parsed, context)
    }

    // run_psr4_checks moved to `rules::psr4`.
}

fn collect_diagnostics_with_rules(
    rules: &[Arc<dyn rules::DiagnosticRule>],
    parsed: &parser::ParsedSource,
    context: &ProjectContext,
) -> Vec<Diagnostic> {
    let ignore_state = IgnoreState::from_source(parsed.source.as_str());
    if ignore_state.ignores_everything() {
        return Vec::new();
    }

    let test_config = TestConfig::from_source(parsed.source.as_str());

    let mut diagnostics = Vec::new();
    for rule in rules {
        let rule_name = rule.name().to_string();

        if test_config.is_test_file() && !test_config.should_run_rule(&rule_name) {
            continue;
        }

        let mut rule_diagnostics = rule.run(parsed, context);
        for diag in rule_diagnostics.iter_mut() {
            diag.rule_name = Some(rule_name.clone());
        }
        diagnostics.extend(rule_diagnostics);
    }

    diagnostics
        .into_iter()
        .filter(|diag| {
            diag.rule_name
                .as_deref()
                .map_or(true, |name| !ignore_state.should_ignore(name))
        })
        .collect()
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

pub fn collect_php_files_from_roots(roots: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut php_files = Vec::new();
    for root in roots {
        let mut files = collect_php_files(root)?;
        php_files.append(&mut files);
    }
    php_files.sort();
    php_files.dedup();
    Ok(php_files)
}

fn parse_files(
    paths: &[PathBuf],
    progress: Option<&indicatif::ProgressBar>,
) -> Result<ProjectContext> {
    let context = Arc::new(Mutex::new(ProjectContext::new()));
    let pb = progress.map(|p| p.clone());

    let results: Vec<Result<()>> = paths
        .par_iter()
        .map(|path| {
            let mut parser = Box::new(parser::TreeSitterPhpParser::new()?);
            let parsed = parser.parse_file(path)?;
            let metadata = collect_file_metadata(&parsed);
            context
                .lock()
                .unwrap()
                .insert_with_metadata(parsed, metadata);
            if let Some(ref pb) = pb {
                pb.inc(1);
            }
            Ok(())
        })
        .collect();

    for result in results {
        result?;
    }

    let context = Arc::try_unwrap(context)
        .unwrap_or_else(|_| {
            panic!("Failed to unwrap Arc - multiple references still exist");
        })
        .into_inner()
        .unwrap_or_else(|_| {
            panic!("Failed to unwrap Mutex: poisoned lock");
        });

    Ok(context)
}

pub fn is_php_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map_or(false, |ext| ext.eq_ignore_ascii_case("php"))
}
