use std::process::Command;

#[derive(Clone, Copy)]
pub struct PowerState {
    pub display_on: bool,
    pub on_battery: bool,
    pub low_power: bool,
}

impl PowerState {
    pub fn allow_work(&self, pause_when_display_off: bool, pause_on_battery: bool) -> bool {
        if pause_when_display_off && !self.display_on {
            return false;
        }
        if pause_on_battery && (self.on_battery || self.low_power) {
            return false;
        }
        true
    }
}

pub fn wait_until_display_on() {
    #[cfg(target_os = "macos")]
    unsafe {
        wallflow_wait_until_display_on();
    }
}

pub fn wait_until_ac_power() {
    #[cfg(target_os = "macos")]
    unsafe {
        wallflow_wait_until_ac_power();
    }
}

#[cfg(target_os = "macos")]
extern "C" {
    fn wallflow_wait_until_display_on() -> i32;
    fn wallflow_wait_until_ac_power() -> i32;
}

pub fn current() -> PowerState {
    if cfg!(not(target_os = "macos")) {
        return PowerState {
            display_on: true,
            on_battery: false,
            low_power: false,
        };
    }
    PowerState {
        display_on: display_is_on(),
        on_battery: on_battery(),
        low_power: low_power_mode(),
    }
}

fn display_is_on() -> bool {
    let Ok(output) = Command::new("ioreg").args(["-n", "IODisplayWrangler", "-d", "1"]).output() else {
        return true;
    };
    let text = String::from_utf8_lossy(&output.stdout);
    if let Some(value) = extract_int(&text, "\"CurrentPowerState\"=") {
        return value >= 3;
    }
    true
}

fn on_battery() -> bool {
    let Ok(output) = Command::new("pmset").args(["-g", "batt"]).output() else {
        return false;
    };
    let text = String::from_utf8_lossy(&output.stdout).to_lowercase();
    text.contains("battery power")
}

fn low_power_mode() -> bool {
    let Ok(output) = Command::new("pmset").args(["-g"]).output() else {
        return false;
    };
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines().any(|line| {
        let line = line.trim();
        line.starts_with("lowpowermode") && line.contains('1')
    })
}

fn extract_int(haystack: &str, key: &str) -> Option<i32> {
    let start = haystack.find(key)? + key.len();
    haystack[start..]
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pauses_when_display_off() {
        let state = PowerState {
            display_on: false,
            on_battery: false,
            low_power: false,
        };
        assert!(!state.allow_work(true, true));
        assert!(state.allow_work(false, true));
    }

    #[test]
    fn pauses_on_battery() {
        let state = PowerState {
            display_on: true,
            on_battery: true,
            low_power: false,
        };
        assert!(!state.allow_work(true, true));
        assert!(state.allow_work(true, false));
    }
}
