# omarchy-batch-monitor

`obm` is a fast, keyboard-driven terminal UI for monitoring
[OpenAI Batch API](https://developers.openai.com/api/docs/guides/batch) job
statuses, built for [Omarchy](https://omarchy.org/) with Rust and
[Ratatui](https://ratatui.rs). It's a plain terminal app, so it runs anywhere,
but it ships an Omarchy app-launcher entry and floats nicely as a pane.

## Features

- Lists all batches (auto-paginated), sorted by creation time or status
- Auto-refreshes on an interval (default 15s), with a live countdown
- Color-coded statuses and per-status summary counts in the header
- Detail panel for the selected batch: timestamps, request counts/progress,
  file IDs, errors, and metadata
- Cancel a running batch (`validating` / `in_progress` / `finalizing`) with a confirm prompt
- Non-blocking: the UI stays responsive while HTTP requests are in flight

## Install

### AUR

```sh
git clone https://github.com/gjlaubenstein/openai-batch-monitor-omarchy.git
cd openai-batch-monitor-omarchy/packaging/aur
makepkg -si
```

(Once published to the AUR proper: `yay -S omarchy-batch-monitor` or
`omarchy pkg aur add omarchy-batch-monitor`.)

### Prebuilt binary

Download the latest `omarchy-batch-monitor-*-x86_64-linux-gnu.tar.gz` from
[Releases](https://github.com/gjlaubenstein/openai-batch-monitor-omarchy/releases),
verify the checksum, then:

```sh
tar xzf omarchy-batch-monitor-*-x86_64-linux-gnu.tar.gz
cd omarchy-batch-monitor-*/
install -Dm755 bin/obm ~/.local/bin/obm
install -Dm644 share/applications/org.omarchy.obm.desktop ~/.local/share/applications/org.omarchy.obm.desktop
install -Dm644 share/icons/hicolor/scalable/apps/org.omarchy.obm.svg ~/.local/share/icons/hicolor/scalable/apps/org.omarchy.obm.svg
```

### From source

```sh
cargo build --release
# binary at target/release/obm
```

## Configure

The API key is read from, in order of precedence:

1. `--api-key <key>` CLI flag
2. `OPENAI_API_KEY` environment variable
3. `api_key` in the config file at `~/.config/omarchy-batch-monitor/config.toml`

Optional config file:

```toml
api_key = "sk-..."
organization = "org-..."   # optional, sets OpenAI-Organization header
project = "proj_..."       # optional, sets OpenAI-Project header
refresh_secs = 15          # optional, default 15
```

`OPENAI_ORG_ID` / `OPENAI_PROJECT_ID` env vars are also honored. `OPENAI_API_BASE` can
override the API base URL (useful for a proxy or compatible endpoint).

## Usage

Launch from the Omarchy application launcher (search "Batch Monitor"), or run:

```sh
OPENAI_API_KEY=sk-... obm
# or
obm --api-key sk-... --refresh 30
```

| Key           | Action                              |
|---------------|--------------------------------------|
| `j` / `↓`     | select next batch                   |
| `k` / `↑`     | select previous batch               |
| `g` / `Home`  | select first                        |
| `G` / `End`   | select last                         |
| `r`           | refresh now                         |
| `c`           | cancel selected batch (with confirm)|
| `s`           | toggle sort (created / status)      |
| `?`           | toggle help                         |
| `q` / `Esc`   | quit                                |

## Running as an Omarchy pane

The installed `.desktop` entry launches `obm` via Omarchy's
`omarchy-launch-or-focus-tui`, which opens it in your default terminal under
app-id `org.omarchy.obm` (or focuses the existing window if it's already
open). To have it float as a centered pane, add a window rule to
`~/.config/hypr/hyprland.lua`:

```lua
o.window("^org\\.omarchy\\.obm$", {
  float = true,
  center = true,
  size = { 1000, 650 },
  persistent_size = true,
})
```

Optionally bind a hotkey to it in `~/.config/hypr/bindings.lua`:

```lua
o.bind("SUPER + B", "Batch monitor", "omarchy-launch-or-focus-tui obm")
```

## Local data

Preferences live in `~/.config/omarchy-batch-monitor/config.toml`. No batch
data or credentials are cached to disk; the API key lives only in your
environment or that config file.

## Development

The repository pins Rust 1.88 and can bootstrap it with Mise:

```sh
mise install
mise exec -- cargo fmt --all -- --check
mise exec -- cargo clippy --all-targets --all-features -- -D warnings
mise exec -- cargo test --all-features --locked
```

## Release process

1. Update the version in `Cargo.toml` and `packaging/aur/PKGBUILD`/`.SRCINFO`.
2. Run formatting, Clippy, tests, and `scripts/package-release.sh`.
3. Tag the commit as `vX.Y.Z` and push the tag; GitHub Actions publishes the tarball and checksum.
4. Replace the AUR `sha256sums` value with the published source archive checksum.
5. Regenerate `.SRCINFO` with `makepkg --printsrcinfo > .SRCINFO`, then push the package to AUR.

The project intentionally does not edit `~/.config/omarchy/` or anything under
`/usr/share/omarchy/`.

## License

GPL-3.0-only. See [LICENSE](LICENSE).
