use std::fs;
use std::path::Path;
use std::process::Command;

use serde_json::{json, Value};

use crate::config::ApplyTo;
use crate::paths::home_dir;

pub fn apply_wallpaper(path: &Path, apply_to: &ApplyTo) -> Result<(), String> {
    apply_desktop(path, apply_to)?;
    if let Err(err) = apply_lock_screen(path) {
        crate::log::warn(&format!("锁屏壁纸未更新: {err}"));
    }
    Ok(())
}

pub fn apply_desktop(path: &Path, apply_to: &ApplyTo) -> Result<(), String> {
    let posix = posix_escaped(path)?;
    let target = match apply_to {
        ApplyTo::All => "every desktop",
        ApplyTo::Main => "desktop 1",
    };
    let script = format!(
        "tell application \"System Events\"\n  tell {target}\n    set picture to POSIX file \"{posix}\"\n  end tell\nend tell"
    );
    run_osascript(&script)
}

fn apply_lock_screen(path: &Path) -> Result<(), String> {
    if cfg!(not(target_os = "macos")) {
        crate::log::info("当前不是 macOS，跳过锁屏壁纸");
        return Ok(());
    }
    let index = wallpaper_index_path();
    if !index.exists() {
        crate::log::info("未找到 Wallpaper Index.plist，锁屏随系统默认处理");
        return Ok(());
    }
    let url = file_url(path)?;
    let raw = plutil_json(&index)?;
    let mut value: Value =
        serde_json::from_str(&raw).map_err(|err| format!("解析 Index.plist 失败: {err}"))?;
    patch_image_nodes(&mut value, &url);
    write_index_plist(&index, &value)?;
    let _ = Command::new("killall").arg("WallpaperAgent").output();
    Ok(())
}

fn wallpaper_index_path() -> std::path::PathBuf {
    home_dir().join("Library/Application Support/com.apple.wallpaper/Store/Index.plist")
}

fn posix_escaped(path: &Path) -> Result<String, String> {
    Ok(path
        .to_str()
        .ok_or_else(|| "壁纸路径无效".to_string())?
        .replace('\\', "\\\\")
        .replace('"', "\\\""))
}

fn file_url(path: &Path) -> Result<String, String> {
    let abs = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let text = abs.to_str().ok_or_else(|| "壁纸路径无效".to_string())?;
    Ok(format!("file://{}", encode_path(text)))
}

fn encode_path(path: &str) -> String {
    path.split('/')
        .map(|segment| {
            let mut out = String::new();
            for byte in segment.bytes() {
                match byte {
                    b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                        out.push(byte as char);
                    }
                    _ => out.push_str(&format!("%{byte:02X}")),
                }
            }
            out
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn image_choice(file_url: &str) -> Value {
    json!({
        "Provider": "com.apple.wallpaper.choice.image",
        "URL": file_url,
        "Files": [{ "relative": file_url }]
    })
}

fn set_image_content(node: &mut Value, file_url: &str) {
    let Some(obj) = node.as_object_mut() else {
        return;
    };
    let content = obj.entry("Content").or_insert_with(|| json!({}));
    if let Some(content) = content.as_object_mut() {
        content.insert("Choices".into(), json!([image_choice(file_url)]));
        content.remove("Shuffle");
    }
}

fn patch_image_nodes(value: &mut Value, file_url: &str) {
    match value {
        Value::Object(map) => {
            for (key, child) in map.iter_mut() {
                if key == "Desktop" || key == "Idle" || key == "LockScreen" {
                    set_image_content(child, file_url);
                } else {
                    patch_image_nodes(child, file_url);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                patch_image_nodes(item, file_url);
            }
        }
        _ => {}
    }
}

fn plutil_json(path: &Path) -> Result<String, String> {
    let output = Command::new("plutil")
        .args(["-convert", "json", "-o", "-", "--"])
        .arg(path)
        .output()
        .map_err(|err| format!("无法执行 plutil: {err}"))?;
    if !output.status.success() {
        return Err(format!(
            "读取 Index.plist 失败: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    String::from_utf8(output.stdout).map_err(|err| format!("Index.plist 不是合法 UTF-8: {err}"))
}

fn write_index_plist(path: &Path, value: &Value) -> Result<(), String> {
    let tmp = path.with_extension("plist.wallflow");
    let json = serde_json::to_vec(value).map_err(|err| format!("序列化 Index.plist 失败: {err}"))?;
    fs::write(&tmp, json).map_err(|err| format!("写入 Index.plist 失败: {err}"))?;
    let output = Command::new("plutil")
        .args(["-convert", "binary1"])
        .arg(&tmp)
        .output()
        .map_err(|err| format!("无法执行 plutil: {err}"))?;
    if !output.status.success() {
        let _ = fs::remove_file(&tmp);
        return Err(format!(
            "转换 Index.plist 失败: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    fs::rename(&tmp, path).map_err(|err| {
        let _ = fs::remove_file(&tmp);
        format!("更新 Index.plist 失败: {err}")
    })
}

fn run_osascript(script: &str) -> Result<(), String> {
    if cfg!(not(target_os = "macos")) {
        crate::log::info("当前不是 macOS，跳过真实设壁纸");
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patches_idle_and_desktop_choices() {
        let mut value = json!({
            "Displays": {
                "ABC": {
                    "Desktop": {
                        "Content": {
                            "Choices": [{ "Provider": "old" }],
                            "Shuffle": { "Type": "once" }
                        }
                    },
                    "Idle": {
                        "Content": {
                            "Choices": [{ "Provider": "old-idle" }]
                        }
                    }
                }
            },
            "SystemDefault": {
                "Idle": { "Content": { "Choices": [] } },
                "LockScreen": { "Content": { "Choices": [] } }
            }
        });
        patch_image_nodes(&mut value, "file:///tmp/wall.jpg");
        let desktop = &value["Displays"]["ABC"]["Desktop"]["Content"];
        assert_eq!(
            desktop["Choices"][0]["Provider"],
            "com.apple.wallpaper.choice.image"
        );
        assert_eq!(desktop["Choices"][0]["URL"], "file:///tmp/wall.jpg");
        assert!(desktop.get("Shuffle").is_none());
        assert_eq!(
            value["Displays"]["ABC"]["Idle"]["Content"]["Choices"][0]["Files"][0]["relative"],
            "file:///tmp/wall.jpg"
        );
        assert_eq!(
            value["SystemDefault"]["Idle"]["Content"]["Choices"][0]["URL"],
            "file:///tmp/wall.jpg"
        );
        assert_eq!(
            value["SystemDefault"]["LockScreen"]["Content"]["Choices"][0]["URL"],
            "file:///tmp/wall.jpg"
        );
    }

    #[test]
    fn encodes_file_url_spaces() {
        assert_eq!(encode_path("/Users/a b/x.jpg"), "/Users/a%20b/x.jpg");
    }
}
