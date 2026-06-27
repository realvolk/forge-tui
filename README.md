# forge-tui

A custom terminal UI library for ArtixForge, built in Rust with `ratatui` + `crossterm`.

Replaces `gum` as the TUI backend while sharing the exact same JSON contract as `forge-gui`.

---

## Why

`gum` served well but hit its limits:

* Everything renders top-left — no centering, no layout control
* No multi-column tables, colored rows, or inline metadata
* No contextual info panels that update on selection
* No progress bar with stage labels
* Hardcoded `/dev/tty` redirections fail on some `dinit` consoles
* No mouse support

`forge-tui` fixes all of this in a single static binary with no runtime dependencies.

---

# Quick Start

## Build

```bash
cargo build --release
```

## Test a Widget

```bash
echo '{
  "widget":"menu",
  "title":"Filesystem",
  "message":"Pick one",
  "choices":["ext4","btrfs","xfs"]
}' | ./target/release/forge-tui --mode widget
```

---

# Supported Widgets

| Widget    | JSON Key    | Description                                             |
| --------- | ----------- | ------------------------------------------------------- |
| Menu      | `menu`      | Single-selection list with keyboard and mouse           |
| Yes/No    | `yesno`     | Boolean confirmation prompt                             |
| Input     | `input`     | Free-form text entry with optional validation           |
| Password  | `password`  | Masked text entry                                       |
| Checklist | `checklist` | Multi-select with toggle, min/max limits                |
| Message   | `msg`       | Informational dialog, any key dismisses                 |
| Summary   | `summary`   | Scrollable text display with scrollbar                  |
| Filter    | `filter`    | Fuzzy search with live filtering                        |
| Progress  | `progress`  | Live command output with progress bar and stage markers |

---

# JSON Contract

`forge-tui` speaks the exact same JSON-in / JSON-out protocol as `forge-gui`.

Swap between TUI, GUI, and non-interactive mode with zero backend changes.

## Request

```json
{
  "widget": "menu",
  "title": "Filesystem",
  "message": "Pick one",
  "choices": ["ext4", "btrfs", "xfs"]
}
```

## Response

```json
{
  "result": "btrfs",
  "cancelled": false
}
```

---

# Integration

```bash
# Bash wrapper — identical function signatures to gum

tui_menu() {
    local title="$1"
    local msg="$2"
    shift 2

    local choices_json
    choices_json=$(
        printf '%s\n' "$@" |
        jq -R . |
        jq -s .
    )

    forge-tui --mode widget <<< "{
        \"widget\":\"menu\",
        \"title\":\"$title\",
        \"message\":\"$msg\",
        \"choices\":$choices_json
    }" | jq -r '.result // empty'
}
```

---

# Features

* **Proper layout engine**
  Everything is centered instead of top-left stacking.

* **Mouse support**
  Click and scroll in menus, checklists, and summary views.

* **Progress bar**
  Tracks stage markers in command output:

  ```text
  [*] Preflight dependencies installed.
  [*] Mount setup completed.
  ```

* **Tab toggle**
  Switch between progress-bar view and raw command output during installation.

* **Filter widget**
  Fuzzy search with live filtering.

* **Theme support**
  Respects:

  ```text
  GUM_TITLE_COLOR
  GUM_ACCENT_COLOR
  ```

* **Single static binary**
  No runtime dependencies. Works in raw TTY (no X11 / Wayland required).

* **No allocations on the hot path**
  Smooth progress updates even during heavy package installs.

---

# Architecture

```text
┌──────────────┐     JSON stdin      ┌──────────────┐     JSON stdout     ┌──────────────┐
│   core.sh    │ ──────────────────▶ │  forge-tui   │ ──────────────────▶ │   core.sh    │
│   (Bash)     │                     │   (Rust)     │                     │   (Bash)     │
│              │                     │              │                     │              │
│ constructs   │                     │ ratatui      │                     │ parses       │
│ JSON, pipes  │                     │ renders TUI  │                     │ response,    │
│ to forge-tui │                     │ to /dev/tty  │                     │ returns value│
└──────────────┘                     └──────────────┘                     └──────────────┘
```

### Data Flow

* JSON flows through a secure FIFO in a `chmod 700` directory — never touches disk
* TUI renders directly to `/dev/tty` — works over SSH, inside `tmux`, and on raw consoles
* Response goes to `stdout` — captured by Bash wrappers using `$()`

---

# Dependencies

## Build

* Rust (stable)
* Cargo

## Runtime

None — single static binary.

---

# License

See [LICENSE](LICENSE)
Licensed under the **Forge Attribution License 1.0**
© Volk 2026
