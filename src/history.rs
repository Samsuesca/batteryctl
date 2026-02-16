use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::battery::BatteryInfo;

/// A historical snapshot of battery state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatterySnapshot {
    pub timestamp: DateTime<Utc>,
    pub level: u8,
    pub is_charging: bool,
    pub power_draw: Option<f64>,
    pub cycle_count: Option<u32>,
    pub max_capacity: Option<u32>,
    pub design_capacity: Option<u32>,
}

/// Summary statistics for a time period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistorySummary {
    pub period_description: String,
    pub snapshots_count: usize,
    pub avg_level: f64,
    pub min_level: u8,
    pub max_level: u8,
    pub charging_periods: u32,
    pub total_charging_minutes: i64,
    pub total_discharging_minutes: i64,
    pub avg_discharge_rate_watts: Option<f64>,
    pub estimated_cycles: f64,
}

/// Manages the SQLite history database.
pub struct HistoryManager {
    conn: Connection,
}

impl HistoryManager {
    /// Open or create the history database.
    pub fn open() -> Result<Self> {
        let db_path = get_db_path()?;

        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {}", parent.display()))?;
        }

        let conn = Connection::open(&db_path)
            .with_context(|| format!("Failed to open database at {}", db_path.display()))?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS snapshots (
                timestamp INTEGER NOT NULL,
                level INTEGER NOT NULL,
                is_charging BOOLEAN NOT NULL,
                power_draw REAL,
                cycle_count INTEGER,
                max_capacity INTEGER,
                design_capacity INTEGER
            );
            CREATE INDEX IF NOT EXISTS idx_snapshots_timestamp ON snapshots(timestamp);",
        )
        .context("Failed to initialize database schema")?;

        Ok(Self { conn })
    }

    /// Open with a specific path (for testing).
    pub fn open_at(path: &std::path::Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS snapshots (
                timestamp INTEGER NOT NULL,
                level INTEGER NOT NULL,
                is_charging BOOLEAN NOT NULL,
                power_draw REAL,
                cycle_count INTEGER,
                max_capacity INTEGER,
                design_capacity INTEGER
            );
            CREATE INDEX IF NOT EXISTS idx_snapshots_timestamp ON snapshots(timestamp);",
        )?;
        Ok(Self { conn })
    }

    /// Record a battery snapshot to the database.
    pub fn record_snapshot(&self, info: &BatteryInfo) -> Result<()> {
        let now = Utc::now().timestamp();
        let is_charging = matches!(
            info.state,
            crate::battery::ChargingState::Charging | crate::battery::ChargingState::Full
        );

        self.conn.execute(
            "INSERT INTO snapshots (timestamp, level, is_charging, power_draw, cycle_count, max_capacity, design_capacity)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                now,
                info.level as i32,
                is_charging,
                info.power_draw_watts,
                info.cycle_count.map(|c| c as i32),
                info.max_capacity_mah.map(|c| c as i32),
                info.design_capacity_mah.map(|c| c as i32),
            ],
        )?;
        Ok(())
    }

    /// Get snapshots within a duration from now.
    pub fn get_snapshots_range(&self, duration: Duration) -> Result<Vec<BatterySnapshot>> {
        let since = (Utc::now() - duration).timestamp();
        let mut stmt = self.conn.prepare(
            "SELECT timestamp, level, is_charging, power_draw, cycle_count, max_capacity, design_capacity
             FROM snapshots
             WHERE timestamp >= ?1
             ORDER BY timestamp ASC",
        )?;

        let snapshots = stmt
            .query_map(params![since], |row| {
                let ts: i64 = row.get(0)?;
                let level: i32 = row.get(1)?;
                let is_charging: bool = row.get(2)?;
                let power_draw: Option<f64> = row.get(3)?;
                let cycle_count: Option<i32> = row.get(4)?;
                let max_capacity: Option<i32> = row.get(5)?;
                let design_capacity: Option<i32> = row.get(6)?;

                Ok(BatterySnapshot {
                    timestamp: DateTime::from_timestamp(ts, 0).unwrap_or_default(),
                    level: level.clamp(0, 100) as u8,
                    is_charging,
                    power_draw,
                    cycle_count: cycle_count.map(|c| c as u32),
                    max_capacity: max_capacity.map(|c| c as u32),
                    design_capacity: design_capacity.map(|c| c as u32),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(snapshots)
    }

    /// Compute a summary of battery usage over a time period.
    pub fn get_summary(&self, duration: Duration) -> Result<HistorySummary> {
        let snapshots = self.get_snapshots_range(duration)?;

        if snapshots.is_empty() {
            return Ok(HistorySummary {
                period_description: format_duration(&duration),
                snapshots_count: 0,
                avg_level: 0.0,
                min_level: 0,
                max_level: 0,
                charging_periods: 0,
                total_charging_minutes: 0,
                total_discharging_minutes: 0,
                avg_discharge_rate_watts: None,
                estimated_cycles: 0.0,
            });
        }

        let levels: Vec<f64> = snapshots.iter().map(|s| s.level as f64).collect();
        let avg_level = levels.iter().sum::<f64>() / levels.len() as f64;
        let min_level = snapshots.iter().map(|s| s.level).min().unwrap_or(0);
        let max_level = snapshots.iter().map(|s| s.level).max().unwrap_or(100);

        // Count charging periods (transitions from not-charging to charging)
        let mut charging_periods = 0u32;
        let mut charging_minutes = 0i64;
        let mut discharging_minutes = 0i64;
        let mut was_charging = false;

        for i in 0..snapshots.len() {
            if snapshots[i].is_charging && !was_charging {
                charging_periods += 1;
            }
            was_charging = snapshots[i].is_charging;

            if i + 1 < snapshots.len() {
                let dt = snapshots[i + 1]
                    .timestamp
                    .signed_duration_since(snapshots[i].timestamp)
                    .num_minutes();
                if snapshots[i].is_charging {
                    charging_minutes += dt;
                } else {
                    discharging_minutes += dt;
                }
            }
        }

        // Average discharge rate
        let discharge_powers: Vec<f64> = snapshots
            .iter()
            .filter(|s| !s.is_charging)
            .filter_map(|s| s.power_draw)
            .collect();
        let avg_discharge_rate = if !discharge_powers.is_empty() {
            Some(discharge_powers.iter().sum::<f64>() / discharge_powers.len() as f64)
        } else {
            None
        };

        // Estimate cycles: sum of |level changes| / 100
        let mut total_level_change: f64 = 0.0;
        for i in 1..snapshots.len() {
            let diff = (snapshots[i].level as f64 - snapshots[i - 1].level as f64).abs();
            if !snapshots[i].is_charging && !snapshots[i - 1].is_charging {
                total_level_change += diff;
            }
        }
        let estimated_cycles = total_level_change / 100.0;

        Ok(HistorySummary {
            period_description: format_duration(&duration),
            snapshots_count: snapshots.len(),
            avg_level,
            min_level,
            max_level,
            charging_periods,
            total_charging_minutes: charging_minutes,
            total_discharging_minutes: discharging_minutes,
            avg_discharge_rate_watts: avg_discharge_rate,
            estimated_cycles,
        })
    }

    /// Get the total number of snapshots stored.
    pub fn snapshot_count(&self) -> Result<usize> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM snapshots",
            [],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    /// Prune old snapshots beyond a retention period.
    pub fn prune(&self, keep_duration: Duration) -> Result<usize> {
        let cutoff = (Utc::now() - keep_duration).timestamp();
        let deleted = self.conn.execute(
            "DELETE FROM snapshots WHERE timestamp < ?1",
            params![cutoff],
        )?;
        Ok(deleted)
    }
}

fn get_db_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    Ok(home.join(".batteryctl").join("history.db"))
}

fn format_duration(d: &Duration) -> String {
    let hours = d.num_hours();
    if hours < 24 {
        format!("Last {} hour{}", hours, if hours != 1 { "s" } else { "" })
    } else {
        let days = d.num_days();
        if days < 7 {
            format!("Last {} day{}", days, if days != 1 { "s" } else { "" })
        } else if days < 30 {
            let weeks = days / 7;
            format!("Last {} week{}", weeks, if weeks != 1 { "s" } else { "" })
        } else {
            let months = days / 30;
            format!(
                "Last {} month{}",
                months,
                if months != 1 { "s" } else { "" }
            )
        }
    }
}

/// Parse a duration string like "24h", "7d", "30d", "1w", "1m" into a chrono Duration.
pub fn parse_duration_str(s: &str) -> Result<Duration> {
    let s = s.trim().to_lowercase();
    let (num_str, unit) = if s.ends_with('h') {
        (&s[..s.len() - 1], 'h')
    } else if s.ends_with('d') {
        (&s[..s.len() - 1], 'd')
    } else if s.ends_with('w') {
        (&s[..s.len() - 1], 'w')
    } else if s.ends_with('m') {
        (&s[..s.len() - 1], 'm')
    } else {
        anyhow::bail!("Invalid duration format '{}'. Use e.g. 24h, 7d, 4w, 1m", s);
    };

    let num: i64 = num_str
        .parse()
        .with_context(|| format!("Invalid number in duration '{}'", s))?;

    let duration = match unit {
        'h' => Duration::hours(num),
        'd' => Duration::days(num),
        'w' => Duration::weeks(num),
        'm' => Duration::days(num * 30),
        _ => unreachable!(),
    };

    Ok(duration)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battery::{BatteryCondition, ChargingState};

    fn make_test_info(level: u8, charging: bool) -> BatteryInfo {
        BatteryInfo {
            level,
            state: if charging {
                ChargingState::Charging
            } else {
                ChargingState::Discharging
            },
            time_remaining_minutes: None,
            power_draw_watts: Some(10.0),
            cycle_count: Some(50),
            max_capacity_mah: Some(4200),
            design_capacity_mah: Some(4500),
            current_capacity_mah: None,
            temperature_celsius: None,
            voltage_mv: None,
            condition: BatteryCondition::Normal,
            manufacture_date: None,
            is_present: true,
        }
    }

    #[test]
    fn test_record_and_retrieve() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let hist = HistoryManager::open_at(tmp.path()).unwrap();

        let info = make_test_info(80, false);
        hist.record_snapshot(&info).unwrap();
        hist.record_snapshot(&info).unwrap();

        assert_eq!(hist.snapshot_count().unwrap(), 2);

        let snapshots = hist.get_snapshots_range(Duration::hours(1)).unwrap();
        assert_eq!(snapshots.len(), 2);
        assert_eq!(snapshots[0].level, 80);
    }

    #[test]
    fn test_parse_duration_str() {
        assert_eq!(parse_duration_str("24h").unwrap(), Duration::hours(24));
        assert_eq!(parse_duration_str("7d").unwrap(), Duration::days(7));
        assert_eq!(parse_duration_str("4w").unwrap(), Duration::weeks(4));
        assert_eq!(parse_duration_str("1m").unwrap(), Duration::days(30));
        assert!(parse_duration_str("abc").is_err());
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(&Duration::hours(12)), "Last 12 hours");
        assert_eq!(format_duration(&Duration::days(3)), "Last 3 days");
        assert_eq!(format_duration(&Duration::weeks(2)), "Last 2 weeks");
        assert_eq!(format_duration(&Duration::days(60)), "Last 2 months");
    }
}
