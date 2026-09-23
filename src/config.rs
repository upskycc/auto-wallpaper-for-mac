use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

use crate::paths::{default_config_path, ensure_parent};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RotateMode {
    Sequential,
    Random,
}

impl Default for RotateMode {
    fn default() -> Self {
        Self::Sequential
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApplyTo {
    All,
    Main,
}

impl Default for ApplyTo {
    fn default() -> Self {
        Self::All
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub url: Option<String>,
    pub path: Option<String>,
    #[serde(default)]
    pub json_path: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub interval_minutes: u64,
    #[serde(default)]
    pub mode: RotateMode,
    #[serde(default)]
    pub apply_to: ApplyTo,
    #[serde(default = "default_true")]
    pub pause_when_display_off: bool,
    #[serde(default = "default_true")]
    pub pause_on_battery: bool,
    #[serde(default = "default_timeout")]
    pub download_timeout_secs: u64,
    #[serde(default)]
    pub log_enabled: bool,
    #[serde(default)]
    pub log_file: Option<String>,
    pub sources: Vec<Source>,
}

fn default_true() -> bool {
    true
}

fn default_timeout() -> u64 {
    15
}

fn validate_json_path(index: usize, json_path: &str) -> Result<(), String> {
    let json_path = json_path.trim();
    if json_path.is_empty() {
        return Err(format!("sources[{index}].json_path 不能为空"));
    }
    if json_path.starts_with('.') || json_path.ends_with('.') || json_path.contains("..") {
        return Err(format!("sources[{index}].json_path 格式错误"));
    }
    Ok(())
}

impl Config {
    pub fn load(path: &Path) -> Result<Self, String> {
        let raw = fs::read_to_string(path).map_err(|err| format!("读取配置失败: {err}"))?;
        let config: Config = toml::from_str(&raw).map_err(|err| format!("配置格式错误: {err}"))?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.interval_minutes == 0 || self.interval_minutes > 1440 {
            return Err("interval_minutes 必须是 1 到 1440".into());
        }
        if self.download_timeout_secs == 0 || self.download_timeout_secs > 120 {
            return Err("download_timeout_secs 必须是 1 到 120".into());
        }
        if self.sources.is_empty() {
            return Err("至少需要一个 [[sources]]".into());
        }
        if let Some(log_file) = &self.log_file {
            if log_file.trim().is_empty() {
                return Err("log_file 不能为空，不需要写文件就删掉这一项".into());
            }
        }
        for (index, source) in self.sources.iter().enumerate() {
            match (&source.url, &source.path, source.json_path.as_deref()) {
                (Some(url), None, json_path) => {
                    if !(url.starts_with("http://") || url.starts_with("https://")) {
                        return Err(format!("sources[{index}].url 必须是 http 或 https"));
                    }
                    if let Some(json_path) = json_path {
                        validate_json_path(index, json_path)?;
                    }
                }
                (None, Some(path), None) if !path.trim().is_empty() => {}
                (None, Some(_), Some(_)) => {
                    return Err(format!("sources[{index}].json_path 只能和 url 一起使用"));
                }
                _ => return Err(format!("sources[{index}] 需要填写 url 或 path 其中一项")),
            }
        }
        Ok(())
    }

    pub fn interval_secs(&self) -> u64 {
        self.interval_minutes.saturating_mul(60)
    }

    pub fn log_target(&self) -> Option<PathBuf> {
        self.log_file.as_deref().map(crate::paths::expand_user)
    }
}

pub fn write_example(path: &Path) -> Result<(), String> {
    ensure_parent(path).map_err(|err| err.to_string())?;
    fs::write(path, EXAMPLE_CONFIG).map_err(|err| format!("写入示例配置失败: {err}"))
}

pub fn resolve_config_path(cli: Option<PathBuf>) -> PathBuf {
    cli.unwrap_or_else(default_config_path)
}

pub const EXAMPLE_CONFIG: &str = include_str!("../config.example.toml");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example_config_parses() {
        let config: Config = toml::from_str(EXAMPLE_CONFIG).unwrap();
        config.validate().unwrap();
        assert_eq!(config.interval_minutes, 1);
        assert!(config.log_enabled);
        assert_eq!(
            config.log_file.as_deref(),
            Some("~/Library/Logs/wallflow.log")
        );
        assert_eq!(config.sources.len(), 2);
    }

    #[test]
    fn rejects_empty_sources() {
        let raw = r#"
interval_minutes = 10
sources = []
"#;
        let config: Config = toml::from_str(raw).unwrap();
        assert!(config.validate().is_err());
    }

    #[test]
    fn interval_is_minutes() {
        let config: Config = toml::from_str(EXAMPLE_CONFIG).unwrap();
        assert_eq!(config.interval_secs(), 60);
        assert!(config.pause_on_battery);
    }

    #[test]
    fn json_path_with_url_is_valid() {
        let raw = r#"
interval_minutes = 10
[[sources]]
url = "https://wp.upx8.com/api.php?format=json"
json_path = "data.url"
"#;
        let config: Config = toml::from_str(raw).unwrap();
        config.validate().unwrap();
        assert_eq!(config.sources[0].json_path.as_deref(), Some("data.url"));
    }

    #[test]
    fn json_path_cannot_pair_with_path() {
        let raw = r#"
interval_minutes = 10
[[sources]]
path = "~/Pictures"
json_path = "data.url"
"#;
        let config: Config = toml::from_str(raw).unwrap();
        assert!(config.validate().is_err());
    }

    #[test]
    fn log_enabled_defaults_off() {
        let raw = r#"
interval_minutes = 10
[[sources]]
url = "https://example.com/a.jpg"
"#;
        let config: Config = toml::from_str(raw).unwrap();
        assert!(!config.log_enabled);
        assert_eq!(config.log_file, None);
    }

    #[test]
    fn log_file_accepts_path() {
        let raw = r#"
interval_minutes = 10
log_enabled = true
log_file = "~/Library/Logs/wallflow.log"
[[sources]]
url = "https://example.com/a.jpg"
"#;
        let config: Config = toml::from_str(raw).unwrap();
        config.validate().unwrap();
        assert_eq!(
            config.log_file.as_deref(),
            Some("~/Library/Logs/wallflow.log")
        );
    }

    #[test]
    fn rejects_keys_placed_after_sources() {
        let raw = r#"
interval_minutes = 10

[[sources]]
url = "https://example.com/a.jpg"

log_enabled = true
"#;
        assert!(toml::from_str::<Config>(raw).is_err());
    }

    #[test]
    fn log_file_rejects_empty() {
        let raw = r#"
interval_minutes = 10
log_file = "  "
[[sources]]
url = "https://example.com/a.jpg"
"#;
        let config: Config = toml::from_str(raw).unwrap();
        assert!(config.validate().is_err());
    }

    #[test]
    fn json_path_rejects_empty() {
        let raw = r#"
interval_minutes = 10
[[sources]]
url = "https://example.com/api"
json_path = "  "
"#;
        let config: Config = toml::from_str(raw).unwrap();
        assert!(config.validate().is_err());
    }
}
