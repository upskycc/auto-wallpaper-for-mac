use std::path::{Path, PathBuf};

use crate::config::{Config, Source};
use crate::paths::expand_user;

const IMAGE_EXTS: &[&str] = &["jpg", "jpeg", "png", "webp", "gif", "heic", "tif", "tiff"];

#[derive(Debug)]
pub enum WallpaperRef {
    Url(String),
    JsonApi { url: String, json_path: String },
    File(PathBuf),
}

pub fn collect_items(config: &Config) -> Vec<WallpaperRef> {
    let mut items = Vec::new();
    for source in &config.sources {
        match source {
            Source {
                url: Some(url),
                path: None,
                json_path: Some(json_path),
            } => items.push(WallpaperRef::JsonApi {
                url: url.clone(),
                json_path: json_path.clone(),
            }),
            Source {
                url: Some(url),
                path: None,
                json_path: None,
            } => items.push(WallpaperRef::Url(url.clone())),
            Source {
                url: None,
                path: Some(path),
                json_path: None,
            } => expand_path(&expand_user(path), &mut items),
            _ => {}
        }
    }
    items
}

pub fn pick_json_image_url(
    raw: &str,
    json_path: &str,
    base_url: &str,
    random: bool,
) -> Result<String, String> {
    let value: serde_json::Value =
        serde_json::from_str(raw).map_err(|err| format!("JSON 解析失败: {err}"))?;
    let mut urls = extract_image_urls(&value, json_path);
    urls.retain(|item| !item.trim().is_empty());
    if urls.is_empty() {
        return Err(format!("json_path `{json_path}` 没有解析到图片地址"));
    }
    let chosen = if random && urls.len() > 1 {
        urls.swap_remove(simple_random(urls.len()))
    } else {
        urls.remove(0)
    };
    absolutize(base_url, &chosen)
        .ok_or_else(|| format!("解析到的地址不是 http/https: {chosen}"))
}

fn extract_image_urls(value: &serde_json::Value, json_path: &str) -> Vec<String> {
    let segments: Vec<&str> = json_path.split('.').filter(|part| !part.is_empty()).collect();
    collect_urls(value, &segments)
}

fn collect_urls(value: &serde_json::Value, segments: &[&str]) -> Vec<String> {
    if segments.is_empty() {
        return match value {
            serde_json::Value::String(text) => vec![text.clone()],
            serde_json::Value::Array(items) => items
                .iter()
                .flat_map(|item| collect_urls(item, &[]))
                .collect(),
            _ => Vec::new(),
        };
    }
    match value {
        serde_json::Value::Object(map) => map
            .get(segments[0])
            .map(|next| collect_urls(next, &segments[1..]))
            .unwrap_or_default(),
        serde_json::Value::Array(items) => items
            .iter()
            .flat_map(|item| collect_urls(item, segments))
            .collect(),
        _ => Vec::new(),
    }
}

fn absolutize(base: &str, href: &str) -> Option<String> {
    let href = href.trim();
    if href.starts_with("https://") || href.starts_with("http://") {
        return Some(href.to_string());
    }
    if href.starts_with("//") {
        let scheme = if base.starts_with("https://") {
            "https:"
        } else {
            "http:"
        };
        return Some(format!("{scheme}{href}"));
    }
    let origin = origin_of(base)?;
    if href.starts_with('/') {
        return Some(format!("{origin}{href}"));
    }
    if href.is_empty() {
        return None;
    }
    let slash = base.find('?').unwrap_or(base.len());
    let prefix = &base[..slash];
    let dir_end = prefix.rfind('/').filter(|index| *index >= origin.len())?;
    Some(format!("{}{href}", &base[..=dir_end]))
}

fn origin_of(base: &str) -> Option<&str> {
    let rest = base.split_once("://")?.1;
    let host_len = rest.find('/').unwrap_or(rest.len());
    Some(&base[..base.len() - (rest.len() - host_len)])
}

fn expand_path(path: &Path, items: &mut Vec<WallpaperRef>) {
    if path.is_file() {
        if is_image(path) {
            items.push(WallpaperRef::File(path.to_path_buf()));
        }
        return;
    }
    let Ok(entries) = std::fs::read_dir(path) else {
        return;
    };
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && is_image(path))
        .collect();
    files.sort();
    items.extend(files.into_iter().map(WallpaperRef::File));
}

fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| IMAGE_EXTS.iter().any(|ok| ext.eq_ignore_ascii_case(ok)))
        .unwrap_or(false)
}

pub fn next_index(len: usize, current: Option<usize>, random: bool) -> Option<usize> {
    if len == 0 {
        return None;
    }
    if random {
        return Some(simple_random(len));
    }
    Some(current.map(|index| (index + 1) % len).unwrap_or(0))
}

fn simple_random(len: usize) -> usize {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as usize)
        .unwrap_or(0);
    nanos % len
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequential_wraps() {
        assert_eq!(next_index(3, None, false), Some(0));
        assert_eq!(next_index(3, Some(0), false), Some(1));
        assert_eq!(next_index(3, Some(2), false), Some(0));
        assert_eq!(next_index(0, None, false), None);
    }

    #[test]
    fn json_path_reads_object_and_array() {
        let object = r#"{"code":200,"data":{"url":"http://cdn.example.com/a.jpg"}}"#;
        assert_eq!(
            pick_json_image_url(object, "data.url", "https://wp.upx8.com/api.php", false).unwrap(),
            "http://cdn.example.com/a.jpg"
        );

        let array = r#"{"data":[{"url":"https://cdn.example.com/a.jpg"},{"url":"https://cdn.example.com/b.jpg"}]}"#;
        assert_eq!(
            pick_json_image_url(array, "data.url", "https://wp.upx8.com/api.php", false).unwrap(),
            "https://cdn.example.com/a.jpg"
        );

        let strings = r#"{"data":["https://cdn.example.com/a.jpg","https://cdn.example.com/b.jpg"]}"#;
        assert_eq!(
            pick_json_image_url(strings, "data", "https://example.com/api", false).unwrap(),
            "https://cdn.example.com/a.jpg"
        );
    }

    #[test]
    fn json_path_joins_relative_url() {
        let raw = r#"{"data":{"url":"/bizhi/a.jpg"}}"#;
        assert_eq!(
            pick_json_image_url(raw, "data.url", "https://wp.upx8.com/api.php?count=1", false)
                .unwrap(),
            "https://wp.upx8.com/bizhi/a.jpg"
        );
    }

    #[test]
    fn json_path_missing_returns_error() {
        let raw = r#"{"data":{"title":"x"}}"#;
        assert!(pick_json_image_url(raw, "data.url", "https://example.com/", false).is_err());
    }
}
