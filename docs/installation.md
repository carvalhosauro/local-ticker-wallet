# Installation

Choose the method that fits your system. Every option installs the **`ltw`** command.

## Quick reference

| Method | Command | Best for |
|--------|---------|----------|
| **APT** (Debian/Ubuntu) | `curl -fsSL …/install-apt.sh \| sudo sh` | Easiest on Linux — system-wide `/usr/bin/ltw` |
| **Shell** (curl) | `curl -LsSf …/ltw-installer.sh \| sh` | Linux/macOS without root — installs to `~/.cargo/bin` |
| **Homebrew** | `brew install carvalhosauro/tap/ltw` | macOS and Linux with Homebrew |
| **crates.io** | `cargo install ltw` | Rust developers |
| **GitHub Release** | download `.tar.xz` from [Releases](https://github.com/carvalhosauro/local-ticker-wallet/releases) | Manual / air-gapped |

---

## Debian / Ubuntu (APT)

**One command** — adds the repository and installs:

```bash
curl -fsSL https://carvalhosauro.github.io/local-ticker-wallet/install-apt.sh | sudo sh
```

After the repository is configured, updates are standard:

```bash
sudo apt update
sudo apt install ltw        # first install
sudo apt upgrade ltw        # upgrade to a new release
```

The package installs `ltw` to `/usr/bin/ltw`. Supports **amd64** and **arm64**.

<details>
<summary>Manual repository setup</summary>

```bash
echo 'deb [trusted=yes arch=amd64,arm64] https://carvalhosauro.github.io/local-ticker-wallet ./' \
  | sudo tee /etc/apt/sources.list.d/ltw.list
sudo apt update
sudo apt install ltw
```

</details>

> **Note:** The APT repository is published on [GitHub Pages](https://carvalhosauro.github.io/local-ticker-wallet/) when a release tag is pushed. Until the first release, use the shell installer or `cargo install` below.

---

## Shell installer (Linux & macOS)

No root required. Detects your CPU/OS, downloads the matching binary, and installs to `~/.cargo/bin`:

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/carvalhosauro/local-ticker-wallet/releases/latest/download/ltw-installer.sh | sh
```

Restart your shell, or run:

```bash
source "$HOME/.cargo/env"    # if rustup installed cargo's env helper
# or
source "$HOME/.profile"      # installer also updates ~/.profile
```

Requires `curl` (or `wget`), `tar`, and `unzip`.

---

## Homebrew

```bash
brew install carvalhosauro/tap/ltw
```

Formulas are published automatically to [carvalhosauro/homebrew-tap](https://github.com/carvalhosauro/homebrew-tap) on each release.

---

## crates.io

```bash
cargo install ltw
```

- Requires **Rust ≥ 1.85** (`rustup` recommended).
- Binary lands in `~/.cargo/bin` — ensure it is on your `PATH`.
- No `mold` linker needed for crates.io installs (only applies when building from a git checkout).

Upgrade to the latest release:

```bash
cargo install ltw --force
```

---

## GitHub Releases (manual)

1. Open [Releases](https://github.com/carvalhosauro/local-ticker-wallet/releases).
2. Download the archive for your platform:
   - `ltw-x86_64-unknown-linux-gnu.tar.xz`
   - `ltw-aarch64-unknown-linux-gnu.tar.xz`
   - `ltw-x86_64-apple-darwin.tar.xz`
   - `ltw-aarch64-apple-darwin.tar.xz`
3. Extract and move `ltw` to a directory on your `PATH` (e.g. `/usr/local/bin` or `~/.local/bin`).

---

## Build from source

For contributors or unreleased changes:

```bash
git clone https://github.com/carvalhosauro/local-ticker-wallet.git
cd local-ticker-wallet
cargo install --path .
```

On Debian/Ubuntu, install the **mold** linker (required by this repo's `.cargo/config.toml`):

```bash
sudo apt install mold
```

---

## Requirements

| Component | When needed |
|-----------|-------------|
| Network | `refresh`, `search`, background poller (Yahoo / brapi) |
| Offline | `add`, `list`, `delete`, `export`, `import` work without network |

## Data locations

Paths follow [XDG Base Directory](https://specifications.freedesktop.org/basedir-spec/basedir-spec-latest.html) conventions:

| Resource | Default path |
|----------|--------------|
| Database | `$XDG_DATA_HOME/local-ticker-wallet/wallet.db` |
| Config | `$XDG_CONFIG_HOME/local-ticker-wallet/config.json` |
| Unix socket | `$XDG_RUNTIME_DIR/local-ticker-wallet.sock` (falls back to temp dir) |

Override `XDG_DATA_HOME`, `XDG_CONFIG_HOME`, and `XDG_RUNTIME_DIR` to isolate a throwaway wallet.

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
sleep 0.5
ltw add PETR4 100 28.50 --date 2026-01-02
ltw list
```

Stop the daemon with `pkill -f 'ltw daemon'` or by killing the background job.

## Verify installation

```bash
command -v ltw
ltw add PETR4 1 1.00 --date 2026-01-01
ltw list
ltw tui   # press q to quit
```
