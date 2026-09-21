use std::path::{Path, PathBuf};

pub fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn support_dir() -> PathBuf {
    home_dir().join("Library/Application Support/wallflow")
}

pub fn default_config_path() -> PathBuf {
    support_dir().join("config.toml")
}

pub fn cache_dir() -> PathBuf {
    support_dir().join("cache")
}

pub fn state_path() -> PathBuf {
    support_dir().join("last_index")
}

pub fn log_path() -> PathBuf {
    home_dir().join("Library/Logs/wallflow.log")
}

pub fn launch_agent_path() -> PathBuf {
    home_dir().join("Library/LaunchAgents/com.wallflow.plist")
}

pub fn installed_binary_path() -> PathBuf {
    support_dir().join("wallflow")
}

pub fn expand_user(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        return home_dir().join(rest);
    }
    if path == "~" {
        return home_dir();
    }
    PathBuf::from(path)
}

pub fn ensure_parent(path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}
