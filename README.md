# omarchy-batch-monitor

A terminal UI for monitoring [OpenAI Batch API](https://developers.openai.com/api/docs/guides/batch)
job statuses, built with Rust and [Ratatui](https://ratatui.rs). Designed to live in a
terminal pane on an Omarchy/Hyprland desktop, but it's a plain terminal app and runs anywhere.

## Features

- Lists all batches (auto-paginated), sorted by creation time or status
- Auto-refreshes on an interval (default 15s), with a live countdown
- Color-coded statuses and per-status summary counts in the header
- Detail panel for the selected batch: timestamps, request counts/progress,
  file IDs, errors, and metadata
- Cancel a running batch (`validating` / `in_progress` / `finalizing`) with a confirm prompt
- Non-blocking: the UI stays responsive while HTTP requests are in flight

## Build

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

## Run

```sh
OPENAI_API_KEY=sk-... obm
# or
obm --api-key sk-... --refresh 30
```

## Keybindings

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

## Running as an Omarchy/Hyprland pane

Any terminal emulator works. For a dedicated floating pane, bind a keystroke to spawn
your terminal running `obm`, e.g. in `~/.config/hypr/bindings.conf`:

```
bind = SUPER, B, exec, uwsm app -- $TERMINAL --class omarchy-batch-monitor -e obm
```

and a matching window rule in `~/.config/hypr/windows.conf` to float/size it, e.g.:

```
windowrule = float, class:^(omarchy-batch-monitor)$
windowrule = size 900 600, class:^(omarchy-batch-monitor)$
windowrule = center, class:^(omarchy-batch-monitor)$
```

Adjust `$TERMINAL` / class name to match your terminal emulator's flags for setting a
window class (e.g. `alacritty --class ... -e`, `kitty --class ... -e`, `foot -a ... `).
