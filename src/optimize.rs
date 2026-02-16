use crate::battery::BatteryInfo;
use crate::power::PowerReport;
use serde::{Deserialize, Serialize};
use sysinfo::System;

/// Priority level for optimization suggestions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    High,
    Medium,
    Low,
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Priority::High => write!(f, "High Impact"),
            Priority::Medium => write!(f, "Medium Impact"),
            Priority::Low => write!(f, "Low Impact"),
        }
    }
}

/// A single optimization suggestion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    pub priority: Priority,
    pub title: String,
    pub description: String,
    pub estimated_savings: Option<String>,
}

/// Overall optimization report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationReport {
    pub suggestions: Vec<Suggestion>,
    pub estimated_total_savings_minutes: Option<i64>,
}

impl OptimizationReport {
    pub fn high_impact(&self) -> Vec<&Suggestion> {
        self.suggestions
            .iter()
            .filter(|s| s.priority == Priority::High)
            .collect()
    }

    pub fn medium_impact(&self) -> Vec<&Suggestion> {
        self.suggestions
            .iter()
            .filter(|s| s.priority == Priority::Medium)
            .collect()
    }

    pub fn low_impact(&self) -> Vec<&Suggestion> {
        self.suggestions
            .iter()
            .filter(|s| s.priority == Priority::Low)
            .collect()
    }
}

/// Generate optimization suggestions based on current state.
pub fn generate_suggestions(
    battery: &BatteryInfo,
    power: Option<&PowerReport>,
    aggressive: bool,
) -> OptimizationReport {
    let mut suggestions = Vec::new();
    let mut total_savings_mins: i64 = 0;

    // ── High Impact: Power-hungry apps ──────────────────────────────────
    if let Some(report) = power {
        for app in report.apps.iter().take(3) {
            if app.cpu_percent > 20.0 {
                let savings = format!("saves ~{:.1}W", app.estimated_power_watts * 0.7);
                suggestions.push(Suggestion {
                    priority: Priority::High,
                    title: format!("{} is using {:.0}% CPU", app.name, app.cpu_percent),
                    description: format!(
                        "Close unused instances or switch to a lighter alternative"
                    ),
                    estimated_savings: Some(savings),
                });
                total_savings_mins += 15;
            }
        }
    }

    // ── High Impact: Brightness (generic suggestion) ────────────────────
    suggestions.push(Suggestion {
        priority: Priority::High,
        title: "Reduce display brightness".to_string(),
        description: "Lower brightness to 50-60% for significant power savings".to_string(),
        estimated_savings: Some("saves ~2W".to_string()),
    });
    total_savings_mins += 30;

    // ── Medium Impact: Background processes ─────────────────────────────
    let sys = System::new_all();
    let process_count = sys.processes().len();
    if process_count > 100 {
        suggestions.push(Suggestion {
            priority: Priority::Medium,
            title: format!("{} processes running", process_count),
            description: "Close unused applications to reduce background power drain".to_string(),
            estimated_savings: Some("saves ~0.5-1W".to_string()),
        });
        total_savings_mins += 15;
    }

    // ── Medium Impact: Battery level advice ─────────────────────────────
    if battery.level > 80 && matches!(battery.state, crate::battery::ChargingState::Charging) {
        suggestions.push(Suggestion {
            priority: Priority::Medium,
            title: "Unplug charger to preserve battery health".to_string(),
            description: "Keeping battery between 20-80% extends its lifespan".to_string(),
            estimated_savings: None,
        });
    }

    if battery.level < 20 && !matches!(battery.state, crate::battery::ChargingState::Charging) {
        suggestions.push(Suggestion {
            priority: Priority::High,
            title: "Battery critically low".to_string(),
            description: "Connect to power source soon to avoid unexpected shutdown".to_string(),
            estimated_savings: None,
        });
    }

    // ── Medium Impact: Temperature ──────────────────────────────────────
    if let Some(temp) = battery.temperature_celsius {
        if temp > 40.0 {
            suggestions.push(Suggestion {
                priority: Priority::Medium,
                title: format!("Battery temperature high ({:.0}C)", temp),
                description: "Move to a cooler environment or reduce workload. High temperature degrades battery health.".to_string(),
                estimated_savings: None,
            });
        }
    }

    // ── Aggressive mode extras ──────────────────────────────────────────
    if aggressive {
        suggestions.push(Suggestion {
            priority: Priority::Medium,
            title: "Disable Bluetooth if not in use".to_string(),
            description: "Bluetooth radio consumes power even when idle".to_string(),
            estimated_savings: Some("saves ~0.3W".to_string()),
        });
        total_savings_mins += 10;

        suggestions.push(Suggestion {
            priority: Priority::Medium,
            title: "Disable Wi-Fi if not needed".to_string(),
            description: "Use airplane mode for offline work to save power".to_string(),
            estimated_savings: Some("saves ~0.5W".to_string()),
        });
        total_savings_mins += 15;

        suggestions.push(Suggestion {
            priority: Priority::Low,
            title: "Turn off keyboard backlight".to_string(),
            description: "Every bit helps when maximizing battery life".to_string(),
            estimated_savings: Some("saves ~0.1W".to_string()),
        });
        total_savings_mins += 5;
    }

    // ── Low Impact: General tips ────────────────────────────────────────
    suggestions.push(Suggestion {
        priority: Priority::Low,
        title: "Enable Low Power Mode when below 20%".to_string(),
        description: "System-level power optimizations extend remaining time".to_string(),
        estimated_savings: Some("saves ~10-15%".to_string()),
    });

    suggestions.push(Suggestion {
        priority: Priority::Low,
        title: "Keep system updated".to_string(),
        description: "OS updates often include power management improvements".to_string(),
        estimated_savings: None,
    });

    // Sort by priority
    suggestions.sort_by_key(|s| s.priority);

    OptimizationReport {
        suggestions,
        estimated_total_savings_minutes: Some(total_savings_mins),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battery::{BatteryCondition, ChargingState};

    #[test]
    fn test_generate_suggestions() {
        let info = BatteryInfo {
            level: 50,
            state: ChargingState::Discharging,
            time_remaining_minutes: Some(120),
            power_draw_watts: Some(10.0),
            cycle_count: Some(100),
            max_capacity_mah: Some(4000),
            design_capacity_mah: Some(4500),
            current_capacity_mah: None,
            temperature_celsius: Some(35.0),
            voltage_mv: None,
            condition: BatteryCondition::Normal,
            manufacture_date: None,
            is_present: true,
        };

        let report = generate_suggestions(&info, None, false);
        assert!(!report.suggestions.is_empty());
    }

    #[test]
    fn test_aggressive_mode_adds_suggestions() {
        let info = BatteryInfo {
            level: 50,
            state: ChargingState::Discharging,
            time_remaining_minutes: None,
            power_draw_watts: None,
            cycle_count: None,
            max_capacity_mah: None,
            design_capacity_mah: None,
            current_capacity_mah: None,
            temperature_celsius: None,
            voltage_mv: None,
            condition: BatteryCondition::Unknown,
            manufacture_date: None,
            is_present: true,
        };

        let normal = generate_suggestions(&info, None, false);
        let aggressive = generate_suggestions(&info, None, true);
        assert!(aggressive.suggestions.len() > normal.suggestions.len());
    }
}
