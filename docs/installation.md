# Installation

## Requirements

| Component | Version / notes |
|-----------|----------------|
| Rust | ≥ 1.85 (`rustup` stable recommended) |
| mold linker | Required on Linux x86_64 (`.cargo/config.toml` sets `-fuse-ld=mold`) |
| Network | Needed for `refresh`, `search`, and the background poller (Yahoo / brapi) |
| Offline commands | `add`, `list`, `delete`, `export`, `import` work without network |

## Install from GitHub Releases

Tagged releases publish archives and installers via [cargo-dist](https://github.com/axodotdev/cargo-dist).

1. Open [Releases](https://github.com/carvalhosauro/local-ticker-wallet/releases).
2. Download the archive for your platform (`x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, or `aarch64-apple-darwin`).
3. Extract and place `ltw` on your `PATH`.

A shell installer script is attached to each release when available.

### Homebrew (macOS / Linux)

```bash
brew install carvalhosauro/tap/ltw
```

Formulas are published to [carvalhosauro/homebrew-tap](https://github.com/carvalhosauro/homebrew-tap) on release.

## Build from source

```bash
git clone https://github.com/carvalhosauro/local-ticker-wallet.git
cd local-ticker-wallet
cargo build --release
# binary: target/release/ltw

# Or install into ~/.cargo/bin
cargo install --path .
```

On Debian/Ubuntu, install mold if needed:

```bash
sudo apt install mold
```

## Data locations

Paths follow [XDG Base Directory](https://specifications.freedesktop.org/basedir-spec/basedir-spec-latest.html) conventions:

| Resource | Default path |
|----------|--------------|
| Database | `$XDG_DATA_HOME/local-ticker-wallet/wallet.db` |
| Config | `$XDG_CONFIG_HOME/local-ticker-wallet/config.json` |
| Unix socket | `$XDG_RUNTIME_DIR/local-ticker-wallet.sock` (falls back to temp dir) |

Override `XDG_DATA_HOME`, `XDG_CONFIG_HOME`, and `XDG_RUNTIME_DIR` to isolate a throwaway wallet (tests do this).

## Configuration

On first run the daemon creates `config.json` if it is missing:

```json
{
  "brapi_token": null,
  "poll_interval_secs": 60,
  "score_weights": {
    "proximity_low": 25,
    "below_sma": 20,
    "drawdown": 15,
    "dividend_yield": 20,
    "cost_vs_trend": 20
  },
  "locale": "pt-BR"
}
```

| Field | Purpose |
|-------|---------|
| `brapi_token` | Optional API token for [brapi.dev](https://brapi.dev) fallback |
| `poll_interval_secs` | Quote poll interval during B3 trading hours |
| `score_weights` | Weights for the opportunity score sub-components |
| `locale` | `pt-BR` or `en` — TUI strings and number formatting |

Edit the file while the daemon is stopped, or restart the daemon after changes.

## Running the daemon

### Interactive use

For normal terminal use, just run CLI or TUI commands — the client auto-spawns `ltw daemon` when the socket is missing.

### Running in scripts

The CLI spawns the daemon as a child that **inherits the caller's stdout/stderr**. If stdout is a pipe (e.g. `$(ltw list)`, `Command::output()` in tests), the child never sees EOF and the caller blocks forever.

**Reliable pattern:**

```bash
ltw daemon >/tmp/ltw-daemon.log 2>&1 &
sleep 0.5   # wait for socket
ltw add PETR4 100 28.50 --date 2026-01-02
ltw list
```

Stop the daemon with `pkill -f 'ltw daemon'` or by killing the background job.

## Verify installation

```bash
ltw daemon >/tmp/ltw-daemon.log 2>&1 &
ltw add PETR4 1 1.00 --date 2026-01-01
ltw list
ltw tui   # interactive — press q to quit
```
