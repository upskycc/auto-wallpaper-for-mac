use std::fs::OpenOptions;
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::paths::{ensure_parent, log_path};

pub fn info(message: &str) {
    write_line("INFO", message);
}

pub fn warn(message: &str) {
    write_line("WARN", message);
}

fn write_line(level: &str, message: &str) {
    let line = format!("{timestamp} [{level}] {message}\n", timestamp = now());
    let path = log_path();
    let _ = ensure_parent(&path);
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = file.write_all(line.as_bytes());
    }
    eprint!("{line}");
}

fn now() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}
