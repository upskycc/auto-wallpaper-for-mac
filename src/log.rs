use std::sync::atomic::{AtomicBool, Ordering};

static ENABLED: AtomicBool = AtomicBool::new(false);

pub fn set_enabled(enabled: bool) {
    ENABLED.store(enabled, Ordering::Relaxed);
}

pub fn info(message: &str) {
    if ENABLED.load(Ordering::Relaxed) {
        eprintln!("{message}");
    }
}

pub fn warn(message: &str) {
    if ENABLED.load(Ordering::Relaxed) {
        eprintln!("{message}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_off() {
        assert!(!ENABLED.load(Ordering::Relaxed));
    }
}
