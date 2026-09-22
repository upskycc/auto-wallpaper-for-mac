use std::fs;
use std::path::Path;
use std::process::Command;

use crate::config::write_example;
use crate::paths::{default_config_path, ensure_parent, installed_binary_path, launch_agent_path, support_dir};

pub fn install() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|err| err.to_string())?;
    let dest = installed_binary_path();
    ensure_parent(&dest).map_err(|err| err.to_string())?;
    fs::copy(&exe, &dest).map_err(|err| format!("复制二进制失败: {err}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&dest).map_err(|err| err.to_string())?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&dest, perms).map_err(|err| err.to_string())?;
    }

    let config = default_config_path();
    if !config.exists() {
        write_example(&config)?;
    }

    let plist_path = launch_agent_path();
    write_plist(&plist_path, &dest, &config)?;
    reload_agent(&plist_path)?;
    crate::log::info(&format!(
        "已安装 LaunchAgent: {} ，配置: {}",
        plist_path.display(),
        config.display()
    ));
    Ok(())
}

pub fn uninstall() -> Result<(), String> {
    let plist_path = launch_agent_path();
    unload_agent();
    remove_if_exists(&plist_path)?;
    let support = support_dir();
    if support.exists() {
        fs::remove_dir_all(&support).map_err(|err| format!("删除程序目录失败: {err}"))?;
    }
    println!("已卸载 wallflow");
    Ok(())
}

fn remove_if_exists(path: &Path) -> Result<(), String> {
    if path.exists() {
        fs::remove_file(path).map_err(|err| format!("删除 {} 失败: {err}", path.display()))?;
    }
    Ok(())
}

fn write_plist(path: &Path, binary: &Path, config: &Path) -> Result<(), String> {
    ensure_parent(path).map_err(|err| err.to_string())?;
    let body = include_str!("../macos/com.wallflow.plist")
        .replace("__SUPPORT_DIR__", &xml_escape(&support_dir().display().to_string()))
        .replace("__BINARY__", &xml_escape(&binary.display().to_string()))
        .replace("__CONFIG__", &xml_escape(&config.display().to_string()));
    fs::write(path, body).map_err(|err| format!("写入 LaunchAgent 失败: {err}"))
}

fn reload_agent(path: &Path) -> Result<(), String> {
    if cfg!(not(target_os = "macos")) {
        crate::log::warn("当前不是 macOS，已写入 plist，未执行 launchctl");
        return Ok(());
    }
    unload_agent();
    let status = Command::new("launchctl")
        .args(["bootstrap", &format!("gui/{}", uid()), &path.display().to_string()])
        .output()
        .map_err(|err| format!("launchctl 失败: {err}"))?;
    if status.status.success() {
        return Ok(());
    }
    let fallback = Command::new("launchctl")
        .args(["load", "-w", &path.display().to_string()])
        .status()
        .map_err(|err| format!("launchctl 失败: {err}"))?;
    if fallback.success() {
        Ok(())
    } else {
        Err("launchctl bootstrap 失败".into())
    }
}

fn unload_agent() {
    if cfg!(not(target_os = "macos")) {
        return;
    }
    let domain = format!("gui/{}", uid());
    let target = format!("{domain}/com.wallflow");
    let _ = Command::new("launchctl")
        .args(["bootout", &target])
        .output();
}

fn uid() -> u32 {
    Command::new("id")
        .arg("-u")
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .and_then(|text| text.trim().parse().ok())
        .unwrap_or(501)
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
