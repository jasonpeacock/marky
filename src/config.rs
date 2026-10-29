use crate::{document::IncludeLevel, paths};
use anyhow::{anyhow, Result};

/// Defaults read from `config.toml` in the config directory.
/// Command line flags take precedence over these values.
#[derive(serde::Deserialize, Debug, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub theme: Option<String>,
    pub highlight: bool,
    pub math: bool,
    pub diagrams: bool,
    pub include_images: Option<IncludeLevel>,
    pub optimize_images: bool,
    pub port: u16,
    pub open: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            theme: None,
            highlight: false,
            math: false,
            diagrams: false,
            include_images: None,
            optimize_images: false,
            port: 8080,
            open: false,
        }
    }
}

pub fn load() -> Result<Config> {
    let path = paths::files::config();
    if !path.exists() {
        return Ok(Config::default());
    }

    let contents = std::fs::read_to_string(&path)
        .map_err(|e| anyhow!("reading {}: {}", path.display(), e))?;

    toml::from_str(&contents).map_err(|e| anyhow!("parsing {}: {}", path.display(), e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_file_uses_defaults() {
        assert_eq!(toml::from_str::<Config>("").unwrap(), Config::default());
    }

    #[test]
    fn documented_defaults_match() {
        // Keep in sync with the example in README.md.
        let documented = r#"
            theme = "sakura"
            highlight = false
            math = false
            diagrams = false
            # include_images = "local"
            optimize_images = false
            port = 8080
            open = false
        "#;

        assert_eq!(
            toml::from_str::<Config>(documented).unwrap(),
            Config {
                theme: Some("sakura".to_string()),
                ..Config::default()
            }
        );
    }

    #[test]
    fn parses_all_fields() {
        let config: Config = toml::from_str(
            r#"
            theme = "air"
            highlight = true
            math = true
            diagrams = true
            include_images = "remote"
            optimize_images = true
            port = 3000
            open = true
            "#,
        )
        .unwrap();

        assert_eq!(
            config,
            Config {
                theme: Some("air".to_string()),
                highlight: true,
                math: true,
                diagrams: true,
                include_images: Some(IncludeLevel::Remote),
                optimize_images: true,
                port: 3000,
                open: true,
            }
        );
    }

    #[test]
    fn rejects_unknown_fields() {
        assert!(toml::from_str::<Config>("maths = true").is_err());
    }
}
