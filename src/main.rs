use php_checker::analyzer;
use php_checker::analyzer::config::AnalyzerConfig;
use php_checker::analyzer::fix;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::time::Instant;

/// Entry point for the PHP checker CLI.
#[derive(Parser)]
#[command(author, version, about = "Static analysis prototype for PHP fixtures.")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    #[arg(long, value_name = "FILE")]
    config: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    /// Analyse a PHP file or directory of PHP fixtures.
    Analyse {
        /// Path to a PHP file or directory containing PHP files.
        path: PathBuf,
        /// Apply available fixes when diagnostics are emitted.
        #[arg(long)]
        fix: bool,
        /// Preview the fix output without modifying files.
        #[arg(long, requires = "fix")]
        dry_run: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Analyse { path, fix, dry_run } => {
            run_analysis(path, cli.config, fix, dry_run)
        }
    }
}

fn run_analysis(
    path: PathBuf,
    config_path: Option<PathBuf>,
    fix: bool,
    dry_run: bool,
) -> Result<()> {
    let canonical_path = path
        .canonicalize()
        .with_context(|| format!("failed to access {}", path.display()))?;

    let config_file = AnalyzerConfig::find_config(config_path, &canonical_path);
    let config = if let Some(path) = config_file {
        Some(AnalyzerConfig::load(path)?)
    } else {
        None
    };

    let php_files = analyzer::collect_php_files(&canonical_path)?;

    if php_files.is_empty() {
        println!("No PHP files found under {}", canonical_path.display());
        return Ok(());
    }

    let mut analyzer = analyzer::Analyzer::new(config)?;
    let start = Instant::now();
    let diagnostics = analyzer.analyse_root(&canonical_path)?;
    let duration = start.elapsed();
    let error_count = diagnostics
        .iter()
        .filter(|d| matches!(d.severity, analyzer::Severity::Error))
        .count();
    let warning_count = diagnostics
        .iter()
        .filter(|d| matches!(d.severity, analyzer::Severity::Warning))
        .count();

    if diagnostics.is_empty() {
        println!(
            "Analysis complete ▸ {} PHP file(s), no diagnostics emitted yet.",
            php_files.len()
        );
    } else {
        for diag in diagnostics {
            println!("{diag}");
        }
    }

    println!(
        "Stats ▸ {} file(s) | {} error(s), {} warning(s) | {:.2}s",
        php_files.len(),
        error_count,
        warning_count,
        duration.as_secs_f64()
    );

    if fix {
        let fixes = analyzer.fix_root(&canonical_path)?;
        if fixes.is_empty() {
            println!("No fixable diagnostics were detected.");
        } else if dry_run {
            for (file, edits) in fixes {
                let source = fs::read_to_string(&file)
                    .with_context(|| format!("failed to read {}", file.display()))?;
                let patched = fix::apply_text_edits(&source, &edits);
                println!("--- {} ---", file.display());
                print!("{patched}");
                if !patched.ends_with('\n') {
                    println!();
                }
            }
        } else {
            for (file, edits) in fixes {
                let source = fs::read_to_string(&file)
                    .with_context(|| format!("failed to read {}", file.display()))?;
                let patched = fix::apply_text_edits(&source, &edits);
                fs::write(&file, patched)
                    .with_context(|| format!("failed to write {}", file.display()))?;
                println!("Fixed {}", file.display());
            }
        }
    }

    Ok(())
}
