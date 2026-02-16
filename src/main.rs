#![allow(dead_code)]

mod alert;
mod battery;
mod display;
mod health;
mod history;
mod optimize;
mod power;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// batteryctl - Battery health and power management CLI
#[derive(Parser)]
#[command(
    name = "batteryctl",
    version,
    about = "Battery health and power management CLI for macOS and Linux",
    long_about = "Monitor battery health, track power consumption, and optimize battery life.\nSupports macOS (Apple Silicon & Intel) and Linux."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Output as JSON instead of formatted text
    #[arg(long, global = true)]
    json: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Current battery status with detailed metrics
    Status {
        /// Show detailed metrics (health, cycles, temperature)
        #[arg(short, long)]
        detailed: bool,

        /// Watch mode: refresh every N seconds
        #[arg(short, long)]
        watch: bool,

        /// Refresh interval in seconds (used with --watch)
        #[arg(short, long, default_value = "5")]
        interval: u64,
    },

    /// Battery health report with degradation trends
    Health {
        /// Show capacity history trend
        #[arg(long)]
        history: bool,

        /// Compare current battery with a new one
        #[arg(long)]
        compare_new: bool,
    },

    /// Identify top power-consuming applications
    PowerHogs {
        /// Show detailed per-process info
        #[arg(short, long)]
        detailed: bool,

        /// Filter by application name
        #[arg(short, long)]
        filter: Option<String>,
    },

    /// Battery usage history over configurable time periods
    History {
        /// Time period (e.g., 24h, 7d, 4w, 1m)
        #[arg(short, long, default_value = "24h")]
        duration: String,

        /// Export to CSV file
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Smart suggestions to optimize battery life
    Optimize {
        /// Include aggressive power-saving tips
        #[arg(short, long)]
        aggressive: bool,
    },

    /// Set battery level alerts
    Alert {
        /// Alert when battery reaches this level (e.g., 20)
        #[arg(short, long)]
        level: Option<u8>,

        /// Alert when battery is fully charged
        #[arg(long)]
        on_full: bool,

        /// Run as background daemon
        #[arg(short, long)]
        daemon: bool,
    },

    /// Record a battery snapshot to the history database
    Record,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Status {
            detailed,
            watch,
            interval,
        } => cmd_status(detailed, watch, interval, cli.json),

        Commands::Health {
            history,
            compare_new,
        } => cmd_health(history, compare_new, cli.json),

        Commands::PowerHogs { detailed, filter } => cmd_power_hogs(detailed, filter, cli.json),

        Commands::History { duration, output } => cmd_history(&duration, output.as_deref(), cli.json),

        Commands::Optimize { aggressive } => cmd_optimize(aggressive, cli.json),

        Commands::Alert {
            level,
            on_full,
            daemon,
        } => cmd_alert(level, on_full, daemon),

        Commands::Record => cmd_record(),
    }
}

// ── Command implementations ────────────────────────────────────────────

fn cmd_status(detailed: bool, watch: bool, interval: u64, json: bool) -> Result<()> {
    if watch {
        let running = Arc::new(AtomicBool::new(true));
        let r = running.clone();
        ctrlc::set_handler(move || {
            r.store(false, Ordering::Relaxed);
        })?;

        while running.load(Ordering::Relaxed) {
            // Clear screen
            print!("\x1B[2J\x1B[1;1H");

            let info = battery::get_battery_info()?;

            // Record snapshot while we're at it
            if let Ok(hist) = history::HistoryManager::open() {
                let _ = hist.record_snapshot(&info);
            }

            if json {
                println!("{}", serde_json::to_string_pretty(&info)?);
            } else {
                display::print_status(&info, detailed);
                println!(
                    "\n{}",
                    format!(
                        "Refreshing every {}s. Press Ctrl+C to stop.",
                        interval
                    )
                );
            }

            // Interruptible sleep
            let sleep_ms = interval * 1000;
            let step = 250u64;
            let mut elapsed = 0u64;
            while elapsed < sleep_ms && running.load(Ordering::Relaxed) {
                std::thread::sleep(std::time::Duration::from_millis(step.min(sleep_ms - elapsed)));
                elapsed += step;
            }
        }
    } else {
        let info = battery::get_battery_info()?;

        // Record snapshot
        if let Ok(hist) = history::HistoryManager::open() {
            let _ = hist.record_snapshot(&info);
        }

        if json {
            println!("{}", serde_json::to_string_pretty(&info)?);
        } else {
            display::print_status(&info, detailed);
        }
    }

    Ok(())
}

fn cmd_health(show_history: bool, compare_new: bool, json: bool) -> Result<()> {
    let info = battery::get_battery_info()?;

    let hist_manager = if show_history {
        history::HistoryManager::open().ok()
    } else {
        None
    };

    let report = health::generate_health_report(&info, hist_manager.as_ref())?;

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    display::print_health_report(&report);

    if compare_new {
        let comparison = health::compare_with_new(&info);
        display::print_health_comparison(&comparison);
    }

    Ok(())
}

fn cmd_power_hogs(detailed: bool, filter: Option<String>, json: bool) -> Result<()> {
    let sys_power = power::get_system_power_draw();

    let report = if let Some(ref f) = filter {
        power::get_power_report_filtered(f, sys_power)?
    } else {
        power::get_power_report(sys_power)?
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        display::print_power_report(&report, detailed);
    }

    Ok(())
}

fn cmd_history(duration_str: &str, output: Option<&str>, json: bool) -> Result<()> {
    let duration = history::parse_duration_str(duration_str)?;
    let hist = history::HistoryManager::open()?;

    let snapshots = hist.get_snapshots_range(duration)?;
    let summary = hist.get_summary(duration)?;

    // Export if requested
    if let Some(path) = output {
        if path.ends_with(".csv") {
            display::export_snapshots_csv(&snapshots, path)?;
        } else {
            // Default to JSON
            let json_str = serde_json::to_string_pretty(&snapshots)?;
            std::fs::write(path, json_str)?;
            println!("Exported {} snapshots to {}", snapshots.len(), path);
        }
        return Ok(());
    }

    if json {
        let output = serde_json::json!({
            "summary": summary,
            "snapshots": snapshots,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        display::print_history(&snapshots, &summary);
    }

    Ok(())
}

fn cmd_optimize(aggressive: bool, json: bool) -> Result<()> {
    let info = battery::get_battery_info()?;
    let sys_power = power::get_system_power_draw();
    let power_report = power::get_power_report(sys_power)?;
    let report = optimize::generate_suggestions(&info, Some(&power_report), aggressive);

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        display::print_optimization_report(&report);
    }

    Ok(())
}

fn cmd_alert(level: Option<u8>, on_full: bool, _daemon: bool) -> Result<()> {
    if level.is_none() && !on_full {
        anyhow::bail!(
            "Please specify at least one alert condition:\n  \
             --level <N>  Alert when battery reaches N%\n  \
             --on-full    Alert when fully charged"
        );
    }

    let config = alert::AlertConfig {
        level_threshold: level,
        on_full,
        check_interval: std::time::Duration::from_secs(60),
    };

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::Relaxed);
    })?;

    alert::run_alert_loop(&config, running)
}

fn cmd_record() -> Result<()> {
    let info = battery::get_battery_info()?;
    let hist = history::HistoryManager::open()?;
    hist.record_snapshot(&info)?;
    println!(
        "Recorded snapshot: {}% ({})",
        info.level, info.state
    );
    Ok(())
}
