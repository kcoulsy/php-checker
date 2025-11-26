use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use php_checker::analyzer::{fix, Analyzer, collect_php_files};

#[test]
fn fixable_fixtures_match_fixed_expectations() -> Result<()> {
    let invalid_dir = Path::new("tests/invalid");
    let mut analyzer = Analyzer::new(None)?;
    let fixes = analyzer.fix_root(invalid_dir)?;

    for php_file in collect_php_files(invalid_dir)? {
        let canonical_php_file = php_file
            .canonicalize()
            .with_context(|| format!("failed to canonicalize {}", php_file.display()))?;
        let expectation = canonical_php_file.with_extension("expect.fixed");
        if !expectation.exists() {
            continue;
        }

        let source = fs::read_to_string(&canonical_php_file)
            .with_context(|| format!("failed to read {}", canonical_php_file.display()))?;
        let edits = fixes
            .get(&canonical_php_file)
            .cloned()
            .unwrap_or_default();

        if edits.is_empty() {
            panic!(
                "No edits were produced for {} but {} exists",
                canonical_php_file.display(),
                expectation.display()
            );
        }

        let fixed = fix::apply_text_edits(&source, &edits);
        let expected = fs::read_to_string(&expectation)
            .with_context(|| format!("failed to read {}", expectation.display()))?;

        assert_eq!(
            expected,
            fixed,
            "Fixed output for {} diverged from expectations",
            canonical_php_file.display()
        );
    }

    Ok(())
}

