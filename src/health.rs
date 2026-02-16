use crate::battery::BatteryInfo;
use crate::history::HistoryManager;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Complete battery health report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    pub design_capacity_mah: Option<u32>,
    pub max_capacity_mah: Option<u32>,
    pub capacity_loss_mah: Option<i32>,
    pub capacity_loss_percent: Option<f64>,
    pub cycle_count: Option<u32>,
    pub estimated_remaining_cycles: Option<u32>,
    pub condition: String,
    pub health_percent: Option<f64>,
    pub manufacture_date: Option<String>,
    pub age_description: Option<String>,
    pub capacity_trend: Vec<CapacityDataPoint>,
}

/// A data point for capacity trending over time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityDataPoint {
    pub date: DateTime<Utc>,
    pub max_capacity_mah: u32,
    pub health_percent: f64,
}

/// Generate a health report from current battery info and historical data.
pub fn generate_health_report(
    info: &BatteryInfo,
    history: Option<&HistoryManager>,
) -> Result<HealthReport> {
    let capacity_loss = info.capacity_loss_mah();
    let capacity_loss_pct = match (info.max_capacity_mah, info.design_capacity_mah) {
        (Some(max), Some(design)) if design > 0 => {
            Some((1.0 - max as f64 / design as f64) * 100.0)
        }
        _ => None,
    };

    let age_description = info
        .manufacture_date
        .as_ref()
        .and_then(|d| calculate_age_description(d));

    // Build capacity trend from historical data
    let capacity_trend = if let Some(hist) = history {
        build_capacity_trend(hist).unwrap_or_default()
    } else {
        Vec::new()
    };

    Ok(HealthReport {
        design_capacity_mah: info.design_capacity_mah,
        max_capacity_mah: info.max_capacity_mah,
        capacity_loss_mah: capacity_loss,
        capacity_loss_percent: capacity_loss_pct,
        cycle_count: info.cycle_count,
        estimated_remaining_cycles: info.estimated_remaining_cycles(),
        condition: info.condition.to_string(),
        health_percent: info.health_percent(),
        manufacture_date: info.manufacture_date.clone(),
        age_description,
        capacity_trend,
    })
}

/// Compare current capacity with a new battery of the same design.
pub fn compare_with_new(info: &BatteryInfo) -> Vec<(String, String, String)> {
    let mut comparisons = Vec::new();

    if let (Some(max), Some(design)) = (info.max_capacity_mah, info.design_capacity_mah) {
        comparisons.push((
            "Max Capacity".to_string(),
            format!("{} mAh", design),
            format!("{} mAh", max),
        ));
    }

    if let Some(health) = info.health_percent() {
        comparisons.push((
            "Health".to_string(),
            "100.0%".to_string(),
            format!("{:.1}%", health),
        ));
    }

    if let Some(cycles) = info.cycle_count {
        comparisons.push((
            "Cycle Count".to_string(),
            "0".to_string(),
            format!("{}", cycles),
        ));
    }

    comparisons.push((
        "Condition".to_string(),
        "Normal".to_string(),
        info.condition.to_string(),
    ));

    comparisons
}

fn calculate_age_description(date_str: &str) -> Option<String> {
    // Try parsing common date formats
    let now = Utc::now();

    let parsed = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .or_else(|_| chrono::NaiveDate::parse_from_str(date_str, "%B %Y"))
        .or_else(|_| chrono::NaiveDate::parse_from_str(date_str, "%m/%d/%Y"))
        .ok()?;

    let manufacture_dt = parsed.and_hms_opt(0, 0, 0)?;
    let manufacture_utc = DateTime::<Utc>::from_naive_utc_and_offset(manufacture_dt, Utc);
    let duration = now.signed_duration_since(manufacture_utc);

    let days = duration.num_days();
    if days < 0 {
        return None;
    }

    let years = days / 365;
    let remaining_days = days % 365;
    let months = remaining_days / 30;

    Some(if years > 0 {
        if months > 0 {
            format!("{} year{}, {} month{}", years, if years > 1 { "s" } else { "" }, months, if months > 1 { "s" } else { "" })
        } else {
            format!("{} year{}", years, if years > 1 { "s" } else { "" })
        }
    } else if months > 0 {
        format!("{} month{}", months, if months > 1 { "s" } else { "" })
    } else {
        format!("{} day{}", days, if days != 1 { "s" } else { "" })
    })
}

fn build_capacity_trend(history: &HistoryManager) -> Result<Vec<CapacityDataPoint>> {
    // Get snapshots grouped by day over the last 6 months
    let snapshots = history.get_snapshots_range(
        chrono::Duration::days(180),
    )?;

    if snapshots.is_empty() {
        return Ok(Vec::new());
    }

    // Group by date and pick the max_capacity value per day
    let mut daily: std::collections::BTreeMap<String, (u32, u32)> =
        std::collections::BTreeMap::new();

    for snap in &snapshots {
        let date_key = snap.timestamp.format("%Y-%m-%d").to_string();
        if let Some(max_cap) = snap.max_capacity {
            let design = snap.design_capacity.unwrap_or(max_cap);
            daily.entry(date_key).or_insert((max_cap, design));
        }
    }

    let trend: Vec<CapacityDataPoint> = daily
        .into_iter()
        .filter_map(|(date_str, (max_cap, design_cap))| {
            let date = chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").ok()?;
            let dt = date.and_hms_opt(0, 0, 0)?;
            let utc = DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc);
            let health = if design_cap > 0 {
                max_cap as f64 / design_cap as f64 * 100.0
            } else {
                100.0
            };
            Some(CapacityDataPoint {
                date: utc,
                max_capacity_mah: max_cap,
                health_percent: health,
            })
        })
        .collect();

    Ok(trend)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battery::{BatteryCondition, ChargingState};

    fn make_test_info() -> BatteryInfo {
        BatteryInfo {
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
        }
    }

    #[test]
    fn test_generate_health_report() {
        let info = make_test_info();
        let report = generate_health_report(&info, None).unwrap();
        assert_eq!(report.design_capacity_mah, Some(4500));
        assert_eq!(report.max_capacity_mah, Some(4215));
        assert_eq!(report.capacity_loss_mah, Some(285));
        assert_eq!(report.cycle_count, Some(47));
        assert_eq!(report.estimated_remaining_cycles, Some(953));
        assert_eq!(report.condition, "Normal");
    }

    #[test]
    fn test_compare_with_new() {
        let info = make_test_info();
        let comparison = compare_with_new(&info);
        assert!(!comparison.is_empty());
        assert_eq!(comparison[0].0, "Max Capacity");
    }
}
