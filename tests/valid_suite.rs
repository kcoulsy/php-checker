use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;

use php_checker::analyzer::{Analyzer, collect_php_files};

fn diagnostic_summary(diag: &php_checker::analyzer::Diagnostic) -> String {
    format!("{}: {}", diag.severity, diag.message)
}

#[derive(Debug)]
struct ValidTestFailure {
    file: PathBuf,
    diagnostics: Vec<String>,
}

impl ValidTestFailure {
    fn format(&self) -> String {
        let mut output = String::new();
        output.push_str(&format!(
            "\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n"
        ));
        output.push_str(&format!(
            "FAILED: {} (should have NO diagnostics)\n",
            self.file.display()
        ));
        output.push_str(&format!(
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n"
        ));
        output.push_str("\nUnexpected diagnostics:\n");
        for (i, diag) in self.diagnostics.iter().enumerate() {
            output.push_str(&format!("  {:2}. {}\n", i + 1, diag));
        }
        output
    }
}

#[test]
fn valid_fixtures_have_no_diagnostics() -> Result<()> {
    let valid_dir = Path::new("tests/valid");
    let php_files = collect_php_files(valid_dir)?;

    let mut analyzer = Analyzer::new(None)?;
    let diagnostics = analyzer.analyse_root(valid_dir)?;

    let mut by_file: HashMap<String, Vec<String>> = HashMap::new();
    for diag in diagnostics {
        by_file
            .entry(diag.file.display().to_string())
            .or_default()
            .push(diagnostic_summary(&diag));
    }

    let mut failures = Vec::new();
    let mut passed = 0;

    for path in php_files {
        if let Some(existing) = by_file.remove(&path.display().to_string()) {
            failures.push(ValidTestFailure {
                file: path,
                diagnostics: existing,
            });
        } else {
            passed += 1;
        }
    }

    if !failures.is_empty() {
        let mut error_msg = String::new();
        error_msg.push_str(&format!(
            "\n\n{} valid test(s) FAILED, {} passed\n",
            failures.len(),
            passed
        ));

        for failure in &failures {
            error_msg.push_str(&failure.format());
        }

        error_msg.push_str("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
        error_msg.push_str(&format!(
            "Summary: {} failed, {} passed\n",
            failures.len(),
            passed
        ));
        error_msg.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

        panic!("{}", error_msg);
    }

    println!("\n✓ All {} valid test(s) passed", passed);
    Ok(())
}
