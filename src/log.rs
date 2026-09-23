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
    let mut sink = SINK.lock().unwrap();
    if let Some(file) = sink.as_mut().and_then(|sink| sink.file.as_mut()) {
        let _ = writeln!(file, "{message}");
        let _ = file.flush();
        return;
    }
    eprintln!("{message}");
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
}
