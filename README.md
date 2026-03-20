# bopen

A fast, terminal-based browser launcher for Linux that makes opening URLs with the right browser, profile, and container effortless.

![bopen demo](https://i.imgur.com/example.gif)

## Features

- **Automatic browser detection** - Scans `.desktop` files to find installed browsers
- **Profile management** - Support for Firefox, Chromium, and derivatives (Chrome, Brave, Edge, Vivaldi, etc.)
- **Container support** - Firefox Containers (Personal, Work, Banking, Shopping)
- **Privacy options** - Incognito/private browsing toggle
- **Window control** - Open in new window option
- **Clipboard integration** - Copy URLs to clipboard
- **Keyboard-driven TUI** - Vim-like navigation
- **Persistent configuration** - Remembers your default choices

## Installation

### Prerequisites

Install Rust and Cargo (if not already installed):

```bash
# On Debian/Ubuntu
sudo apt install rustc cargo

# On Fedora
sudo dnf install rust cargo

# On Arch Linux
sudo pacman -S rust cargo

# Or use rustup for the latest version
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Compile and Install

```bash
# Clone the repository
git clone https://github.com/kyhou/bopen.git
cd bopen

# Build the release version
cargo build --release

# Install the binary
sudo cp target/release/bopen /usr/local/bin/

# Verify installation
bopen --version
```

### Uninstall

```bash
sudo rm /usr/local/bin/bopen
rm -rf ~/.config/bopen
rm ~/.local/share/applications/bopen.desktop
```

### Set as Default Browser

To use bopen as your default browser, install the `.desktop` file and register the MIME types:

```bash
# Install the desktop file
cp data/bopen.desktop ~/.local/share/applications/

# Register as default browser for HTTP/HTTPS
xdg-mime default bopen.desktop x-scheme-handler/http
xdg-mime default bopen.desktop x-scheme-handler/https

# Optional: Also handle HTML files
xdg-mime default bopen.desktop text/html
```

To verify the registration:
```bash
xdg-mime query default x-scheme-handler/http
xdg-mime query default x-scheme-handler/https
```

To revert to another browser:
```bash
xdg-mime default firefox.desktop x-scheme-handler/http
xdg-mime default firefox.desktop x-scheme-handler/https
```

### Dependencies

- `arboard` (clipboard support)
- `ratatui` (TUI framework)
- `crossterm` (terminal handling)

These are included as Cargo dependencies and will be installed automatically.

## Usage

```bash
# Open the launcher
bopen

# Pre-fill a URL
bopen https://example.com
```

## Controls

| Key | Action |
|-----|--------|
| `TAB` / `Arrows` | Navigate between fields |
| `ENTER` | Select dropdown item / Open URL |
| `c` | Copy URL to clipboard |
| `i` | Toggle incognito/private mode |
| `w` | Toggle new window |
| `q` | Quit |

## Configuration

On first run, bopen creates a config file at `~/.config/bopen/config.json` with your selected browser, profile, and options. These settings are remembered for subsequent runs.

## How It Works

1. **Browser Discovery** - Scans standard Linux desktop file locations:
   - `/usr/share/applications/`
   - `/usr/local/share/applications/`
   - `~/.local/share/applications/`

2. **Profile Detection** - Automatically detects browser profiles:
   - Firefox: Parses `profiles.ini` and reads containers from `containers.json`
   - Chromium-based: Reads from `Preferences` files

3. **Launch** - Constructs the appropriate command with flags for the selected browser, profile, and container

## Supported Browsers

### Firefox-based
- Firefox
- LibreWolf
- Waterfox

### Chromium-based
- Google Chrome
- Chromium
- Brave
- Microsoft Edge
- Vivaldi
- Opera

## License

MIT
