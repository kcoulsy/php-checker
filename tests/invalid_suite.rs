use std::{fs, path::Path};

use anyhow::{Context, Result};

use php_checker::analyzer::{Analyzer, Diagnostic};

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
    let entries = fs::read_dir(invalid_dir)?;

    for entry in entries {
        let path = entry?.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("php") {
            continue;
        }

        let expect_path = path.with_extension("expect");
        if !expect_path.exists() {
            continue;
        }

        let expect = expect_lines(&expect_path)?;

        let mut analyzer = Analyzer::new()?;
        let diagnostics = analyzer.analyse_file(&path)?;
        let actual: Vec<String> = diagnostics.iter().map(diagnostic_summary).collect();

        assert_eq!(
            expect,
            actual,
            "analysis output for {} did not match expectations",
            path.display()
        );
    }

    Ok(())
}
