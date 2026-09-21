use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::paths::{cache_dir, ensure_parent};

const MAX_BYTES: usize = 20 * 1024 * 1024;

pub fn cached_or_download(url: &str, timeout_secs: u64) -> Result<PathBuf, String> {
    let dest = cache_path(url);
    if !dest.exists() {
        download(url, &dest, timeout_secs)?;
    }
    Ok(dest)
}

pub fn keep_only(path: &Path) {
    prune_dir(&cache_dir(), path);
}

fn prune_dir(dir: &Path, keep: &Path) {
    let Ok(dir) = dir.canonicalize() else {
        return;
    };
    let Ok(keep) = keep.canonicalize() else {
        return;
    };
    if !keep.starts_with(&dir) {
        return;
    }
    let Ok(entries) = fs::read_dir(&dir) else {
        return;
    };
    for entry in entries.flatten() {
        let candidate = entry.path();
        let is_keep = candidate.canonicalize().ok().as_ref() == Some(&keep);
        if !is_keep {
            let _ = fs::remove_file(candidate);
        }
    }
}

fn cache_path(url: &str) -> PathBuf {
    let name = format!("{:016x}", fnv1a64(url.as_bytes()));
    cache_dir().join(format!("{name}.img"))
}

fn download(url: &str, dest: &Path, timeout_secs: u64) -> Result<(), String> {
    ensure_parent(dest).map_err(|err| err.to_string())?;
    let agent = ureq::builder()
        .timeout_connect(Duration::from_secs(timeout_secs))
        .timeout_read(Duration::from_secs(timeout_secs))
        .user_agent("wallflow/1.0")
        .build();
    let response = agent.get(url).call().map_err(|err| format!("下载失败: {err}"))?;
    if !(200..300).contains(&response.status()) {
        return Err(format!("下载失败，状态码 {}", response.status()));
    }
    let reader = response.into_reader();
    let mut buf = Vec::new();
    reader
        .take((MAX_BYTES as u64) + 1)
        .read_to_end(&mut buf)
        .map_err(|err| format!("读取图片失败: {err}"))?;
    if buf.len() > MAX_BYTES {
        return Err("图片超过 20MB 限制".into());
    }
    if !is_image(&buf) {
        return Err("链接不是受支持的图片格式".into());
    }
    let tmp = dest.with_extension("tmp");
    fs::write(&tmp, &buf).map_err(|err| format!("写入缓存失败: {err}"))?;
    fs::rename(&tmp, dest).map_err(|err| format!("保存缓存失败: {err}"))?;
    Ok(())
}

fn is_image(buffer: &[u8]) -> bool {
    (buffer.len() >= 3 && buffer[0] == 0xff && buffer[1] == 0xd8 && buffer[2] == 0xff)
        || (buffer.len() >= 8 && buffer.starts_with(b"\x89PNG"))
        || (buffer.len() >= 6 && buffer.starts_with(b"GIF"))
        || (buffer.len() >= 12 && buffer.starts_with(b"RIFF") && &buffer[8..12] == b"WEBP")
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_stable() {
        assert_eq!(fnv1a64(b"abc"), fnv1a64(b"abc"));
        assert_ne!(fnv1a64(b"abc"), fnv1a64(b"abd"));
    }

    #[test]
    fn sniffs_jpeg() {
        assert!(is_image(&[0xff, 0xd8, 0xff, 0x00]));
        assert!(!is_image(&[0x00, 0x01]));
    }

    #[test]
    fn keep_only_deletes_other_files() {
        let dir = std::env::temp_dir().join(format!("wallflow-cache-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let keep = dir.join("keep.img");
        let old = dir.join("old.img");
        fs::write(&keep, b"keep").unwrap();
        fs::write(&old, b"old").unwrap();
        prune_dir(&dir, &keep);
        assert!(keep.exists());
        assert!(!old.exists());
        let _ = fs::remove_file(&keep);
        let _ = fs::remove_dir(&dir);
    }

    #[test]
    fn prune_ignores_files_outside_cache() {
        let dir = std::env::temp_dir().join(format!("wallflow-cache-out-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let cached = dir.join("cached.img");
        let outside = std::env::temp_dir().join(format!("wallflow-outside-{}", std::process::id()));
        fs::write(&cached, b"cached").unwrap();
        fs::write(&outside, b"outside").unwrap();
        prune_dir(&dir, &outside);
        assert!(cached.exists());
        let _ = fs::remove_file(&cached);
        let _ = fs::remove_file(&outside);
        let _ = fs::remove_dir(&dir);
    }
}
