use crate::battery::{get_battery_info, ChargingState};
use anyhow::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Alert configuration.
#[derive(Debug, Clone)]
pub struct AlertConfig {
    pub level_threshold: Option<u8>,
    pub on_full: bool,
    pub check_interval: Duration,
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            level_threshold: None,
            on_full: false,
            check_interval: Duration::from_secs(60),
        }
    }
}

/// Run the alert monitoring loop.
///
/// This blocks the current thread and monitors the battery, printing alerts
/// when conditions are met. Use `running` to signal the loop to stop.
pub fn run_alert_loop(config: &AlertConfig, running: Arc<AtomicBool>) -> Result<()> {
    let mut level_alerted = false;
    let mut full_alerted = false;

    eprintln!(
        "Battery alert monitor started (checking every {}s)",
        config.check_interval.as_secs()
    );

    if let Some(level) = config.level_threshold {
        eprintln!("  Alert when battery <= {}%", level);
    }
    if config.on_full {
        eprintln!("  Alert when battery is fully charged");
    }

    while running.load(Ordering::Relaxed) {
        match get_battery_info() {
            Ok(info) => {
                // Low battery alert
                if let Some(threshold) = config.level_threshold {
                    if info.level <= threshold
                        && !matches!(info.state, ChargingState::Charging | ChargingState::Full)
                    {
                        if !level_alerted {
                            print_alert(&format!(
                                "Battery LOW: {}% (threshold: {}%)",
                                info.level, threshold
                            ));
                            send_notification(
                                "Battery Low",
                                &format!("Battery is at {}%", info.level),
                            );
                            level_alerted = true;
                        }
                    } else {
                        level_alerted = false;
                    }
                }

                // Full battery alert
                if config.on_full {
                    if matches!(info.state, ChargingState::Full) || info.level >= 100 {
                        if !full_alerted {
                            print_alert("Battery FULL: 100% charged");
                            send_notification(
                                "Battery Full",
                                "Battery is fully charged. You can unplug.",
                            );
                            full_alerted = true;
                        }
                    } else {
                        full_alerted = false;
                    }
                }
            }
            Err(e) => {
                eprintln!("Warning: Could not read battery info: {}", e);
            }
        }

        // Sleep in small increments so we can check the running flag
        let sleep_ms = config.check_interval.as_millis() as u64;
        let step = 500u64;
        let mut elapsed = 0u64;
        while elapsed < sleep_ms && running.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(step.min(sleep_ms - elapsed)));
            elapsed += step;
        }
    }

    eprintln!("Alert monitor stopped.");
    Ok(())
}

fn print_alert(message: &str) {
    use colored::Colorize;
    let timestamp = chrono::Local::now().format("%H:%M:%S");
    eprintln!("\n{}", "=".repeat(50).red().bold());
    eprintln!("{}", format!("  ALERT [{}]", timestamp).red().bold());
    eprintln!("  {}", message.yellow().bold());
    eprintln!("{}\n", "=".repeat(50).red().bold());

    // Ring terminal bell
    eprint!("\x07");
}

fn send_notification(title: &str, body: &str) {
    if cfg!(target_os = "macos") {
        let script = format!(
            "display notification \"{}\" with title \"batteryctl\" subtitle \"{}\"",
            body, title
        );
        let _ = std::process::Command::new("osascript")
            .args(["-e", &script])
            .output();
    } else if cfg!(target_os = "linux") {
        let _ = std::process::Command::new("notify-send")
            .args([
                "--urgency=critical",
                &format!("batteryctl: {}", title),
                body,
            ])
            .output();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alert_config_default() {
        let config = AlertConfig::default();
        assert_eq!(config.level_threshold, None);
        assert!(!config.on_full);
        assert_eq!(config.check_interval, Duration::from_secs(60));
    }
}
