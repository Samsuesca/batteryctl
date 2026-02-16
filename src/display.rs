use crate::battery::{BatteryCondition, BatteryInfo, ChargingState};
use crate::health::HealthReport;
use crate::history::{BatterySnapshot, HistorySummary};
use crate::optimize::OptimizationReport;
use crate::power::PowerReport;
use colored::Colorize;

// ── Battery Status Display ─────────────────────────────────────────────

pub fn print_status(info: &BatteryInfo, detailed: bool) {
    let width = 57;
    let border_top = format!("╭{}╮", "─".repeat(width));
    let border_mid = format!("├{}┤", "─".repeat(width));
    let border_bot = format!("╰{}╯", "─".repeat(width));

    println!("{}", border_top);
    println!(
        "│{:^width$}│",
        "BATTERY STATUS".bold(),
        width = width
    );
    println!("{}", border_mid);

    // Level with color
    let level_str = format!("{}%", info.level);
    let level_colored = match info.level {
        0..=15 => level_str.red().bold(),
        16..=30 => level_str.yellow(),
        31..=79 => level_str.green(),
        _ => level_str.bright_green().bold(),
    };
    println!("│ {:<20} {:>34} │", "Level:", level_colored);

    // State
    let state_str = match info.state {
        ChargingState::Charging => "Charging (AC Power)".green().to_string(),
        ChargingState::Discharging => "On Battery".yellow().to_string(),
        ChargingState::Full => "Full (AC Power)".bright_green().to_string(),
        ChargingState::NotCharging => "Not Charging".white().to_string(),
        ChargingState::Unknown => "Unknown".dimmed().to_string(),
    };
    println!("│ {:<20} {:>34} │", "State:", state_str);

    // Time remaining
    match info.state {
        ChargingState::Charging => {
            println!(
                "│ {:<20} {:>34} │",
                "Time to Full:",
                info.time_remaining_display()
            );
        }
        ChargingState::Discharging => {
            println!(
                "│ {:<20} {:>34} │",
                "Time Remaining:",
                info.time_remaining_display()
            );
        }
        _ => {}
    }

    // Power draw
    if let Some(power) = info.power_draw_watts {
        let power_str = if matches!(info.state, ChargingState::Charging) {
            format!("-{:.1}W (charging)", power)
        } else {
            format!("{:.1}W", power)
        };
        println!("│ {:<20} {:>34} │", "Power Draw:", power_str);
    }

    if detailed {
        println!("│ {:>55} │", "");

        // Health
        let condition_str = match info.condition {
            BatteryCondition::Normal => "Normal".green().to_string(),
            BatteryCondition::ServiceRecommended => "Service Recommended".yellow().to_string(),
            BatteryCondition::Replace => "Replace".red().to_string(),
            BatteryCondition::Poor => "Poor".red().bold().to_string(),
            BatteryCondition::Unknown => "Unknown".dimmed().to_string(),
        };
        println!("│ {:<20} {:>34} │", "Health:", condition_str);

        // Max capacity
        if let Some(health) = info.health_percent() {
            let cap_str = match (info.max_capacity_mah, info.design_capacity_mah) {
                (Some(max), Some(design)) => {
                    format!("{:.0}% ({} / {} mAh)", health, max, design)
                }
                _ => format!("{:.0}%", health),
            };
            println!("│ {:<20} {:>34} │", "Max Capacity:", cap_str);
        }

        // Cycle count
        if let Some(cycles) = info.cycle_count {
            let remaining = info.estimated_remaining_cycles().unwrap_or(0);
            println!(
                "│ {:<20} {:>34} │",
                "Cycle Count:",
                format!("{} / 1,000 (~{} remaining)", cycles, remaining)
            );
        }

        // Temperature
        if let Some(temp) = info.temperature_celsius {
            let temp_str = if temp > 40.0 {
                format!("{:.0}C", temp).red().to_string()
            } else {
                format!("{:.0}C", temp)
            };
            println!("│ {:<20} {:>34} │", "Temperature:", temp_str);
        }

        // Voltage
        if let Some(voltage) = info.voltage_mv {
            println!(
                "│ {:<20} {:>34} │",
                "Voltage:",
                format!("{:.1} mV", voltage)
            );
        }
    }

    println!("{}", border_bot);
}

// ── Health Report Display ──────────────────────────────────────────────

pub fn print_health_report(report: &HealthReport) {
    println!("{}", "Battery Health Report:".bold());
    println!();

    let width = 44;
    let border_top = format!("╭{}╮", "─".repeat(width));
    let border_bot = format!("╰{}╯", "─".repeat(width));

    println!("{}", border_top);

    if let Some(design) = report.design_capacity_mah {
        println!("│ {:<24} {:>17} │", "Design Capacity:", format!("{} mAh", design));
    }
    if let Some(max) = report.max_capacity_mah {
        println!("│ {:<24} {:>17} │", "Current Max:", format!("{} mAh", max));
    }
    if let (Some(loss), Some(pct)) = (report.capacity_loss_mah, report.capacity_loss_percent) {
        let loss_str = format!("-{} mAh (-{:.0}%)", loss, pct);
        println!("│ {:<24} {:>17} │", "Capacity Loss:", loss_str);
    }

    println!("│ {:>42} │", "");

    if let Some(cycles) = report.cycle_count {
        println!("│ {:<24} {:>17} │", "Cycle Count:", cycles);
    }
    if let Some(remaining) = report.estimated_remaining_cycles {
        println!(
            "│ {:<24} {:>17} │",
            "Est. Remaining:",
            format!("{} cycles", remaining)
        );
    }

    println!("│ {:>42} │", "");

    let condition_display = match report.condition.as_str() {
        "Normal" => "Normal".green().to_string(),
        "Service Recommended" => "Service Recommended".yellow().to_string(),
        "Replace" => "Replace".red().to_string(),
        _ => report.condition.clone(),
    };
    println!("│ {:<24} {:>17} │", "Condition:", condition_display);

    if let Some(ref date) = report.manufacture_date {
        println!("│ {:<24} {:>17} │", "Manufactured:", date);
    }
    if let Some(ref age) = report.age_description {
        println!("│ {:<24} {:>17} │", "Age:", age);
    }

    println!("{}", border_bot);

    // Capacity trend chart
    if !report.capacity_trend.is_empty() {
        println!();
        println!("{}", "Capacity Trend:".bold());
        print_simple_chart(&report.capacity_trend.iter().map(|p| p.health_percent).collect::<Vec<_>>());
    }
}

pub fn print_health_comparison(comparisons: &[(String, String, String)]) {
    println!();
    println!("{}", "Comparison with New Battery:".bold());
    println!(
        "  {:<20} {:<15} {:<15}",
        "Metric".underline(),
        "New".underline(),
        "Current".underline()
    );
    for (metric, new_val, current_val) in comparisons {
        println!("  {:<20} {:<15} {:<15}", metric, new_val, current_val);
    }
}

// ── Power Hogs Display ─────────────────────────────────────────────────

pub fn print_power_report(report: &PowerReport, detailed: bool) {
    println!("{}", "Top Power Consumers:".bold());
    println!();

    // Table header
    println!(
        "  {:<5} {:<26} {:<13} {:<10}",
        "#".bold(),
        "Application".bold(),
        "Est. Power".bold(),
        "% Total".bold()
    );
    println!("  {}", "─".repeat(55));

    let total = report.total_estimated_watts.max(0.001);
    let display_count = if detailed { report.apps.len().min(20) } else { report.apps.len().min(10) };
    let displayed_apps = &report.apps[..display_count];

    for (i, app) in displayed_apps.iter().enumerate() {
        let pct = app.estimated_power_watts / total * 100.0;
        let power_str = format!("{:.1} W", app.estimated_power_watts);
        let pct_str = format!("{:.1}%", pct);

        let name = if detailed && app.process_count > 1 {
            format!("{} ({})", app.name, app.process_count)
        } else {
            app.name.clone()
        };

        println!(
            "  {:<5} {:<26} {:<13} {:<10}",
            format!("{}", i + 1),
            truncate_str(&name, 25),
            power_str,
            pct_str
        );
    }

    // "Other" row if needed
    if report.apps.len() > display_count {
        let other_count = report.apps.len() - display_count;
        let other_power: f64 = report.apps[display_count..]
            .iter()
            .map(|a| a.estimated_power_watts)
            .sum();
        let other_pct = other_power / total * 100.0;
        println!(
            "  {:<5} {:<26} {:<13} {:<10}",
            "",
            format!("Other ({} apps)", other_count),
            format!("{:.1} W", other_power),
            format!("{:.1}%", other_pct)
        );
    }

    println!();
    println!(
        "  Total estimated consumption: {:.1} W",
        report.total_estimated_watts
    );
    if let Some(sys_power) = report.system_power_draw {
        println!("  System power draw: {:.1} W", sys_power);
    }
}

// ── History Display ────────────────────────────────────────────────────

pub fn print_history(
    snapshots: &[BatterySnapshot],
    summary: &HistorySummary,
) {
    println!(
        "{}",
        format!("Battery History ({}):", summary.period_description).bold()
    );
    println!();

    if snapshots.is_empty() {
        println!("  No history data available for this period.");
        println!("  Tip: Run 'batteryctl status' periodically to collect data,");
        println!("  or use 'batteryctl alert --daemon' for continuous monitoring.");
        return;
    }

    // Chart of battery level over time
    let levels: Vec<f64> = snapshots.iter().map(|s| s.level as f64).collect();
    print_simple_chart(&levels);

    println!();

    // Summary stats
    let charging_h = summary.total_charging_minutes / 60;
    let charging_m = summary.total_charging_minutes % 60;
    let discharging_h = summary.total_discharging_minutes / 60;
    let discharging_m = summary.total_discharging_minutes % 60;

    println!(
        "  Charging periods: {} (total: {}h {:02}m)",
        summary.charging_periods, charging_h, charging_m
    );
    println!(
        "  On battery: {}h {:02}m",
        discharging_h, discharging_m
    );
    if let Some(rate) = summary.avg_discharge_rate_watts {
        println!("  Avg discharge rate: {:.1} W", rate);
    }
    println!("  Cycles completed: {:.2} cycles", summary.estimated_cycles);
    println!("  Snapshots recorded: {}", summary.snapshots_count);
}

// ── Optimization Display ───────────────────────────────────────────────

pub fn print_optimization_report(report: &OptimizationReport) {
    println!("{}", "Battery Optimization Suggestions:".bold());
    println!();

    let high = report.high_impact();
    let medium = report.medium_impact();
    let low = report.low_impact();

    if !high.is_empty() {
        println!("{}", "High Impact (Immediate):".red().bold());
        for s in &high {
            println!("  {} {}", ">>".red(), s.title.bold());
            println!("     -> {}", s.description);
            if let Some(ref savings) = s.estimated_savings {
                println!("     ({})", savings.dimmed());
            }
        }
        println!();
    }

    if !medium.is_empty() {
        println!("{}", "Medium Impact:".yellow().bold());
        for s in &medium {
            println!("  {} {}", "->".yellow(), s.title);
            println!("     -> {}", s.description);
            if let Some(ref savings) = s.estimated_savings {
                println!("     ({})", savings.dimmed());
            }
        }
        println!();
    }

    if !low.is_empty() {
        println!("{}", "Low Impact:".blue());
        for s in &low {
            println!("  {} {}", "--".blue(), s.title);
            println!("     -> {}", s.description);
            if let Some(ref savings) = s.estimated_savings {
                println!("     ({})", savings.dimmed());
            }
        }
        println!();
    }

    if let Some(mins) = report.estimated_total_savings_minutes {
        if mins > 0 {
            let hours = mins / 60;
            let remaining_mins = mins % 60;
            println!(
                "Estimated savings: {}",
                format!("+{}h {:02}m battery life", hours, remaining_mins)
                    .green()
                    .bold()
            );
        }
    }
}

// ── Export Functions ────────────────────────────────────────────────────

pub fn export_snapshots_csv(
    snapshots: &[BatterySnapshot],
    path: &str,
) -> anyhow::Result<()> {
    let mut wtr = csv::Writer::from_path(path)?;
    wtr.write_record(["timestamp", "level", "is_charging", "power_draw", "cycle_count", "max_capacity"])?;

    for snap in snapshots {
        wtr.write_record(&[
            snap.timestamp.to_rfc3339(),
            snap.level.to_string(),
            snap.is_charging.to_string(),
            snap.power_draw.map_or("".to_string(), |p| format!("{:.2}", p)),
            snap.cycle_count.map_or("".to_string(), |c| c.to_string()),
            snap.max_capacity.map_or("".to_string(), |c| c.to_string()),
        ])?;
    }

    wtr.flush()?;
    println!("Exported {} snapshots to {}", snapshots.len(), path);
    Ok(())
}

pub fn export_json(info: &BatteryInfo) -> anyhow::Result<String> {
    Ok(serde_json::to_string_pretty(info)?)
}

// ── Helper Functions ───────────────────────────────────────────────────

fn print_simple_chart(values: &[f64]) {
    if values.is_empty() {
        return;
    }

    let height = 8;
    let width = 50.min(values.len());
    let min_val = values.iter().cloned().fold(f64::INFINITY, f64::min).max(0.0);
    let max_val = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max).max(min_val + 1.0);

    // Sample values to fit the width
    let sampled: Vec<f64> = if values.len() > width {
        let step = values.len() as f64 / width as f64;
        (0..width)
            .map(|i| {
                let idx = (i as f64 * step) as usize;
                values[idx.min(values.len() - 1)]
            })
            .collect()
    } else {
        values.to_vec()
    };

    let range = max_val - min_val;

    for row in (0..height).rev() {
        let threshold = min_val + range * row as f64 / (height - 1) as f64;
        let label = format!("{:>5.0}%", threshold);
        print!("  {} │", label);

        for val in &sampled {
            if *val >= threshold {
                print!("█");
            } else {
                print!("░");
            }
        }
        println!();
    }

    print!("  {:>5} ╰", "");
    for _ in 0..sampled.len() {
        print!("─");
    }
    println!();
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}
