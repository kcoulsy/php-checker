use php_checker::analyzer;
use php_checker::analyzer::config::AnalyzerConfig;
use php_checker::analyzer::fix;
use serde::Serialize;
use serde_json::to_writer_pretty;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand, ValueEnum};
use glob::glob;
use indicatif::{ProgressBar, ProgressStyle};
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
    let requested_targets = resolve_targets(&path)?;
    let canonical_targets = canonicalize_paths(requested_targets)?;
    let analysis_root = derive_analysis_root(&canonical_targets);

    let config_file = AnalyzerConfig::find_config(config_path, &analysis_root);
    let config = if let Some(path) = config_file {
        Some(AnalyzerConfig::load(path)?)
    } else {
        None
    };

    let php_files = analyzer::collect_php_files_from_roots(&canonical_targets)?;
    let php_file_count = php_files.len();

    if php_file_count == 0 {
        println!("No PHP files found under {}", analysis_root.display());
        return Ok(());
    }

    println!("Checking {} file(s)...", php_file_count);

    let mut analyzer = analyzer::Analyzer::new(config)?;
    let start = Instant::now();

    let (diagnostics, diagnostics_streamed) = match output_format {
        OutputFormat::Text => {
            let progress = ProgressBar::new(php_file_count as u64);
            progress.set_style(
                ProgressStyle::default_bar()
                    .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} ({percent}%)")
                    .expect("valid progress bar template")
                    .progress_chars("#>-"),
            );
            let result = analyzer.analyse_files_with_progress(
                &php_files,
                &analysis_root,
                Some(&progress),
            )?;
            progress.finish_and_clear();
            (result, true)
        }
        OutputFormat::Json => (
            analyzer.analyse_files_with_progress(&php_files, &analysis_root, None)?,
            false,
        ),
    };

    let duration = start.elapsed();
    let error_count = diagnostics
        .iter()
        .filter(|d| matches!(d.severity, analyzer::Severity::Error))
        .count();
    let warning_count = diagnostics
        .iter()
        .filter(|d| matches!(d.severity, analyzer::Severity::Warning))
        .count();

    let fixes = analyzer.fix_files(&php_files)?;
    let fixable_count = fixes.values().map(Vec::len).sum::<usize>();

    match output_format {
        OutputFormat::Text => {
            if diagnostics.is_empty() {
                println!(
                    "Analysis complete ▸ {} PHP file(s), no diagnostics emitted yet.",
                    php_file_count
                );
            } else if !diagnostics_streamed {
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

fn resolve_targets(path: &Path) -> Result<Vec<PathBuf>> {
    if path_contains_glob(path) {
        let pattern = path.as_os_str().to_string_lossy().into_owned();
        let matches = glob(&pattern)
            .with_context(|| format!("invalid glob pattern \"{pattern}\""))?
            .collect::<Result<Vec<_>, _>>()
            .with_context(|| format!("failed to read entries for pattern \"{pattern}\""))?;

        if matches.is_empty() {
            bail!("no files matched \"{pattern}\"");
        }

        Ok(matches)
    } else {
        Ok(vec![path.to_path_buf()])
    }
}

fn canonicalize_paths(paths: Vec<PathBuf>) -> Result<Vec<PathBuf>> {
    let mut canonical_paths = Vec::new();
    for path in paths {
        let canonical_path = path
            .canonicalize()
            .with_context(|| format!("failed to access {}", path.display()))?;
        canonical_paths.push(canonical_path);
    }
    canonical_paths.sort();
    canonical_paths.dedup();
    Ok(canonical_paths)
}

fn derive_analysis_root(targets: &[PathBuf]) -> PathBuf {
    let directories: Vec<PathBuf> = targets
        .iter()
        .map(|target| {
            if target.is_file() {
                target
                    .parent()
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| target.clone())
            } else {
                target.clone()
            }
        })
        .collect();

    longest_common_directory(&directories).unwrap_or_else(|| directories[0].clone())
}

fn longest_common_directory(paths: &[PathBuf]) -> Option<PathBuf> {
    if paths.is_empty() {
        return None;
    }

    let mut common = ancestors_from_root(&paths[0]);
    for path in paths.iter().skip(1) {
        let next = ancestors_from_root(path);
        let mut idx = 0;
        while idx < common.len() && idx < next.len() && common[idx] == next[idx] {
            idx += 1;
        }
        common.truncate(idx);
        if common.is_empty() {
            break;
        }
    }

    common.last().cloned()
}

fn ancestors_from_root(path: &Path) -> Vec<PathBuf> {
    let mut ancestors: Vec<PathBuf> = path.ancestors().map(PathBuf::from).collect();
    ancestors.reverse();
    ancestors
}

fn path_contains_glob(path: &Path) -> bool {
    path.as_os_str()
        .to_string_lossy()
        .chars()
        .any(|c| matches!(c, '*' | '?' | '[' | ']' | '{' | '}'))
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
