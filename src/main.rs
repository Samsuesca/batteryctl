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
    long_about = "Monitor battery health, track power consumption, and optimize battery life.\nSupports macOS (Apple Silicon & Intel) and Linux.",
    after_help = "Common workflows:\n  Monitor while compiling:  batteryctl status --watch &\n  Quick health check:       batteryctl health\n  Full health with trends:  batteryctl health --history --compare-new\n  Find power hogs:          batteryctl power-hogs --detailed\n  Export usage data:        batteryctl history -d 7d -o battery.csv\n  Background alerts:        batteryctl alert --level 20 --daemon\n  Optimize battery life:    batteryctl optimize --aggressive\n\nAll subcommands support --json for machine-readable output.\nRun 'batteryctl <command> --help' for detailed usage of each command."
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
    #[command(
        long_about = "Display current battery status including charge level, power state, and time remaining.\n\nExamples:\n  batteryctl status              # Quick status overview\n  batteryctl status -d           # Detailed view with health, cycles, temperature\n  batteryctl status --watch      # Live monitoring with 5s refresh\n  batteryctl status -w -i 2      # Live monitoring with 2s refresh interval\n  batteryctl status --json       # Machine-readable JSON output"
    )]
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
    #[command(
        long_about = "Generate a comprehensive battery health report showing capacity degradation,\ncycle count, estimated remaining lifespan, and manufacturing details.\n\nExamples:\n  batteryctl health                    # Basic health report\n  batteryctl health --history          # Include capacity trend chart over time\n  batteryctl health --compare-new      # Compare current battery vs new baseline\n  batteryctl health --history --compare-new  # Full report with trends and comparison\n  batteryctl health --json             # Export health data as JSON"
    )]
    Health {
        /// Show capacity history trend
        #[arg(long)]
        history: bool,

        /// Compare current battery with a new one
        #[arg(long)]
        compare_new: bool,
    },

    /// Identify top power-consuming applications
    #[command(
        long_about = "List the top power-consuming applications currently running on your system.\nEstimates per-app power draw based on CPU usage and system power metrics.\n\nExamples:\n  batteryctl power-hogs              # Top 10 power consumers\n  batteryctl power-hogs -d           # Detailed view with process counts (top 20)\n  batteryctl power-hogs -f chrome    # Filter results by application name\n  batteryctl power-hogs -d -f slack  # Detailed info for a specific app\n  batteryctl power-hogs --json       # JSON output for scripting"
    )]
    PowerHogs {
        /// Show detailed per-process info
        #[arg(short, long)]
        detailed: bool,

        /// Filter by application name
        #[arg(short, long)]
        filter: Option<String>,
    },

    /// Battery usage history over configurable time periods
    #[command(
        long_about = "View battery usage history with charge level charts and summary statistics.\nData is collected automatically when running other batteryctl commands.\n\nDuration format: <number><unit> where unit is h (hours), d (days), w (weeks), m (months).\n\nExamples:\n  batteryctl history                 # Last 24 hours (default)\n  batteryctl history -d 7d           # Last 7 days\n  batteryctl history -d 4w           # Last 4 weeks\n  batteryctl history -d 1m           # Last month\n  batteryctl history -d 24h -o data.csv   # Export to CSV file\n  batteryctl history -d 7d -o data.json   # Export to JSON file"
    )]
    History {
        /// Time period (e.g., 24h, 7d, 4w, 1m)
        #[arg(short, long, default_value = "24h")]
        duration: String,

        /// Export to CSV file
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Smart suggestions to optimize battery life
    #[command(
        long_about = "Analyze your current battery usage and running applications to provide\nactionable suggestions for extending battery life, ranked by impact.\n\nExamples:\n  batteryctl optimize              # Standard optimization suggestions\n  batteryctl optimize -a           # Include aggressive power-saving tips\n  batteryctl optimize --json       # JSON output for integration with scripts\n  batteryctl optimize -a --json    # Aggressive tips in JSON format"
    )]
    Optimize {
        /// Include aggressive power-saving tips
        #[arg(short, long)]
        aggressive: bool,
    },

    /// Set battery level alerts
    #[command(
        long_about = "Configure battery level alerts that notify you when the charge drops below\na threshold or when the battery is fully charged. Runs as a foreground\nprocess or background daemon.\n\nExamples:\n  batteryctl alert --level 20            # Alert at 20% battery\n  batteryctl alert --on-full             # Alert when fully charged\n  batteryctl alert --level 15 --on-full  # Both low battery and full alerts\n  batteryctl alert --level 20 --daemon   # Run alerts in background daemon\n  batteryctl alert -l 10 -d              # Shorthand for daemon at 10%"
    )]
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
    #[command(
        long_about = "Manually record a single battery snapshot to the local history database.\nThis is useful for cron jobs or periodic data collection scripts.\nNote: Snapshots are also recorded automatically by 'status' and 'status --watch'.\n\nExamples:\n  batteryctl record                          # Record current state\n  watch -n 300 batteryctl record             # Record every 5 minutes (shell)\n  crontab: */10 * * * * batteryctl record    # Cron job every 10 minutes"
    )]
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
