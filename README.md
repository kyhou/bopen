# bopen

Terminal-based browser launcher for Linux with profile and container support.

![bopen screenshot](https://github.com/user-attachments/assets/13ccfbf6-8916-4706-b162-9cae1ad5ddec)

## Install

```bash
# Quick install (recommended)
curl -sSL https://raw.githubusercontent.com/kyhou/bopen/main/install.sh | sh

# Options
curl ... | sh -s -- -d /usr/local/bin    # custom directory
curl ... | sh -s -- --skip-desktop        # skip desktop file
curl ... | sh -s -- --version v1.1.0     # specific version
curl ... | sh -s -- -y                    # skip confirmations

# From source (requires Rust)
git clone https://github.com/kyhou/bopen.git && cd bopen
cargo build --release && sudo cp target/release/bopen /usr/local/bin/
```

## Usage

```bash
bopen                    # open launcher
bopen https://...        # pre-fill URL
```

### URL Pattern Matching

bopen supports automatic browser/profile selection based on URL patterns:

```bash
# If a pattern matches, browser opens directly without TUI
bopen "https://github.com/user/repo"
```

Configure patterns in the [Pattern Manager](#pattern-manager) (Ctrl+P).

## Controls

### Main Screen

| Key | Action |
|-----|--------|
| `TAB` / `Arrows` | Navigate |
| `ENTER` | Select / Open |
| `c` | Copy URL |
| `i` | Toggle incognito |
| `w` | Toggle new window |
| `Ctrl+P` | Open Pattern Manager |
| `q` | Quit |

### Pattern Manager

| Key | Action |
|-----|--------|
| `a` | Add new pattern |
| `e` | Edit selected pattern |
| `d` / `Delete` | Delete pattern |
| `↑` / `↓` | Navigate patterns |
| `q` / `Esc` | Close |
| `Ctrl+C` | Close (alternative) |

### Text Editing (URL field, Pattern field)

| Key | Action |
|-----|--------|
| `←` / `→` | Move cursor |
| `Backspace` | Delete before cursor |
| `Delete` | Delete after cursor |
| Type | Insert at cursor position |

## Features

- **Automatic browser detection** — Discovers all installed browsers
- **Profile selection** — Firefox, Chromium, Chrome, Brave, Edge, etc.
- **Firefox containers** — Personal, Work, Banking, Shopping
- **URL Pattern Matching** — Auto-launch browsers based on URL patterns
- **Pattern Manager** — TUI for managing URL patterns (Ctrl+P)
- **Incognito and new window** — Quick toggle options
- **Persistent configuration** — Saves preferences and patterns

## Pattern Manager

Access the Pattern Manager by pressing `Ctrl+P` in the main UI.

### Pattern Format

Patterns use regular expressions to match URLs:

```
Pattern:    .*github\.com.*
Browser:    Firefox
Profile:    work
Container:  (optional, Firefox only)
Options:    [ ] Private  [ ] New Window
```

### Example Patterns

| Pattern | Browser | Profile | Container | Use Case |
|---------|---------|---------|-----------|----------|
| `.*github\.com.*` | Firefox | work | | Work repositories |
| `.*youtube\.com.*` | Chrome | Personal | | Entertainment |
| `.*bank.*` | Firefox | default | Banking | Secure banking |
| `.*reddit\.com.*` | Brave | default | | Private browsing |

## Configuration

Configuration is stored in `~/.config/bopen/config.json`:

```json
{
  "last_browser": "Firefox",
  "last_profile": "Personal",
  "url_patterns": [
    {
      "pattern": ".*github\\.com.*",
      "browser": "Firefox",
      "profile": "work",
      "container": null,
      "incognito": false,
      "new_window": false
    }
  ]
}
```

## Set as Default Browser

```bash
curl ... | sh  # installer asks this automatically

# or manually:
cp data/bopen.desktop ~/.local/share/applications/
xdg-mime default bopen.desktop x-scheme-handler/http
xdg-mime default bopen.desktop x-scheme-handler/https
```

## Uninstall

```bash
rm ~/.local/bin/bopen        # (or your install path)
rm -rf ~/.config/bopen
rm ~/.local/share/applications/bopen.desktop
```

## License

MIT
