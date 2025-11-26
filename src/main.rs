use php_checker::analyzer;
use php_checker::analyzer::config::AnalyzerConfig;
use php_checker::analyzer::fix;
use serde::Serialize;
use serde_json::to_writer_pretty;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use std::time::Instant;

#[derive(ValueEnum, Clone, Copy)]
enum OutputFormat {
    Text,
    Json,
}

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
        /// Choose the CLI output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Analyse {
            path,
            fix,
            dry_run,
            format,
        } => run_analysis(path, cli.config, fix, dry_run, format),
    }
}

fn run_analysis(
    path: PathBuf,
    config_path: Option<PathBuf>,
    fix: bool,
    dry_run: bool,
    output_format: OutputFormat,
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
    let php_file_count = php_files.len();

    if php_file_count == 0 {
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

    let fixes = analyzer.fix_root(&canonical_path)?;
    let fixable_count = fixes.values().map(Vec::len).sum::<usize>();

    match output_format {
        OutputFormat::Text => {
            if diagnostics.is_empty() {
                println!(
                    "Analysis complete ▸ {} PHP file(s), no diagnostics emitted yet.",
                    php_file_count
                );
            } else {
                for diag in &diagnostics {
                    println!("{diag}");
                }
            }

            println!(
                "Stats ▸ {} file(s) | {} error(s), {} warning(s) | {:.2}s ({} potentially fixable with --fix)",
                php_file_count,
                error_count,
                warning_count,
                duration.as_secs_f64(),
                fixable_count
            );
        }
        OutputFormat::Json => {
            let stats = JsonStats {
                files: php_file_count,
                errors: error_count,
                warnings: warning_count,
                fixable: fixable_count,
                duration_seconds: duration.as_secs_f64(),
            };
            let output = JsonOutput {
                diagnostics: diagnostics.iter().map(|diag| diag.to_json()).collect(),
                stats,
            };

            let stdout = io::stdout();
            let mut handle = stdout.lock();
            to_writer_pretty(&mut handle, &output)?;
            handle.write_all(b"\n")?;
        }
    }

    if fix {
        if fixes.is_empty() {
            println!("No fixable diagnostics were detected.");
        } else if dry_run {
            for (file, edits) in &fixes {
                let source = fs::read_to_string(&file)
                    .with_context(|| format!("failed to read {}", file.display()))?;
                let patched = fix::apply_text_edits(&source, edits);
                println!("--- {} ---", file.display());
                print!("{patched}");
                if !patched.ends_with('\n') {
                    println!();
                }
            }
        } else {
            for (file, edits) in &fixes {
                let source = fs::read_to_string(&file)
                    .with_context(|| format!("failed to read {}", file.display()))?;
                let patched = fix::apply_text_edits(&source, edits);
                fs::write(&file, patched)
                    .with_context(|| format!("failed to write {}", file.display()))?;
                println!("Fixed {}", file.display());
            }
        }
    }

    Ok(())
}

#[derive(Serialize)]
struct JsonStats {
    files: usize,
    errors: usize,
    warnings: usize,
    fixable: usize,
    duration_seconds: f64,
}

#[derive(Serialize)]
struct JsonOutput {
    diagnostics: Vec<analyzer::DiagnosticJson>,
    stats: JsonStats,
}
