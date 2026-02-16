use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

/// Represents the current charging state of the battery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChargingState {
    Charging,
    Discharging,
    Full,
    NotCharging,
    Unknown,
}

impl std::fmt::Display for ChargingState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChargingState::Charging => write!(f, "Charging"),
            ChargingState::Discharging => write!(f, "Discharging"),
            ChargingState::Full => write!(f, "Full"),
            ChargingState::NotCharging => write!(f, "Not Charging"),
            ChargingState::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Battery condition assessment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BatteryCondition {
    Normal,
    Replace,
    ServiceRecommended,
    Poor,
    Unknown,
}

impl std::fmt::Display for BatteryCondition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BatteryCondition::Normal => write!(f, "Normal"),
            BatteryCondition::Replace => write!(f, "Replace"),
            BatteryCondition::ServiceRecommended => write!(f, "Service Recommended"),
            BatteryCondition::Poor => write!(f, "Poor"),
            BatteryCondition::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Core battery information snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryInfo {
    pub level: u8,
    pub state: ChargingState,
    pub time_remaining_minutes: Option<i64>,
    pub power_draw_watts: Option<f64>,
    pub cycle_count: Option<u32>,
    pub max_capacity_mah: Option<u32>,
    pub design_capacity_mah: Option<u32>,
    pub current_capacity_mah: Option<u32>,
    pub temperature_celsius: Option<f64>,
    pub voltage_mv: Option<f64>,
    pub condition: BatteryCondition,
    pub manufacture_date: Option<String>,
    pub is_present: bool,
}

impl BatteryInfo {
    /// Calculate the health percentage (max_capacity / design_capacity * 100).
    pub fn health_percent(&self) -> Option<f64> {
        match (self.max_capacity_mah, self.design_capacity_mah) {
            (Some(max), Some(design)) if design > 0 => {
                Some(max as f64 / design as f64 * 100.0)
            }
            _ => None,
        }
    }

    /// Calculate the capacity loss in mAh.
    pub fn capacity_loss_mah(&self) -> Option<i32> {
        match (self.max_capacity_mah, self.design_capacity_mah) {
            (Some(max), Some(design)) => Some(design as i32 - max as i32),
            _ => None,
        }
    }

    /// Estimate remaining cycles based on typical 1000 cycle lifespan.
    pub fn estimated_remaining_cycles(&self) -> Option<u32> {
        self.cycle_count.map(|c| 1000u32.saturating_sub(c))
    }

    /// Get time remaining as a formatted string.
    pub fn time_remaining_display(&self) -> String {
        match self.time_remaining_minutes {
            Some(mins) if mins >= 0 => {
                let hours = mins / 60;
                let remaining_mins = mins % 60;
                if hours > 0 {
                    format!("{}h {:02}m", hours, remaining_mins)
                } else {
                    format!("{}m", remaining_mins)
                }
            }
            _ => "Calculating...".to_string(),
        }
    }
}

/// Reads battery information from the current platform.
pub fn get_battery_info() -> Result<BatteryInfo> {
    if cfg!(target_os = "macos") {
        get_battery_info_macos()
    } else if cfg!(target_os = "linux") {
        get_battery_info_linux()
    } else {
        anyhow::bail!("Unsupported platform. batteryctl supports macOS and Linux.")
    }
}

// ── Linux implementation ───────────────────────────────────────────────

fn get_battery_info_linux() -> Result<BatteryInfo> {
    let base = find_linux_battery_path()
        .context("No battery found. Are you on a laptop?")?;

    let level = read_sysfs_u32(&base.join("capacity")).unwrap_or(0) as u8;

    let status_str = read_sysfs_string(&base.join("status")).unwrap_or_default();
    let state = match status_str.trim().to_lowercase().as_str() {
        "charging" => ChargingState::Charging,
        "discharging" => ChargingState::Discharging,
        "full" => ChargingState::Full,
        "not charging" => ChargingState::NotCharging,
        _ => ChargingState::Unknown,
    };

    // Capacities: Linux reports in µAh or µWh depending on the driver
    let energy_full = read_sysfs_u32(&base.join("energy_full"));
    let energy_full_design = read_sysfs_u32(&base.join("energy_full_design"));
    let energy_now = read_sysfs_u32(&base.join("energy_now"));
    let charge_full = read_sysfs_u32(&base.join("charge_full"));
    let charge_full_design = read_sysfs_u32(&base.join("charge_full_design"));
    let charge_now = read_sysfs_u32(&base.join("charge_now"));

    // Use charge_* (µAh) if available, else energy_* (µWh)
    let (max_cap, design_cap, current_cap) = if charge_full.is_some() {
        (
            charge_full.map(|v| v / 1000),        // µAh -> mAh
            charge_full_design.map(|v| v / 1000),
            charge_now.map(|v| v / 1000),
        )
    } else {
        (
            energy_full.map(|v| v / 1000),         // µWh -> mWh (approximate)
            energy_full_design.map(|v| v / 1000),
            energy_now.map(|v| v / 1000),
        )
    };

    let power_now = read_sysfs_u32(&base.join("power_now"))
        .map(|v| v as f64 / 1_000_000.0); // µW -> W
    let current_now = read_sysfs_u32(&base.join("current_now"));
    let voltage_now = read_sysfs_u32(&base.join("voltage_now"))
        .map(|v| v as f64 / 1000.0); // µV -> mV

    let power_draw = power_now.or_else(|| {
        match (current_now, voltage_now) {
            (Some(curr), Some(volt)) => Some(curr as f64 * volt / 1_000_000_000.0),
            _ => None,
        }
    });

    let cycle_count = read_sysfs_u32(&base.join("cycle_count"));

    let temperature = read_sysfs_u32(&base.join("temp"))
        .map(|v| v as f64 / 10.0); // tenths of degree C

    // Time remaining estimation
    let time_remaining = estimate_time_remaining_linux(
        &state, energy_now, energy_full, power_now,
    );

    let condition = determine_condition(max_cap, design_cap, cycle_count);

    let manufacture_date = read_sysfs_string(&base.join("manufacture_date")).ok();

    Ok(BatteryInfo {
        level,
        state,
        time_remaining_minutes: time_remaining,
        power_draw_watts: power_draw,
        cycle_count,
        max_capacity_mah: max_cap,
        design_capacity_mah: design_cap,
        current_capacity_mah: current_cap,
        temperature_celsius: temperature,
        voltage_mv: voltage_now,
        condition,
        manufacture_date,
        is_present: true,
    })
}

fn find_linux_battery_path() -> Option<std::path::PathBuf> {
    let power_supply = Path::new("/sys/class/power_supply");
    if !power_supply.exists() {
        return None;
    }

    let entries = std::fs::read_dir(power_supply).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        let type_file = path.join("type");
        if let Ok(ptype) = std::fs::read_to_string(&type_file) {
            if ptype.trim().eq_ignore_ascii_case("battery") {
                return Some(path);
            }
        }
    }
    None
}

fn read_sysfs_string(path: &Path) -> Result<String> {
    std::fs::read_to_string(path)
        .map(|s| s.trim().to_string())
        .with_context(|| format!("Failed to read {}", path.display()))
}

fn read_sysfs_u32(path: &Path) -> Option<u32> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| s.trim().parse::<u32>().ok())
}

fn estimate_time_remaining_linux(
    state: &ChargingState,
    energy_now: Option<u32>,
    energy_full: Option<u32>,
    power_now: Option<f64>,
) -> Option<i64> {
    let power = power_now.filter(|&p| p > 0.1)?;

    match state {
        ChargingState::Discharging => {
            let energy = energy_now? as f64 / 1_000_000.0; // µWh -> Wh
            Some((energy / power * 60.0) as i64)
        }
        ChargingState::Charging => {
            let energy = energy_now? as f64 / 1_000_000.0;
            let full = energy_full? as f64 / 1_000_000.0;
            let remaining = full - energy;
            if remaining > 0.0 {
                Some((remaining / power * 60.0) as i64)
            } else {
                Some(0)
            }
        }
        _ => None,
    }
}

// ── macOS implementation ───────────────────────────────────────────────

fn get_battery_info_macos() -> Result<BatteryInfo> {
    let pmset_output = Command::new("pmset")
        .args(["-g", "batt"])
        .output()
        .context("Failed to run pmset")?;
    let pmset_str = String::from_utf8_lossy(&pmset_output.stdout);

    let profiler_output = Command::new("system_profiler")
        .args(["SPPowerDataType"])
        .output()
        .context("Failed to run system_profiler")?;
    let profiler_str = String::from_utf8_lossy(&profiler_output.stdout);

    parse_macos_battery(&pmset_str, &profiler_str)
}

fn parse_macos_battery(pmset: &str, profiler: &str) -> Result<BatteryInfo> {
    // Parse pmset output: e.g. "InternalBattery-0 (id=...)  87%; charging; 1:23 remaining"
    let mut level: u8 = 0;
    let mut state = ChargingState::Unknown;
    let mut time_remaining_minutes: Option<i64> = None;

    for line in pmset.lines() {
        let line_lower = line.to_lowercase();
        if line_lower.contains("internalbattery") || line_lower.contains("%") {
            // Extract percentage
            if let Some(pct) = extract_number_before(line, '%') {
                level = pct.min(100) as u8;
            }

            if line_lower.contains("charging") && !line_lower.contains("discharging") && !line_lower.contains("not charging") {
                state = ChargingState::Charging;
            } else if line_lower.contains("discharging") {
                state = ChargingState::Discharging;
            } else if line_lower.contains("charged") || line_lower.contains("finishing charge") {
                state = ChargingState::Full;
            } else if line_lower.contains("not charging") {
                state = ChargingState::NotCharging;
            }

            // Parse time remaining "H:MM remaining"
            if let Some(time_str) = extract_time_remaining(line) {
                time_remaining_minutes = Some(time_str);
            }
        }
    }

    // Parse system_profiler output
    let cycle_count = extract_profiler_value(profiler, "Cycle Count")
        .and_then(|v| v.parse::<u32>().ok());
    let max_capacity = extract_profiler_value(profiler, "Maximum Capacity")
        .and_then(|v| v.replace('%', "").trim().parse::<u32>().ok());
    let design_capacity_mah = extract_profiler_value(profiler, "Design Capacity")
        .and_then(|v| v.replace("mAh", "").trim().parse::<u32>().ok());
    let full_charge_capacity = extract_profiler_value(profiler, "Full Charge Capacity")
        .and_then(|v| v.replace("mAh", "").trim().parse::<u32>().ok());

    let condition_str = extract_profiler_value(profiler, "Condition")
        .unwrap_or_default();
    let condition = match condition_str.to_lowercase().as_str() {
        "normal" => BatteryCondition::Normal,
        "replace soon" | "replace now" | "replace" => BatteryCondition::Replace,
        "service" | "service recommended" => BatteryCondition::ServiceRecommended,
        _ => {
            if let Some(pct) = max_capacity {
                if pct >= 80 {
                    BatteryCondition::Normal
                } else if pct >= 60 {
                    BatteryCondition::ServiceRecommended
                } else {
                    BatteryCondition::Replace
                }
            } else {
                BatteryCondition::Unknown
            }
        }
    };

    let max_capacity_mah = full_charge_capacity.or(design_capacity_mah.map(|d| {
        max_capacity.map_or(d, |pct| (d as f64 * pct as f64 / 100.0) as u32)
    }));

    let manufacture_date = extract_profiler_value(profiler, "Manufacture Date");

    Ok(BatteryInfo {
        level,
        state,
        time_remaining_minutes,
        power_draw_watts: None,
        cycle_count,
        max_capacity_mah,
        design_capacity_mah,
        current_capacity_mah: None,
        temperature_celsius: None,
        voltage_mv: None,
        condition,
        manufacture_date,
        is_present: true,
    })
}

fn extract_number_before(s: &str, delimiter: char) -> Option<u32> {
    let idx = s.find(delimiter)?;
    let before = &s[..idx];
    // Walk backwards to find the start of the number
    let num_str: String = before.chars().rev()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .chars().rev()
        .collect();
    num_str.parse().ok()
}

fn extract_time_remaining(line: &str) -> Option<i64> {
    // Pattern: "H:MM remaining" or "(no estimate)"
    if line.contains("(no estimate)") || line.contains("not charging") {
        return None;
    }
    let parts: Vec<&str> = line.split_whitespace().collect();
    for (i, part) in parts.iter().enumerate() {
        if part.contains(':') && i + 1 < parts.len() && parts[i + 1] == "remaining" {
            let time_parts: Vec<&str> = part.split(':').collect();
            if time_parts.len() == 2 {
                let hours: i64 = time_parts[0].parse().ok()?;
                let mins: i64 = time_parts[1].parse().ok()?;
                return Some(hours * 60 + mins);
            }
        }
    }
    None
}

fn extract_profiler_value(profiler: &str, key: &str) -> Option<String> {
    for line in profiler.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(key) {
            if let Some((_k, v)) = trimmed.split_once(':') {
                return Some(v.trim().to_string());
            }
        }
    }
    None
}

fn determine_condition(
    max_cap: Option<u32>,
    design_cap: Option<u32>,
    cycle_count: Option<u32>,
) -> BatteryCondition {
    if let (Some(max), Some(design)) = (max_cap, design_cap) {
        if design == 0 {
            return BatteryCondition::Unknown;
        }
        let health = max as f64 / design as f64 * 100.0;
        if health >= 80.0 {
            return BatteryCondition::Normal;
        } else if health >= 60.0 {
            return BatteryCondition::ServiceRecommended;
        } else {
            return BatteryCondition::Replace;
        }
    }
    if let Some(cycles) = cycle_count {
        if cycles < 800 {
            return BatteryCondition::Normal;
        } else if cycles < 1000 {
            return BatteryCondition::ServiceRecommended;
        } else {
            return BatteryCondition::Replace;
        }
    }
    BatteryCondition::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_number_before() {
        assert_eq!(extract_number_before("87%;", '%'), Some(87));
        assert_eq!(extract_number_before("  100%", '%'), Some(100));
        assert_eq!(extract_number_before("abc%", '%'), None);
    }

    #[test]
    fn test_extract_time_remaining() {
        assert_eq!(
            extract_time_remaining("InternalBattery 87%; charging; 1:23 remaining"),
            Some(83)
        );
        assert_eq!(
            extract_time_remaining("InternalBattery 87%; (no estimate)"),
            None
        );
    }

    #[test]
    fn test_battery_info_health() {
        let info = BatteryInfo {
            level: 87,
            state: ChargingState::Charging,
            time_remaining_minutes: Some(83),
            power_draw_watts: Some(15.2),
            cycle_count: Some(47),
            max_capacity_mah: Some(4215),
            design_capacity_mah: Some(4500),
            current_capacity_mah: Some(3667),
            temperature_celsius: Some(32.0),
            voltage_mv: Some(12400.0),
            condition: BatteryCondition::Normal,
            manufacture_date: Some("2024-03-15".to_string()),
            is_present: true,
        };

        let health = info.health_percent().unwrap();
        assert!((health - 93.67).abs() < 0.1);
        assert_eq!(info.capacity_loss_mah(), Some(285));
        assert_eq!(info.estimated_remaining_cycles(), Some(953));
        assert_eq!(info.time_remaining_display(), "1h 23m");
    }

    #[test]
    fn test_determine_condition() {
        assert_eq!(
            determine_condition(Some(4500), Some(5000), None),
            BatteryCondition::Normal
        );
        assert_eq!(
            determine_condition(Some(3500), Some(5000), None),
            BatteryCondition::ServiceRecommended
        );
        assert_eq!(
            determine_condition(Some(2000), Some(5000), None),
            BatteryCondition::Replace
        );
    }

    #[test]
    fn test_parse_macos_battery() {
        let pmset = r#"Now drawing from 'AC Power'
 -InternalBattery-0 (id=1234567)	87%; charging; 1:23 remaining present: true"#;
        let profiler = r#"Battery Information:
      Cycle Count: 47
      Condition: Normal
      Full Charge Capacity (mAh): 4215
      Design Capacity (mAh): 4500"#;

        let info = parse_macos_battery(pmset, profiler).unwrap();
        assert_eq!(info.level, 87);
        assert_eq!(info.state, ChargingState::Charging);
        assert_eq!(info.time_remaining_minutes, Some(83));
        assert_eq!(info.cycle_count, Some(47));
    }
}
