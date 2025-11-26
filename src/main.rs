use php_checker::analyzer;
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
}

#[derive(Subcommand)]
enum Commands {
    /// Analyse a PHP file or directory of PHP fixtures.
    Analyse {
        /// Path to a PHP file or directory containing PHP files.
        path: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Analyse { path } => run_analysis(path),
    }
}

fn run_analysis(path: PathBuf) -> Result<()> {
    let canonical_path = path
        .canonicalize()
        .with_context(|| format!("failed to access {}", path.display()))?;

    let php_files = analyzer::collect_php_files(&canonical_path)?;

    if php_files.is_empty() {
        println!("No PHP files found under {}", canonical_path.display());
        return Ok(());
    }

    let mut analyzer = analyzer::Analyzer::new()?;
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

    Ok(())
}
