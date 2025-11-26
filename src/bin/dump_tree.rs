use std::fs;

use anyhow::{Context, Result};
use tree_sitter::Parser;

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
    let path = std::env::args().nth(1).context("path argument missing")?;
    let source = fs::read_to_string(&path).with_context(|| format!("read {}", path))?;

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
