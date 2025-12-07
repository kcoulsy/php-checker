use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

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

#[derive(Debug)]
struct TestFailure {
    file: PathBuf,
    expected: Vec<String>,
    actual: Vec<String>,
}

impl TestFailure {
    fn format_diff(&self) -> String {
        let mut output = String::new();
        output.push_str(&format!(
            "\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n"
        ));
        output.push_str(&format!("FAILED: {}\n", self.file.display()));
        output.push_str(&format!(
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n"
        ));

        output.push_str("\nExpected diagnostics:\n");
        if self.expected.is_empty() {
            output.push_str("  (none)\n");
        } else {
            for (i, line) in self.expected.iter().enumerate() {
                output.push_str(&format!("  {:2}. {}\n", i + 1, line));
            }
        }

        output.push_str("\nActual diagnostics:\n");
        if self.actual.is_empty() {
            output.push_str("  (none)\n");
        } else {
            for (i, line) in self.actual.iter().enumerate() {
                output.push_str(&format!("  {:2}. {}\n", i + 1, line));
            }
        }

        output.push_str("\nDifferences:\n");

        // Show missing diagnostics
        let missing: Vec<_> = self
            .expected
            .iter()
            .filter(|e| !self.actual.contains(e))
            .collect();
        if !missing.is_empty() {
            output.push_str("  Missing (expected but not found):\n");
            for line in missing {
                output.push_str(&format!("    - {}\n", line));
            }
        }

        // Show unexpected diagnostics
        let unexpected: Vec<_> = self
            .actual
            .iter()
            .filter(|a| !self.expected.contains(a))
            .collect();
        if !unexpected.is_empty() {
            output.push_str("  Unexpected (found but not expected):\n");
            for line in unexpected {
                output.push_str(&format!("    + {}\n", line));
            }
        }

        output
    }
}

// #[test]
// fn invalid_fixtures_match_expectations() -> Result<()> {
//     let invalid_dir = Path::new("tests/invalid");
//     let config = AnalyzerConfig::find_config(None, invalid_dir)
//         .map(|path| AnalyzerConfig::load(path))
//         .transpose()?;
//     let mut analyzer = Analyzer::new(config)?;
//     let php_files = collect_php_files(invalid_dir)?;
//     let diagnostics = analyzer.analyse_root(invalid_dir)?;

//     let mut by_file: HashMap<String, Vec<String>> = HashMap::new();
//     for diag in diagnostics {
//         if let Some(name) = diag.file.file_name().and_then(|n| n.to_str()) {
//             by_file
//                 .entry(name.to_string())
//                 .or_default()
//                 .push(diagnostic_summary(&diag));
//         }
//     }

//     let mut failures = Vec::new();
//     let mut passed = 0;

//     for path in php_files {
//         let expect_path = path.with_extension("expect");
//         if !expect_path.exists() {
//             continue;
//         }

//         let expect = expect_lines(&expect_path)?;
//         if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
//             let actual = by_file.remove(name).unwrap_or_default();

//             if expect != actual {
//                 failures.push(TestFailure {
//                     file: path.clone(),
//                     expected: expect,
//                     actual,
//                 });
//             } else {
//                 passed += 1;
//             }
//         }
//     }

//     if !failures.is_empty() {
//         let mut error_msg = String::new();
//         error_msg.push_str(&format!(
//             "\n\n{} test(s) FAILED, {} passed\n",
//             failures.len(),
//             passed
//         ));

//         for failure in &failures {
//             error_msg.push_str(&failure.format_diff());
//         }

//         error_msg.push_str("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
//         error_msg.push_str(&format!(
//             "Summary: {} failed, {} passed\n",
//             failures.len(),
//             passed
//         ));
//         error_msg.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

//         panic!("{}", error_msg);
//     }

//     println!("\n✓ All {} test(s) passed", passed);
//     Ok(())
// }
