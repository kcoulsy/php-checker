use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

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
    let invalid_dir_canonical = invalid_dir.canonicalize()?;
    let mut analyzer = Analyzer::new()?;
    let php_files = collect_php_files(invalid_dir)?;
    let diagnostics = analyzer.analyse_root(invalid_dir)?;

    let mut by_file: HashMap<PathBuf, Vec<String>> = HashMap::new();
    for diag in diagnostics {
        let key = relative_to_base(&diag.file, &invalid_dir_canonical);
        by_file
            .entry(key)
            .or_default()
            .push(diagnostic_summary(&diag));
    }

    for path in php_files {
        let expect_path = path.with_extension("expect");
        if !expect_path.exists() {
            continue;
        }

        let expect = expect_lines(&expect_path)?;
        let key = relative_to_base(&path, &invalid_dir_canonical);
        let actual = by_file.remove(&key).unwrap_or_default();

        assert_eq!(
            expect,
            actual,
            "analysis output for {} did not match expectations",
            path.display()
        );
    }

    Ok(())
}

fn relative_to_base(path: &Path, base: &Path) -> PathBuf {
    if let Ok(canonical) = path.canonicalize() {
        if let Ok(rel) = canonical.strip_prefix(base) {
            return rel.to_path_buf();
        }
    }
    if let Ok(rel) = path.strip_prefix(base) {
        return rel.to_path_buf();
    }
    path.to_path_buf()
}
