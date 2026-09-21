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
pub struct Source {
    pub url: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, Deserialize)]
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
    pub sources: Vec<Source>,
}

fn default_true() -> bool {
    true
}

fn default_timeout() -> u64 {
    15
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
        for (index, source) in self.sources.iter().enumerate() {
            match (&source.url, &source.path) {
                (Some(url), None) => {
                    if !(url.starts_with("http://") || url.starts_with("https://")) {
                        return Err(format!("sources[{index}].url 必须是 http 或 https"));
                    }
                }
                (None, Some(path)) if !path.trim().is_empty() => {}
                _ => return Err(format!("sources[{index}] 需要填写 url 或 path 其中一项")),
            }
        }
        Ok(())
    }

    pub fn interval_secs(&self) -> u64 {
        self.interval_minutes.saturating_mul(60)
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
        assert_eq!(config.interval_minutes, 30);
        assert_eq!(config.sources.len(), 3);
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
        assert_eq!(config.interval_secs(), 1800);
        assert!(config.pause_on_battery);
    }
}
