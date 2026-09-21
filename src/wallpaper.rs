use std::path::Path;
use std::process::Command;

use crate::config::ApplyTo;

pub fn apply_desktop(path: &Path, apply_to: &ApplyTo) -> Result<(), String> {
    let posix = path
        .to_str()
        .ok_or_else(|| "壁纸路径无效".to_string())?
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    let target = match apply_to {
        ApplyTo::All => "every desktop",
        ApplyTo::Main => "desktop 1",
    };
    let script = format!(
        "tell application \"System Events\"\n  tell {target}\n    set picture to POSIX file \"{posix}\"\n  end tell\nend tell"
    );
    run_osascript(&script)
}

fn run_osascript(script: &str) -> Result<(), String> {
    if cfg!(not(target_os = "macos")) {
        crate::log::warn("当前不是 macOS，跳过真实设壁纸");
        return Ok(());
    }
    let output = Command::new("osascript")
        .args(["-e", script])
        .output()
        .map_err(|err| format!("无法执行 osascript: {err}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "设置桌面失败: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}
