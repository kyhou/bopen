# bopen

Terminal-based browser launcher for Linux with profile and container support.

![bopen screenshot](https://github.com/user-attachments/assets/35cc67fc-a215-48b1-a255-9dcca1ddd7c3)

## Install

```bash
# Quick install (recommended)
curl -sSL https://raw.githubusercontent.com/kyhou/bopen/main/install.sh | sh

# Options
curl ... | sh -s -- -d /usr/local/bin    # custom directory
curl ... | sh -s -- --skip-desktop        # skip desktop file
curl ... | sh -s -- --version v0.1.0     # specific version
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

## Controls

| Key | Action |
|-----|--------|
| `TAB` / `Arrows` | Navigate |
| `ENTER` | Select / Open |
| `c` | Copy URL |
| `i` | Toggle incognito |
| `w` | Toggle new window |
| `q` | Quit |

## Features

- Automatic browser detection
- Profile selection (Firefox, Chromium, Chrome, Brave, Edge, etc.)
- Firefox containers (Personal, Work, Banking, Shopping)
- Incognito and new window options
- Persistent configuration

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
