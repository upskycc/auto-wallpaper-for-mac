use std::fs;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use crate::cache::cached_or_download;
use crate::config::{Config, RotateMode};
use crate::log;
use crate::power;
use crate::sources::{collect_items, next_index, WallpaperRef};
use crate::state::RotateState;
use crate::wallpaper::apply_desktop;

pub fn run_loop(config_path: &Path) {
    let mut last_rotate: Option<Instant> = None;
    let mut last_mtime = config_mtime(config_path);
    loop {
        match Config::load(config_path) {
            Ok(config) => {
                let power = power::current();
                if !power.allow_work(config.pause_when_display_off, config.pause_on_battery) {
                    if config.pause_when_display_off && !power.display_on {
                        power::wait_until_display_on();
                    } else if config.pause_on_battery && (power.on_battery || power.low_power) {
                        power::wait_until_ac_power();
                    }
                    if !power::current().allow_work(config.pause_when_display_off, config.pause_on_battery) {
                        wait_for_interval_or_config_change(config_path, last_mtime, 60);
                    }
                    continue;
                }
                let mtime = config_mtime(config_path);
                let config_changed = mtime != last_mtime;
                last_mtime = mtime;
                let interval = Duration::from_secs(config.interval_secs());
                let due = last_rotate
                    .map(|started| started.elapsed() >= interval)
                    .unwrap_or(true)
                    || config_changed;
                if due {
                    rotate_once(&config);
                    last_rotate = Some(Instant::now());
                }
                let remaining = last_rotate
                    .map(|started| interval.saturating_sub(started.elapsed()))
                    .unwrap_or(interval);
                if wait_for_interval_or_config_change(
                    config_path,
                    last_mtime,
                    remaining.as_secs().max(1),
                ) {
                    continue;
                }
            }
            Err(err) => {
                log::warn(&err);
                wait_for_interval_or_config_change(config_path, last_mtime, 60);
            }
        }
    }
}

pub fn rotate_once(config: &Config) {
    let items = collect_items(config);
    let mut state = RotateState::load();
    let random = matches!(config.mode, RotateMode::Random);
    let Some(index) = next_index(items.len(), state.last_index, random) else {
        log::warn("没有可用壁纸来源");
        return;
    };
    let item = &items[index];
    let result = resolve_file(item, config.download_timeout_secs, random).and_then(|(path, label)| {
        apply_desktop(&path, &config.apply_to)?;
        if matches!(item, WallpaperRef::Url(_) | WallpaperRef::JsonApi { .. }) {
            crate::cache::keep_only(&path);
        }
        Ok(label)
    });
    state.last_index = Some(index);
    state.save();
    match result {
        Ok(label) => log::info(&format!("已更换壁纸: {label}")),
        Err(err) => log::warn(&err),
    }
}

fn resolve_file(
    item: &WallpaperRef,
    timeout_secs: u64,
    random: bool,
) -> Result<(std::path::PathBuf, String), String> {
    match item {
        WallpaperRef::File(path) => Ok((path.clone(), path.display().to_string())),
        WallpaperRef::Url(url) => {
            let path = cached_or_download(url, timeout_secs)?;
            Ok((path, url.clone()))
        }
        WallpaperRef::JsonApi { url, json_path } => {
            let raw = crate::cache::fetch_json(url, timeout_secs)?;
            let image_url = crate::sources::pick_json_image_url(&raw, json_path, url, random)?;
            let path = cached_or_download(&image_url, timeout_secs)?;
            Ok((path, image_url))
        }
    }
}

fn config_mtime(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).and_then(|meta| meta.modified()).ok()
}

fn wait_for_interval_or_config_change(path: &Path, known: Option<SystemTime>, secs: u64) -> bool {
    let deadline = Instant::now() + Duration::from_secs(secs);
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return false;
        }
        thread::sleep(remaining.min(Duration::from_secs(5)));
        if config_mtime(path) != known {
            thread::sleep(Duration::from_millis(200));
            return true;
        }
    }
}
