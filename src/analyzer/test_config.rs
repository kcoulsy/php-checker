/// Test configuration parsed from special comments at the top of test files
#[derive(Debug, Clone, Default)]
pub struct TestConfig {
    /// If set, only run these rules (ignore all others)
    pub only_rules: Option<Vec<String>>,

    /// Rules to skip
    pub skip_rules: Vec<String>,
}

impl TestConfig {
    /// Parse test configuration from source code
    ///
    /// Looks for special comments like:
    /// // php-checker-test: only-rules=strict_typing/phpdoc_var_check,strict_typing/phpdoc_param_check
    /// // php-checker-test: skip-rules=sanity/undefined_variable
    pub fn from_source(source: &str) -> Self {
        let mut config = TestConfig::default();

        for line in source.lines().take(20) {
            // Only check first 20 lines
            let line = line.trim();

            // Check for test config comment
            if let Some(directive) = line.strip_prefix("// php-checker-test:") {
                Self::parse_directive(directive.trim(), &mut config);
            }
        }

        config
    }

    fn parse_directive(directive: &str, config: &mut TestConfig) {
        if let Some(rules_str) = directive.strip_prefix("only-rules=") {
            let rules: Vec<String> = rules_str
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            if !rules.is_empty() {
                config.only_rules = Some(rules);
            }
        } else if let Some(rules_str) = directive.strip_prefix("skip-rules=") {
            let rules: Vec<String> = rules_str
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            config.skip_rules.extend(rules);
        }
    }

    /// Check if a rule should run based on this test config
    pub fn should_run_rule(&self, rule_name: &str) -> bool {
        // If skip list contains this rule, skip it
        if self.skip_rules.iter().any(|r| r == rule_name) {
            return false;
        }

        // If only_rules is set, only run rules in that list
        if let Some(ref only_rules) = self.only_rules {
            return only_rules.iter().any(|r| r == rule_name);
        }

        // Otherwise, run the rule
        true
    }

    /// Check if this is a test file (has test config directives)
    pub fn is_test_file(&self) -> bool {
        self.only_rules.is_some() || !self.skip_rules.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_only_rules() {
        let source = r#"<?php
// php-checker-test: only-rules=strict_typing/phpdoc_var_check

class Test {}
"#;
        let config = TestConfig::from_source(source);
        assert!(config.only_rules.is_some());
        assert_eq!(
            config.only_rules.unwrap(),
            vec!["strict_typing/phpdoc_var_check"]
        );
    }

    #[test]
    fn test_parse_multiple_only_rules() {
        let source = r#"<?php
// php-checker-test: only-rules=strict_typing/phpdoc_var_check,strict_typing/phpdoc_param_check

class Test {}
"#;
        let config = TestConfig::from_source(source);
        assert!(config.only_rules.is_some());
        let rules = config.only_rules.unwrap();
        assert_eq!(rules.len(), 2);
        assert!(rules.contains(&"strict_typing/phpdoc_var_check".to_string()));
        assert!(rules.contains(&"strict_typing/phpdoc_param_check".to_string()));
    }

    #[test]
    fn test_parse_skip_rules() {
        let source = r#"<?php
// php-checker-test: skip-rules=sanity/undefined_variable,cleanup/unused_variable

class Test {}
"#;
        let config = TestConfig::from_source(source);
        assert_eq!(config.skip_rules.len(), 2);
        assert!(
            config
                .skip_rules
                .contains(&"sanity/undefined_variable".to_string())
        );
    }

    #[test]
    fn test_should_run_rule_with_only_rules() {
        let source = r#"<?php
// php-checker-test: only-rules=strict_typing/phpdoc_var_check

class Test {}
"#;
        let config = TestConfig::from_source(source);

        assert!(config.should_run_rule("strict_typing/phpdoc_var_check"));
        assert!(!config.should_run_rule("sanity/undefined_variable"));
        assert!(!config.should_run_rule("cleanup/unused_variable"));
    }

    #[test]
    fn test_should_run_rule_with_skip_rules() {
        let source = r#"<?php
// php-checker-test: skip-rules=sanity/undefined_variable

class Test {}
"#;
        let config = TestConfig::from_source(source);

        assert!(!config.should_run_rule("sanity/undefined_variable"));
        assert!(config.should_run_rule("strict_typing/phpdoc_var_check"));
        assert!(config.should_run_rule("cleanup/unused_variable"));
    }

    #[test]
    fn test_no_config() {
        let source = r#"<?php

class Test {}
"#;
        let config = TestConfig::from_source(source);

        assert!(!config.is_test_file());
        assert!(config.should_run_rule("sanity/undefined_variable"));
        assert!(config.should_run_rule("strict_typing/phpdoc_var_check"));
    }
}
