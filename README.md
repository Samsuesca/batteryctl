# batteryctl

> Battery health and power management CLI for macOS

![macOS](https://img.shields.io/badge/macOS-Apple_Silicon-blue)
![Rust](https://img.shields.io/badge/Rust-1.70+-orange)
![License](https://img.shields.io/badge/license-MIT-green)

**batteryctl** is a command-line tool for monitoring battery health, tracking power consumption, and optimizing battery life on macOS (Apple Silicon & Intel).

---

## Features

- **Battery Status**: Current level, charging state, time remaining
- **Health Metrics**: Cycle count, max capacity, battery condition
- **Power Consumption**: Top apps consuming battery power
- **History**: Battery usage over time (last 24h, week, month)
- **Optimization**: Smart suggestions to extend battery life
- **Alerts**: Notifications when battery reaches thresholds
- **Export**: Battery reports to JSON/CSV for analysis

---

## Installation

```bash
git clone https://github.com/Samsuesca/batteryctl.git
cd batteryctl
cargo build --release
cargo install --path .
```

---

## Usage

### Battery Status

```bash
# Current battery status
batteryctl status

# Detailed status with health metrics
batteryctl status --detailed

# Watch mode (refresh every N seconds)
batteryctl status --watch --interval 5
```

**Output:**
```
┌─────────────────────────────────────────────────────────┐
│                    BATTERY STATUS                       │
├─────────────────────────────────────────────────────────┤
│ Level:           87%                                    │
│ State:           🔌 Charging (AC Power)                 │
│ Time to Full:    ⏱️  1h 23m                             │
│ Power Draw:      -15.2W (charging)                      │
│                                                          │
│ Health:          ✅ Normal                               │
│ Max Capacity:    94% (4,215 mAh / 4,500 mAh)           │
│ Cycle Count:     47 / 1,000                             │
│ Temperature:     32°C                                    │
└─────────────────────────────────────────────────────────┘
```

### Health Metrics

```bash
# Battery health report
batteryctl health

# Show degradation over time
batteryctl health --history

# Compare with new battery
batteryctl health --compare-new
```

**Output:**
```
Battery Health Report:

┌────────────────────────────────────────────┐
│ Design Capacity:     4,500 mAh            │
│ Current Max:         4,215 mAh            │
│ Capacity Loss:       -285 mAh (-6%)      │
│                                            │
│ Cycle Count:         47                   │
│ Estimated Remaining: 953 cycles           │
│                                            │
│ Condition:           ✅ Normal             │
│ Manufactured:        March 2024           │
│ Age:                 11 months            │
└────────────────────────────────────────────┘

📊 Capacity Trend (last 6 months):
  100% │ ●
       │  ●
   95% │   ●●
       │     ●
   90% │      ●
       └──────────────
       Mar  May  Jul  Sep  Nov  Jan
```

### Power Consumption

```bash
# Apps consuming most battery
batteryctl power-hogs

# Show consumption with percentages
batteryctl power-hogs --detailed

# Filter by app name
batteryctl power-hogs --filter "Chrome"
```

**Output:**
```
Top Power Consumers (last 12 hours):

┌─────┬──────────────────────────┬─────────────┬──────────┐
│ #   │ Application              │ Energy Used │ % Total  │
├─────┼──────────────────────────┼─────────────┼──────────┤
│ 1   │ Google Chrome            │ 23.4 Wh     │ 34.2%    │
│ 2   │ Visual Studio Code       │ 12.1 Wh     │ 17.7%    │
│ 3   │ Docker Desktop           │ 8.9 Wh      │ 13.0%    │
│ 4   │ Stata/MP                 │ 6.2 Wh      │ 9.1%     │
│ 5   │ Spotify                  │ 4.8 Wh      │ 7.0%     │
│     │ Other (15 apps)          │ 13.0 Wh     │ 19.0%    │
└─────┴──────────────────────────┴─────────────┴──────────┘

Total consumption: 68.4 Wh
Estimated discharge rate: 5.7 W
Battery life remaining: ~6h 15m
```

### Battery History

```bash
# Usage over last 24 hours
batteryctl history --duration 24h

# Weekly overview
batteryctl history --duration 7d

# Export to CSV for analysis
batteryctl history --duration 30d --output battery_log.csv
```

**Output:**
```
Battery History (Last 24 Hours):

 100% │     ──────
      │   ╱        ╲
  75% │  ╱          ╲___
      │ ╱               ╲
  50% │╱                 ╲___
      │                      ╲
  25% │                       ╲
      └─────────────────────────
      00:00  06:00  12:00  18:00  24:00

🔌 Charging periods: 3 (total: 4h 32m)
⚡ On battery: 19h 28m
📊 Avg discharge rate: 8.2 W
📉 Cycles completed: 0.87 cycles
```

### Optimization Suggestions

```bash
# Get battery optimization tips
batteryctl optimize

# Show aggressive power-saving tips
batteryctl optimize --aggressive
```

**Output:**
```
🔋 Battery Optimization Suggestions:

High Impact (Immediate):
  ⚠️  Google Chrome is using 34% of battery
     → Close unused tabs or switch to Safari
  ⚠️  Display brightness: 85%
     → Reduce to 60% (saves ~2W)
  ⚠️  Docker Desktop running while idle
     → Stop containers when not developing

Medium Impact:
  💡 Bluetooth enabled but no devices connected
     → Disable to save ~0.3W
  💡 6 apps running in background
     → Quit unused apps (Activity Monitor)

Low Impact:
  ℹ️  Turn off keyboard backlight (saves ~0.1W)
  ℹ️  Enable "Low Power Mode" when below 20%

Estimated savings: +2h 15m battery life
```

### Alerts

```bash
# Alert when battery reaches 20%
batteryctl alert --level 20

# Alert when fully charged
batteryctl alert --on-full

# Run in background (daemon mode)
batteryctl alert --level 20 --daemon
```

---

## Command Reference

| Command | Description | Options |
|---------|-------------|---------|
| `status` | Current battery status | `--detailed`, `--watch`, `--interval` |
| `health` | Battery health metrics | `--history`, `--compare-new` |
| `power-hogs` | Apps consuming battery | `--detailed`, `--filter` |
| `history` | Battery usage over time | `--duration`, `--output` |
| `optimize` | Optimization suggestions | `--aggressive` |
| `alert` | Set battery alerts | `--level`, `--on-full`, `--daemon` |

---

## Use Cases

### Development Sessions

```bash
# Monitor battery during long compile
batteryctl status --watch --interval 10 &

# Run Rust build
cargo build --release

# Check power consumption after
batteryctl power-hogs
```

### Daily Routine

```bash
# Morning check
batteryctl status --detailed

# Set low battery alert
batteryctl alert --level 15 --daemon

# End of day report
batteryctl history --duration 24h
```

### Battery Health Tracking

```bash
# Weekly health check (add to cron)
batteryctl health --history >> ~/battery_logs/health_$(date +%Y%m%d).txt

# Export for spreadsheet analysis
batteryctl history --duration 30d --output ~/battery_logs/jan_2026.csv
```

---

## Technical Stack

**Language**: Rust 2021 edition

**Dependencies**:
- `clap` - CLI parsing
- `ioreg-rs` or direct `IOKit` bindings - Battery metrics (macOS)
- `sysinfo` - System information
- `chrono` - Time handling
- `serde` / `serde_json` - Data serialization
- `colored` - Terminal colors
- `tabled` - Table formatting
- `textplots` - ASCII charts

---

## Architecture

```
src/
├── main.rs           # CLI entry point
├── battery.rs        # Battery metrics via IOKit
├── power.rs          # Power consumption tracking
├── health.rs         # Health calculations
├── history.rs        # Historical data management
├── optimize.rs       # Optimization engine
├── alert.rs          # Alert daemon
└── display.rs        # Formatted output
```

---

## Implementation Notes

### Accessing Battery Data on macOS

Use `IOKit` framework to read battery information:

```rust
// Example: Reading battery percentage
use iokit_rs::battery::BatteryInfo;

let battery = BatteryInfo::new()?;
let percentage = battery.current_capacity() as f32 / battery.max_capacity() as f32 * 100.0;
```

Alternatively, parse output of `pmset -g batt` and `system_profiler SPPowerDataType`.

### Power Consumption Tracking

macOS provides power metrics via `powermetrics` command (requires sudo for some data):

```bash
# System-wide power consumption
sudo powermetrics --samplers tasks -n 1 -i 1000
```

Parse this output to extract per-app energy usage.

### Historical Data Storage

Store battery snapshots in local SQLite database:
```
~/.batteryctl/history.db
```

Schema:
```sql
CREATE TABLE snapshots (
  timestamp INTEGER,
  level INTEGER,
  is_charging BOOLEAN,
  power_draw REAL,
  cycle_count INTEGER,
  max_capacity INTEGER
);
```

---

## Platform Support

| Platform | Support |
|----------|---------|
| macOS (Apple Silicon) | ✅ Full support |
| macOS (Intel) | ✅ Full support |
| Linux | ⚠️ Partial (UPower API) |
| Windows | ❌ Not supported |

---

## Roadmap

- [ ] Linux support (UPower integration)
- [ ] Battery calibration assistant
- [ ] Historical trend predictions (ML-based)
- [ ] Integration with macOS notifications
- [ ] Export to Apple Health (if API available)
- [ ] Comparative analysis (battery vs similar devices)

---

## License

MIT License

---

## Author

**Angel Samuel Suesca Ríos**
suescapsam@gmail.com

---

**Perfect for**: MacBook users who want to maximize battery lifespan and optimize power consumption.
