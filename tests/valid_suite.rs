use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;

use php_checker::analyzer::{Analyzer, collect_php_files};

fn diagnostic_summary(diag: &php_checker::analyzer::Diagnostic) -> String {
    format!("{}: {}", diag.severity, diag.message)
}

#[test]
fn valid_fixtures_have_no_diagnostics() -> Result<()> {
    let valid_dir = Path::new("tests/valid");
    let php_files = collect_php_files(valid_dir)?;

    let mut analyzer = Analyzer::new()?;
    let diagnostics = analyzer.analyse_root(valid_dir)?;

    let mut by_file: HashMap<String, Vec<String>> = HashMap::new();
    for diag in diagnostics {
        by_file
            .entry(diag.file.display().to_string())
            .or_default()
            .push(diagnostic_summary(&diag));
    }

    for path in php_files {
        if let Some(existing) = by_file.remove(&path.display().to_string()) {
            panic!(
                "Valid fixture {} emitted diagnostics: {:?}",
                path.display(),
                existing
            );
        }
    }

    Ok(())
}
