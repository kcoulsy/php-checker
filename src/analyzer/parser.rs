use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Result};
use tree_sitter::Parser;

/// Parsed contents of a source file.
#[allow(dead_code)]
pub struct ParsedSource {
    pub path: PathBuf,
    pub source: Arc<String>,
    pub tree: tree_sitter::Tree,
}

/// Trait that abstracts PHP parsing implementations.
pub trait PhpParser {
    fn parse_file(&mut self, path: &Path) -> Result<ParsedSource>;
}

/// Parser wrapper that uses tree-sitter-php as the backend.
pub struct TreeSitterPhpParser {
    parser: Parser,
}

impl TreeSitterPhpParser {
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        let language = tree_sitter_php::language();
        parser
            .set_language(language)
            .context("failed to load tree-sitter-php language")?;

        Ok(Self { parser })
    }
}

impl PhpParser for TreeSitterPhpParser {
    fn parse_file(&mut self, path: &Path) -> Result<ParsedSource> {
        let source = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let source = Arc::new(source);

        let tree = self
            .parser
            .parse(source.as_str(), None)
            .context("tree-sitter failed to parse PHP source")?;

        Ok(ParsedSource {
            path: path.to_path_buf(),
            source,
            tree,
        })
    }
}
