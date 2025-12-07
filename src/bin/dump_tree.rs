use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use clap::Parser as ClapParser;
use tree_sitter::Parser;

#[derive(ClapParser)]
#[command(name = "dump-tree")]
#[command(about = "Dump the AST structure of PHP code")]
struct Args {
    /// PHP code to parse (file path or string if --string flag is used)
    input: String,

    /// Treat input as a string instead of a file path
    #[arg(short, long)]
    string: bool,
}

fn print_node(node: tree_sitter::Node, source: &str, indent: usize) {
    let text = node
        .utf8_text(source.as_bytes())
        .unwrap_or("<invalid utf8>");
    println!(
        "{:indent$}{} [{:?}:{:?}] {:?}",
        "",
        node.kind(),
        node.start_position(),
        node.end_position(),
        text.trim(),
        indent = indent * 2
    );

    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            print_node(cursor.node(), source, indent + 1);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    let source = if args.string {
        // Treat input as a direct string
        args.input
    } else {
        // Check if input is a file path
        if Path::new(&args.input).exists() {
            fs::read_to_string(&args.input)
                .with_context(|| format!("read {}", args.input))?
        } else {
            // If file doesn't exist, treat as string (for convenience)
            // This allows: cargo run --bin dump-tree "<?php echo 'test';"
            args.input
        }
    };

    let mut parser = Parser::new();
    parser
        .set_language(tree_sitter_php::language())
        .context("load tree-sitter-php language")?;

    let tree = parser
        .parse(source.as_str(), None)
        .context("parse PHP source")?;

    print_node(tree.root_node(), &source, 0);
    Ok(())
}
