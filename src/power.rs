use anyhow::Result;
use serde::{Deserialize, Serialize};
use sysinfo::System;

/// Per-process power consumption estimate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessPowerInfo {
    pub name: String,
    pub pid: u32,
    pub cpu_percent: f32,
    pub memory_mb: f64,
    pub estimated_power_watts: f64,
}

/// Aggregated power consumption by application name.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppPowerInfo {
    pub name: String,
    pub cpu_percent: f32,
    pub memory_mb: f64,
    pub estimated_power_watts: f64,
    pub process_count: usize,
}

/// Overall power consumption report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerReport {
    pub apps: Vec<AppPowerInfo>,
    pub total_cpu_percent: f32,
    pub total_estimated_watts: f64,
    pub system_power_draw: Option<f64>,
}

impl PowerReport {
    /// Calculate the percentage of total power for each app.
    pub fn with_percentages(&self) -> Vec<(String, f64, f64)> {
        let total = self.total_estimated_watts.max(0.001);
        self.apps
            .iter()
            .map(|app| {
                let pct = app.estimated_power_watts / total * 100.0;
                (app.name.clone(), app.estimated_power_watts, pct)
            })
            .collect()
    }
}

/// Estimate per-process power based on CPU usage.
///
/// This is an approximation: we assume total system TDP and distribute
/// power proportionally to CPU usage. On macOS with `powermetrics`,
/// more accurate data could be obtained (requires sudo).
const ESTIMATED_TDP_WATTS: f64 = 30.0;

pub fn get_power_report(system_power_draw: Option<f64>) -> Result<PowerReport> {
    let mut sys = System::new_all();
    // Refresh twice with a small delay for accurate CPU measurements
    sys.refresh_all();
    std::thread::sleep(std::time::Duration::from_millis(500));
    sys.refresh_all();

    let tdp = system_power_draw.unwrap_or(ESTIMATED_TDP_WATTS);

    // Collect per-process data
    let mut processes: Vec<ProcessPowerInfo> = Vec::new();
    let mut total_cpu: f32 = 0.0;

    for (pid, process) in sys.processes() {
        let cpu = process.cpu_usage();
        if cpu < 0.1 {
            continue;
        }
        let memory_mb = process.memory() as f64 / (1024.0 * 1024.0);
        let name = process.name().to_string_lossy().to_string();

        total_cpu += cpu;
        processes.push(ProcessPowerInfo {
            name,
            pid: pid.as_u32(),
            cpu_percent: cpu,
            memory_mb,
            estimated_power_watts: 0.0, // calculated below
        });
    }

    // Distribute power proportionally to CPU usage
    let cpu_factor = if total_cpu > 0.0 {
        tdp / total_cpu as f64
    } else {
        0.0
    };

    for proc in &mut processes {
        proc.estimated_power_watts = proc.cpu_percent as f64 * cpu_factor;
    }

    // Aggregate by application name
    let mut app_map: std::collections::HashMap<String, AppPowerInfo> =
        std::collections::HashMap::new();

    for proc in &processes {
        let entry = app_map.entry(proc.name.clone()).or_insert(AppPowerInfo {
            name: proc.name.clone(),
            cpu_percent: 0.0,
            memory_mb: 0.0,
            estimated_power_watts: 0.0,
            process_count: 0,
        });
        entry.cpu_percent += proc.cpu_percent;
        entry.memory_mb += proc.memory_mb;
        entry.estimated_power_watts += proc.estimated_power_watts;
        entry.process_count += 1;
    }

    let mut apps: Vec<AppPowerInfo> = app_map.into_values().collect();
    apps.sort_by(|a, b| {
        b.estimated_power_watts
            .partial_cmp(&a.estimated_power_watts)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let total_estimated = apps.iter().map(|a| a.estimated_power_watts).sum();

    Ok(PowerReport {
        apps,
        total_cpu_percent: total_cpu,
        total_estimated_watts: total_estimated,
        system_power_draw,
    })
}

/// Get power report filtered by application name.
pub fn get_power_report_filtered(
    filter: &str,
    system_power_draw: Option<f64>,
) -> Result<PowerReport> {
    let mut report = get_power_report(system_power_draw)?;
    let filter_lower = filter.to_lowercase();
    report.apps.retain(|app| {
        app.name.to_lowercase().contains(&filter_lower)
    });
    Ok(report)
}

/// Try to get the actual system power draw from platform-specific sources.
pub fn get_system_power_draw() -> Option<f64> {
    if cfg!(target_os = "linux") {
        get_linux_power_draw()
    } else if cfg!(target_os = "macos") {
        get_macos_power_draw()
    } else {
        None
    }
}

fn get_linux_power_draw() -> Option<f64> {
    // Try reading from power_supply
    let base = std::path::Path::new("/sys/class/power_supply");
    if !base.exists() {
        return None;
    }
    for entry in std::fs::read_dir(base).ok()?.flatten() {
        let path = entry.path();
        let type_file = path.join("type");
        if let Ok(ptype) = std::fs::read_to_string(&type_file) {
            if ptype.trim().eq_ignore_ascii_case("battery") {
                if let Ok(power) = std::fs::read_to_string(path.join("power_now")) {
                    if let Ok(val) = power.trim().parse::<f64>() {
                        return Some(val / 1_000_000.0); // µW -> W
                    }
                }
            }
        }
    }
    None
}

fn get_macos_power_draw() -> Option<f64> {
    // On macOS, we could parse `pmset -g rawlog` or use IOKit, but that's complex.
    // For now, return None and rely on the TDP estimation.
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power_report_percentages() {
        let report = PowerReport {
            apps: vec![
                AppPowerInfo {
                    name: "Chrome".to_string(),
                    cpu_percent: 34.0,
                    memory_mb: 500.0,
                    estimated_power_watts: 10.0,
                    process_count: 5,
                },
                AppPowerInfo {
                    name: "Code".to_string(),
                    cpu_percent: 17.0,
                    memory_mb: 300.0,
                    estimated_power_watts: 5.0,
                    process_count: 2,
                },
            ],
            total_cpu_percent: 51.0,
            total_estimated_watts: 15.0,
            system_power_draw: None,
        };

        let pcts = report.with_percentages();
        assert_eq!(pcts.len(), 2);
        assert!((pcts[0].2 - 66.67).abs() < 0.1);
        assert!((pcts[1].2 - 33.33).abs() < 0.1);
    }
}
