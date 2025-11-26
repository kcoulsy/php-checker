use anyhow::{Context, Result};
use serde::Deserialize;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

/// Configuration for enabling/disabling individual rules plus general analyzer settings.
#[derive(Clone, Debug, Deserialize, Default)]
#[serde(default)]
pub struct AnalyzerConfig {
    #[serde(default)]
    pub rules: HashMap<String, bool>,
    #[serde(default)]
    pub psr4: Psr4Config,
}

impl AnalyzerConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)
            .with_context(|| format!("failed to read config {}", path.display()))?;
        let config = serde_yaml::from_str(&content)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        Ok(config)
    }

    pub fn enabled(&self, rule_name: &str) -> bool {
        let mut candidate = rule_name;
        loop {
            if let Some(enabled) = self.rules.get(candidate) {
                return *enabled;
            }

            if let Some(idx) = candidate.rfind('/') {
                candidate = &candidate[..idx];
                continue;
            }

            break;
        }

        true
    }

    pub fn find_config(path: Option<PathBuf>, root: &Path) -> Option<PathBuf> {
        if let Some(path) = path {
            return Some(path);
        }

        let candidates = ["php_checker.yaml", "php_checker.yml"];
        for candidate in &candidates {
            let candidate_path = root.join(candidate);
            if candidate_path.is_file() {
                return Some(candidate_path);
            }
        }

        None
    }
}

/// PSR-4 expectations that the analyzer can validate when requested.
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct Psr4Config {
    pub enabled: bool,
    pub namespace_root: Option<PathBuf>,
}

impl Default for Psr4Config {
    fn default() -> Self {
        Self {
            enabled: false,
            namespace_root: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn psr4_config_deserializes_values() {
        let yaml = "psr4:\n  enabled: true\n  namespace_root: src";
        let config: AnalyzerConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.psr4.enabled);
        assert_eq!(config.psr4.namespace_root, Some(PathBuf::from("src")));
    }

    #[test]
    fn rule_group_defaults_propagate_to_children() {
        let mut config = AnalyzerConfig::default();
        config.rules.insert("psr4".to_string(), false);
        assert!(!config.enabled("psr4/namespace"));
    }

    #[test]
    fn specific_rule_toggle_overrides_group() {
        let mut config = AnalyzerConfig::default();
        config.rules.insert("psr4".to_string(), true);
        config.rules.insert("psr4/namespace".to_string(), false);

        assert!(config.enabled("psr4"));
        assert!(!config.enabled("psr4/namespace"));
        assert!(config.enabled("psr4/anything"));
    }
}
