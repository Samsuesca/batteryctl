<!-- AUTO-GENERATED GIT WORKFLOW HEADER -->
<!-- Version: 1.0.0 | Template: GIT_WORKFLOW_CLI.md | Last Updated: 2026-02-16 -->
<!-- DO NOT EDIT MANUALLY - Run: ~/.claude/scripts/sync-git-workflow.sh -->

---

# Git Workflow & Commit Standards

**Version:** 1.0.0
**Last Updated:** 2026-02-15
**Template Type:** Rust CLI Tools

---

## Branch Strategy

### Main Branches

- **`main`** - Stable releases. Tagged with semantic versions.
  - Only merge via Pull Requests
  - All commits must be tested and pass `cargo test`
  - Release tags: `v1.0.0`, `v1.1.0`, etc.

- **`develop`** - Integration branch for features
  - Merge feature branches here first
  - Run full test suite before merging to main
  - Base branch for new features

### Supporting Branches

- **`feature/*`** - New functionality
  - Branch from: `develop`
  - Merge into: `develop`
  - Naming: `feature/add-verbose-flag`, `feature/config-file-support`

- **`bugfix/*`** - Bug fixes
  - Branch from: `develop`
  - Merge into: `develop`
  - Naming: `bugfix/parsing-error`, `bugfix/crash-on-empty-input`

- **`release/*`** - Preparing new releases
  - Branch from: `develop`
  - Merge into: `main` AND `develop`
  - Naming: `release/v1.2.0`

---

## Commit Convention

### Format

```
<emoji> <type>: <description>

[optional body]

[optional footer]
```

### Commit Types with Emojis

```bash
✨ feat:       New feature or command
🐛 fix:        Bug fix
♻️ refactor:   Code restructuring without behavior change
📚 docs:       Documentation updates (README, --help text)
✅ test:       Adding or updating tests
🔒 security:   Security fixes or improvements
⚡ perf:       Performance optimization
🚀 chore:      Dependencies, build config, tooling
🎨 style:      Formatting, whitespace (rustfmt)
🔧 config:     Configuration files
🗑️ remove:     Removing code or files
```

### Examples

**Good commits:**
```bash
✨ feat: add --json flag for structured output
🐛 fix: handle empty input files gracefully
♻️ refactor: extract parsing logic to separate module
📚 docs: update installation instructions in README
✅ test: add integration tests for config loading
🔒 security: validate file paths to prevent traversal
⚡ perf: use rayon for parallel processing
🚀 chore: update clap to 4.5.0
```

---

## Cargo Development Workflow

### Standard Development Cycle

```bash
# 1. Create feature branch
git checkout develop
git pull origin develop
git checkout -b feature/add-verbose-mode

# 2. Implement feature
# Edit: src/main.rs, src/cli.rs

# 3. Format code
cargo fmt

# 4. Run linter
cargo clippy -- -D warnings

# 5. Run tests
cargo test

# 6. Build and test locally
cargo build --release
./target/release/your-cli --help

# 7. Commit changes
git add src/
git commit -m "✨ feat: add --verbose flag for detailed output"

# 8. Push and create PR
git push -u origin feature/add-verbose-mode
```

### Pre-Commit Checklist (CLI Tools)

Before every commit:

- [ ] **`cargo fmt`** - Code is formatted
- [ ] **`cargo clippy`** - No warnings
- [ ] **`cargo test`** - All tests pass
- [ ] **`cargo build --release`** - Release build succeeds
- [ ] **Manual testing** - Tested key use cases
- [ ] **Help text updated** - If new flags/commands added
- [ ] **README updated** - If usage changed

---

## Release Versioning

### Semantic Versioning (SemVer)

Follow **MAJOR.MINOR.PATCH** format:

- **MAJOR** - Breaking changes (e.g., removed commands, changed CLI interface)
- **MINOR** - New features (backward compatible)
- **PATCH** - Bug fixes (backward compatible)

**Examples:**
- `v1.0.0` → `v1.1.0` - Added new `--format` flag (minor)
- `v1.1.0` → `v1.1.1` - Fixed parsing bug (patch)
- `v1.1.1` → `v2.0.0` - Removed deprecated `--old-flag` (major)

### Release Process

```bash
# 1. Create release branch from develop
git checkout develop
git pull origin develop
git checkout -b release/v1.2.0

# 2. Update version in Cargo.toml
# Edit: Cargo.toml
# version = "1.2.0"

# 3. Update CHANGELOG.md
# Add section for v1.2.0 with changes

# 4. Build and test release
cargo build --release
cargo test --release

# 5. Commit version bump
git add Cargo.toml CHANGELOG.md Cargo.lock
git commit -m "🚀 chore: bump version to 1.2.0"

# 6. Merge to main
git checkout main
git merge release/v1.2.0

# 7. Tag release
git tag -a v1.2.0 -m "Release v1.2.0"
git push origin main --tags

# 8. Merge back to develop
git checkout develop
git merge release/v1.2.0
git push origin develop

# 9. Delete release branch
git branch -d release/v1.2.0
```

---

## Binary Distribution

### Building for Distribution

```bash
# macOS (native)
cargo build --release

# macOS (universal binary - Intel + Apple Silicon)
rustup target add x86_64-apple-darwin
rustup target add aarch64-apple-darwin
cargo build --release --target x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin
lipo -create \
  target/x86_64-apple-darwin/release/your-cli \
  target/aarch64-apple-darwin/release/your-cli \
  -output target/release/your-cli-universal

# Linux
rustup target add x86_64-unknown-linux-gnu
cargo build --release --target x86_64-unknown-linux-gnu

# Windows
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu
```

### Installation Methods

**Cargo:**
```bash
cargo install --path .
```

**Homebrew (if published):**
```bash
brew install your-cli
```

**Manual:**
```bash
cp target/release/your-cli /usr/local/bin/
```

---

## Standard Workflows

### 1. Feature Development

```bash
# 1. Start from develop
git checkout develop
git pull origin develop

# 2. Create feature branch
git checkout -b feature/json-output

# 3. Implement, format, test
# ... make changes ...
cargo fmt && cargo clippy && cargo test

# 4. Commit
git add src/
git commit -m "✨ feat: add --json flag for structured output"

# 5. Push and create PR
git push -u origin feature/json-output
```

### 2. Bug Fix

```bash
# 1. Start from develop
git checkout develop
git pull origin develop

# 2. Create bugfix branch
git checkout -b bugfix/handle-empty-files

# 3. Fix, add test, verify
# ... fix bug ...
cargo test
cargo build --release
./target/release/your-cli test-empty-file.txt

# 4. Commit
git add src/ tests/
git commit -m "🐛 fix: handle empty input files without panicking"

# 5. Push and create PR
git push -u origin bugfix/handle-empty-files
```

---

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_config() {
        let config = parse_config("test.conf").unwrap();
        assert_eq!(config.key, "value");
    }
}
```

### Integration Tests

```bash
# tests/integration_test.rs
use assert_cmd::Command;

#[test]
fn test_help_flag() {
    let mut cmd = Command::cargo_bin("your-cli").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("Usage:"));
}
```

### Manual Testing Checklist

- [ ] **Happy path** - Normal use case works
- [ ] **Edge cases** - Empty input, max values, special characters
- [ ] **Error handling** - Invalid flags, missing files, permissions
- [ ] **Help text** - `--help` shows correct information
- [ ] **Version** - `--version` shows correct version

---

## Performance Optimization

### Profiling

```bash
# Install profiler
cargo install flamegraph

# Generate flamegraph
cargo flamegraph -- your-args

# Benchmark
cargo install cargo-criterion
cargo criterion
```

### Common Optimizations

- Use `cargo build --release` (optimization level 3)
- Minimize allocations (use `&str` instead of `String` when possible)
- Use `rayon` for parallel processing
- Enable LTO (Link-Time Optimization) in Cargo.toml

```toml
[profile.release]
lto = true
codegen-units = 1
opt-level = 3
```

---

## Commit Best Practices

### DO ✅

- **Run `cargo fmt` before committing** - Consistent formatting
- **Fix all `clippy` warnings** - Better code quality
- **Add tests for new features** - Prevent regressions
- **Update help text** - Keep CLI docs in sync
- **Use semantic versioning** - Clear version increments
- **Document breaking changes** - In CHANGELOG and commit message

### DON'T ❌

- **Commit `Cargo.lock` for libraries** - Only for binaries
- **Skip tests** - Always run before pushing
- **Ignore clippy warnings** - Fix them or document why they're okay
- **Hardcode paths** - Use `PathBuf` and platform-agnostic code
- **Panic in library code** - Return `Result<T, E>` instead

---

## Pull Request Process

### PR Description Template

```markdown
## Summary
Brief description of changes

## Changes
- Added X feature
- Fixed Y bug
- Optimized Z function

## Testing
- [ ] Unit tests added/updated
- [ ] Integration tests pass
- [ ] Manual testing completed
- [ ] Clippy warnings resolved

## Performance Impact
[If applicable] Benchmark results or profiling data

## Breaking Changes
[If applicable] List any breaking changes and migration path

## Related Issues
Closes #123
```

---

## .gitignore Essentials

```bash
# Rust
target/
Cargo.lock  # Only for libraries; commit for binaries

# IDE
.vscode/
.idea/
*.swp
.DS_Store

# Testing
*.profdata

# Build artifacts
*.dSYM/
flamegraph.svg
```

---

## Emergency Commands

### Undo Last Commit (Keep Changes)

```bash
git reset --soft HEAD~1
```

### Revert to Last Working Build

```bash
git log --oneline
git checkout <commit-hash>
cargo build --release
```

### Clean Build Artifacts

```bash
cargo clean
```

---

## Resources

- **Rust Book:** https://doc.rust-lang.org/book/
- **Cargo Book:** https://doc.rust-lang.org/cargo/
- **Clippy Lints:** https://rust-lang.github.io/rust-clippy/
- **SemVer:** https://semver.org

---

**Note:** This workflow header is auto-generated from `~/.claude/templates/GIT_WORKFLOW_CLI.md`.
To update across all projects, run: `~/.claude/scripts/sync-git-workflow.sh`

---

<!-- END AUTO-GENERATED GIT WORKFLOW HEADER -->
# batteryctl - Battery health and power management

> macOS command-line utility (Rust)

## Project Overview

**Type:** Command-line utility (Rust)
**Purpose:** Battery health and power management
**Platform:** macOS (Apple Silicon & Intel)

## Features

- Battery health monitoring
- Charge cycle tracking
- Charging status and rate
- Power consumption analysis
- Battery temperature monitoring
- Estimated runtime calculations

## Development

```bash
# Build
cargo build

# Run
cargo run -- [ARGS]

# Test
cargo test

# Format and lint
cargo fmt
cargo clippy

# Build release
cargo build --release
```

## Installation

```bash
# Install locally
cargo install --path .

# Build and copy to system
cargo build --release
cp target/release/batteryctl /usr/local/bin/
```

## Usage

```bash
# Show help
batteryctl --help

# Show version
batteryctl --version
```
