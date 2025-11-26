//! Utilities to honor in-source ignore directives for diagnostics.

const DIRECTIVE: &str = "php-checker-ignore";
const FILE_DIRECTIVE: &str = "php-checker-ignore-file";

/// Tracks the ignore directives declared in a file.
#[derive(Clone, Debug, Default)]
pub struct IgnoreState {
    ignore_all: bool,
    patterns: Vec<String>,
}

impl IgnoreState {
    /// Parses the ignore directives declared in the supplied source.
    pub fn from_source(source: &str) -> Self {
        let mut state = Self::default();

        for line in source.lines() {
            if state.ignore_all {
                break;
            }

            state.collect_from_line(line);
        }

        state
    }

    fn collect_from_line(&mut self, line: &str) {
        if let Some(idx) = line.find(FILE_DIRECTIVE) {
            self.ignore_all = true;
            self.apply_args(&line[idx + FILE_DIRECTIVE.len()..]);
            return;
        }

        if let Some(idx) = line.find(DIRECTIVE) {
            self.apply_args(&line[idx + DIRECTIVE.len()..]);
        }
    }

    fn apply_args(&mut self, tail: &str) {
        if self.ignore_all {
            return;
        }

        let mut args = trim_comment_tail(tail).trim_start();
        if args.is_empty() {
            self.ignore_all = true;
            return;
        }

        if let Some(stripped) = args.strip_prefix(':') {
            args = stripped.trim_start();
        }

        if args.is_empty() {
            self.ignore_all = true;
            return;
        }

        for token in args.split(|c: char| c == ',' || c.is_whitespace()) {
            let mut trimmed = token
                .trim()
                .trim_matches(|c| c == '"' || c == '\'' || c == '`');
            trimmed = trimmed.trim_end_matches('/');
            if trimmed.is_empty() {
                continue;
            }

            let normalized = trimmed.to_ascii_lowercase();
            if ["*", "all", "file"].contains(&normalized.as_str()) {
                self.ignore_all = true;
                break;
            }

            self.patterns.push(normalized);
        }
    }

    /// Returns `true` if diagnostics emitted for `rule_name` should be suppressed.
    pub fn should_ignore(&self, rule_name: &str) -> bool {
        if self.ignore_all {
            return true;
        }

        let rule_lower = rule_name.to_ascii_lowercase();
        let rule_bytes = rule_lower.as_bytes();

        for pattern in &self.patterns {
            if rule_lower == *pattern {
                return true;
            }

            if rule_lower.starts_with(pattern) {
                if rule_bytes.len() > pattern.len() && rule_bytes[pattern.len()] == b'/' {
                    return true;
                }
            }
        }

        false
    }

    /// Returns `true` if a file-level ignore directive was encountered.
    pub fn ignores_everything(&self) -> bool {
        self.ignore_all
    }
}

fn trim_comment_tail(value: &str) -> &str {
    let mut limit = value.len();

    for marker in ["//", "/*", "#", "*/"] {
        if let Some(idx) = value.find(marker) {
            limit = limit.min(idx);
        }
    }

    value[..limit].trim_end()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_file_when_directive_has_no_tokens() {
        let source = "// php-checker-ignore\n";
        let state = IgnoreState::from_source(source);
        assert!(state.ignores_everything());
    }

    #[test]
    fn ignores_file_when_directive_is_file_alias() {
        let source = "/* php-checker-ignore-file */";
        let state = IgnoreState::from_source(source);
        assert!(state.ignores_everything());
    }

    #[test]
    fn parses_rule_and_group_tokens() {
        let source = "
            // php-checker-ignore: cleanup/unused_use cleanup
            // php-checker-ignore strict_typing/missing_argument
        ";

        let state = IgnoreState::from_source(source);
        assert!(state.should_ignore("cleanup/unused_use"));
        assert!(state.should_ignore("cleanup/unused_variable"));
        assert!(state.should_ignore("strict_typing/missing_argument"));
        assert!(!state.should_ignore("strict_typing/missing_return"));
    }

    #[test]
    fn stops_parsing_at_inline_comment_end() {
        let source = "// php-checker-ignore: cleanup // extra notes";
        let state = IgnoreState::from_source(source);
        assert!(state.should_ignore("cleanup/unused_use"));
        assert!(!state.should_ignore("strict_typing/missing_argument"));
    }

    #[test]
    fn ignores_group_from_block_comment_without_hitting_wildcard() {
        let source = "/* php-checker-ignore: cleanup */";
        let state = IgnoreState::from_source(source);
        assert!(!state.ignores_everything());
        assert!(state.should_ignore("cleanup/unused_use"));
        assert!(!state.should_ignore("strict_typing/missing_argument"));
    }
}
