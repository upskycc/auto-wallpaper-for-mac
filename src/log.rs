use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::io::Write;

static ENABLED: AtomicBool = AtomicBool::new(false);
static SINK: Mutex<Option<Sink>> = Mutex::new(None);

struct Sink {
    path: Option<PathBuf>,
    file: Option<File>,
}

pub fn configure(enabled: bool, file: Option<&Path>) {
    ENABLED.store(enabled, Ordering::Relaxed);
    let desired = if enabled {
        file.map(Path::to_path_buf)
    } else {
        None
    };
    let mut sink = SINK.lock().unwrap();
    let unchanged = matches!(sink.as_ref(), Some(current) if current.path == desired);
    if unchanged {
        return;
    }
    let opened = desired.as_ref().and_then(|path| open_append(path).ok());
    *sink = Some(Sink {
        path: desired,
        file: opened,
    });
}

pub fn info(message: &str) {
    emit(message);
}

pub fn warn(message: &str) {
    emit(message);
}

fn emit(message: &str) {
    if !ENABLED.load(Ordering::Relaxed) {
        return;
    }
    let line = format!("{} {message}", timestamp());
    let mut sink = SINK.lock().unwrap();
    if let Some(file) = sink.as_mut().and_then(|sink| sink.file.as_mut()) {
        let _ = writeln!(file, "{line}");
        let _ = file.flush();
        return;
    }
    eprintln!("{line}");
}

fn timestamp() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as libc::time_t)
        .unwrap_or(0);
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    let ok = unsafe { !libc::localtime_r(&secs, &mut tm).is_null() };
    if !ok {
        return "0000-00-00 00:00:00".into();
    }
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        tm.tm_year + 1900,
        tm.tm_mon + 1,
        tm.tm_mday,
        tm.tm_hour,
        tm.tm_min,
        tm.tm_sec
    )
}

fn open_append(path: &Path) -> std::io::Result<File> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    OpenOptions::new().create(true).append(true).open(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_off() {
        assert!(!ENABLED.load(Ordering::Relaxed));
    }

    #[test]
    fn writes_to_configured_file() {
        let path = std::env::temp_dir().join(format!("wallflow-log-{}.log", std::process::id()));
        let _ = std::fs::remove_file(&path);
        configure(true, Some(&path));
        info("hello");
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("hello"));
        configure(false, None);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn timestamp_has_expected_shape() {
        let stamp = timestamp();
        assert_eq!(stamp.len(), 19);
        assert_eq!(&stamp[4..5], "-");
        assert_eq!(&stamp[10..11], " ");
        assert_eq!(&stamp[13..14], ":");
        assert_eq!(&stamp[16..17], ":");
        assert!(stamp.chars().enumerate().all(|(index, ch)| {
            matches!(index, 4 | 7 | 10 | 13 | 16) || ch.is_ascii_digit()
        }));
    }
}
