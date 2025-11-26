use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use php_checker::analyzer::config::AnalyzerConfig;
use php_checker::analyzer::{Analyzer, Diagnostic, collect_php_files};

fn diagnostic_summary(diag: &Diagnostic) -> String {
    format!("{}: {}", diag.severity, diag.message)
}

fn expect_lines(path: &Path) -> Result<Vec<String>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read expectation file {}", path.display()))?;
    Ok(content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect())
}

#[test]
fn invalid_fixtures_match_expectations() -> Result<()> {
    let invalid_dir = Path::new("tests/invalid");
    let config = AnalyzerConfig::find_config(None, invalid_dir)
        .map(|path| AnalyzerConfig::load(path))
        .transpose()?;
    let mut analyzer = Analyzer::new(config)?;
    let php_files = collect_php_files(invalid_dir)?;
    let diagnostics = analyzer.analyse_root(invalid_dir)?;

    let mut by_file: HashMap<String, Vec<String>> = HashMap::new();
    for diag in diagnostics {
        if let Some(name) = diag.file.file_name().and_then(|n| n.to_str()) {
            by_file
                .entry(name.to_string())
                .or_default()
                .push(diagnostic_summary(&diag));
        }
    }

    for path in php_files {
        let expect_path = path.with_extension("expect");
        if !expect_path.exists() {
            continue;
        }

        let expect = expect_lines(&expect_path)?;
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            let actual = by_file.remove(name).unwrap_or_default();

            assert_eq!(
                expect,
                actual,
                "analysis output for {} did not match expectations",
                path.display()
            );
        }
    }

    Ok(())
}
