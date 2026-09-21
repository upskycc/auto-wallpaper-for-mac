use std::fs;

use crate::paths::{ensure_parent, state_path};

pub struct RotateState {
    pub last_index: Option<usize>,
}

impl RotateState {
    pub fn load() -> Self {
        let raw = fs::read_to_string(state_path()).unwrap_or_default();
        let last_index = raw.trim().parse().ok();
        Self { last_index }
    }

    pub fn save(&self) {
        let path = state_path();
        let _ = ensure_parent(&path);
        if let Some(index) = self.last_index {
            let _ = fs::write(path, index.to_string());
        }
    }
}
