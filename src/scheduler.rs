use std::thread;
use std::time::{Duration, Instant};

use crate::cache::cached_or_download;
use crate::config::{Config, RotateMode};
use crate::log;
use crate::power;
use crate::sources::{collect_items, next_index, WallpaperRef};
use crate::state::RotateState;
use crate::wallpaper::apply_desktop;

pub fn run_loop(config_path: &std::path::Path) {
    let mut last_rotate: Option<Instant> = None;
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
                        thread::sleep(Duration::from_secs(60));
                    }
                    continue;
                }
                let interval = Duration::from_secs(config.interval_secs());
                let due = last_rotate
                    .map(|started| started.elapsed() >= interval)
                    .unwrap_or(true);
                if due {
                    rotate_once(&config);
                    last_rotate = Some(Instant::now());
                }
                let remaining = last_rotate
                    .map(|started| interval.saturating_sub(started.elapsed()))
                    .unwrap_or(interval);
                sleep_with_leeway(remaining.as_secs().max(1));
            }
            Err(err) => {
                log::warn(&err);
                thread::sleep(Duration::from_secs(60));
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
    let result = resolve_file(item, config.download_timeout_secs).and_then(|path| {
        apply_desktop(&path, &config.apply_to)?;
        if matches!(item, WallpaperRef::Url(_)) {
            crate::cache::keep_only(&path);
        }
        Ok(())
    });
    state.last_index = Some(index);
    state.save();
    match result {
        Ok(()) => log::info(&format!("已更换壁纸: {}", display_item(item))),
        Err(err) => log::warn(&err),
    }
}

fn resolve_file(item: &WallpaperRef, timeout_secs: u64) -> Result<std::path::PathBuf, String> {
    match item {
        WallpaperRef::File(path) => Ok(path.clone()),
        WallpaperRef::Url(url) => cached_or_download(url, timeout_secs),
    }
}

fn display_item(item: &WallpaperRef) -> String {
    match item {
        WallpaperRef::Url(url) => url.clone(),
        WallpaperRef::File(path) => path.display().to_string(),
    }
}

fn sleep_with_leeway(secs: u64) {
    let leeway = (secs / 8).clamp(15, 180);
    thread::sleep(Duration::from_secs(secs.saturating_add(leeway / 2)));
}
