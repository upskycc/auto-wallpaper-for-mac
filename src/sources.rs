use std::path::{Path, PathBuf};

use crate::config::{Config, Source};
use crate::paths::expand_user;

const IMAGE_EXTS: &[&str] = &["jpg", "jpeg", "png", "webp", "gif", "heic", "tif", "tiff"];

#[derive(Debug)]
pub enum WallpaperRef {
    Url(String),
    File(PathBuf),
}

pub fn collect_items(config: &Config) -> Vec<WallpaperRef> {
    let mut items = Vec::new();
    for source in &config.sources {
        match source {
            Source { url: Some(url), path: None } => items.push(WallpaperRef::Url(url.clone())),
            Source { url: None, path: Some(path) } => expand_path(&expand_user(path), &mut items),
            _ => {}
        }
    }
    items
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
}
