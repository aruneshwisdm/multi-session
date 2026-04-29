use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct JcAppConfig {
    pub attribution: AttributionMode,
    pub vim_mode: bool,
    #[serde(default)]
    pub features: HashMap<String, bool>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AttributionMode {
    #[default]
    CoAuthor,
    Author,
    NoAttribution,
}

impl JcAppConfig {
    pub fn is_enabled(&self, feature: &str) -> bool {
        self.features.get(feature).copied().unwrap_or(false)
    }
}

fn config_path() -> PathBuf {
    dirs::home_dir()
        .expect("could not determine home directory")
        .join(".config/jc/jc-app.toml")
}

pub fn load() -> Result<JcAppConfig> {
    let path = config_path();
    match std::fs::read_to_string(&path) {
        Ok(contents) => {
            toml::from_str(&contents)
                .with_context(|| format!("failed to parse {}", path.display()))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(JcAppConfig::default()),
        Err(e) => Err(anyhow::anyhow!("failed to read {}: {e}", path.display())),
    }
}

pub fn save(config: &JcAppConfig) -> Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let contents =
        toml::to_string_pretty(config).context("failed to serialize jc-app config")?;
    std::fs::write(&path, contents)
        .with_context(|| format!("failed to write {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = JcAppConfig::default();
        assert_eq!(config.attribution, AttributionMode::CoAuthor);
        assert!(!config.vim_mode);
        assert!(config.features.is_empty());
    }

    #[test]
    fn is_enabled_returns_false_for_missing_key() {
        let config = JcAppConfig::default();
        assert!(!config.is_enabled("nonexistent"));
    }

    #[test]
    fn is_enabled_returns_stored_value() {
        let mut config = JcAppConfig::default();
        config.features.insert("session_restore".into(), true);
        config.features.insert("disabled_feature".into(), false);
        assert!(config.is_enabled("session_restore"));
        assert!(!config.is_enabled("disabled_feature"));
    }

    #[test]
    fn roundtrip_serialize_deserialize() {
        let mut config = JcAppConfig {
            attribution: AttributionMode::NoAttribution,
            vim_mode: true,
            features: HashMap::new(),
        };
        config.features.insert("session_restore".into(), true);

        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: JcAppConfig = toml::from_str(&toml_str).unwrap();

        assert_eq!(parsed.attribution, AttributionMode::NoAttribution);
        assert!(parsed.vim_mode);
        assert!(parsed.is_enabled("session_restore"));
    }

    #[test]
    fn forward_compatible_with_unknown_keys() {
        let toml_str = r#"
            vim_mode = false
            future_setting = "hello"
            another_unknown = 42

            [features]
            session_restore = true
        "#;
        let config: JcAppConfig = toml::from_str(toml_str).unwrap();
        assert!(!config.vim_mode);
        assert!(config.is_enabled("session_restore"));
    }

    #[test]
    fn empty_toml_deserializes_to_default() {
        let config: JcAppConfig = toml::from_str("").unwrap();
        assert_eq!(config.attribution, AttributionMode::CoAuthor);
        assert!(!config.vim_mode);
        assert!(config.features.is_empty());
    }

    #[test]
    fn attribution_variants_serialize_in_config() {
        for (mode, expected) in [
            (AttributionMode::CoAuthor, "co_author"),
            (AttributionMode::Author, "author"),
            (AttributionMode::NoAttribution, "no_attribution"),
        ] {
            let config = JcAppConfig { attribution: mode, ..Default::default() };
            let toml_str = toml::to_string_pretty(&config).unwrap();
            assert!(
                toml_str.contains(expected),
                "expected {expected} in:\n{toml_str}"
            );
        }
    }
}
