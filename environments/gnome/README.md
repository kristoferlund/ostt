# GNOME Setup

## Quick Setup

Bind `ostt launch -c` to a global hotkey using GNOME's built-in custom shortcuts. No third-party tools required.

### Install OSTT

```bash
curl -fsSL https://ostt.ai/install | bash
ostt auth
```

Prefer a normal package install? On supported x86_64 Linux distributions, install the release package with your system package manager instead:

```bash
# Debian, Ubuntu, Mint, Pop!_OS
curl -sLO https://github.com/kristoferlund/ostt/releases/latest/download/ostt_latest_amd64.deb
sudo apt install ./ostt_latest_amd64.deb

# Fedora, RHEL, Rocky Linux
sudo dnf install https://github.com/kristoferlund/ostt/releases/latest/download/ostt-latest.x86_64.rpm
```

The package-manager route installs OSTT as a system package, installs declared dependencies, and allows normal uninstall commands such as `sudo apt remove ostt` or `sudo dnf remove ostt`.

### Bind to a Hotkey

1. Open **Settings** > **Keyboard** > **View and Customize Shortcuts**
2. Scroll to the bottom and click **Custom Shortcuts**
3. Click **+** (Add Shortcut)
4. Fill in:
   - **Name:** `OSTT`
   - **Command:** `ostt launch -c`
5. Click **Set Shortcut** and press your desired key combination (e.g. `Alt+Space` or `Ctrl+Alt+R`)
6. Click **Add**

That's it. Press the hotkey from any application to start recording.

### Usage

1. **Press your hotkey** — a popup terminal opens and recording starts
2. **Speak**
3. **Press the hotkey again** — recording stops, transcription runs, result is copied to clipboard
4. **Ctrl+V** — paste the transcription

The toggle works because pressing the hotkey a second time sends a signal to the running OSTT process, which finishes the recording. You never need to focus the popup window.

### Multiple Hotkeys

Add additional custom shortcuts for different workflows:

| Name | Command | Hotkey |
|------|---------|--------|
| OSTT | `ostt launch -c` | `Alt+Space` |
| OSTT Process | `ostt launch -c -p` | `Alt+Ctrl+Space` |
| OSTT Translate | `ostt launch -c -p translate` | `Ctrl+Alt+T` |

## Terminal Selection

By default, `ostt launch` auto-detects a terminal emulator. On a stock GNOME desktop, it will find gnome-terminal which works but shows a titlebar on the popup window.

For a cleaner look (borderless popup), install one of the preferred terminals:

```bash
# Any one of these — Ghostty is recommended
sudo apt install -y ghostty     # if available in your repos
sudo apt install -y kitty
sudo apt install -y alacritty
```

Then optionally set it in `~/.config/ostt/ostt.toml`:

```toml
[popup]
terminal = "kitty"
```

If not set, OSTT auto-detects a supported terminal. See the main README for the shared popup configuration.

## Popup Behavior

GNOME Wayland ignores client-side window positioning. OSTT popup size works as expected, but GNOME decides where the window appears.

## Troubleshooting

### Popup Not Appearing

```bash
# Verify ostt is installed
which ostt

# Check which terminal will be used
ostt launch -c
# If it fails, check the error message for which terminal it tried

# Test with a specific terminal
ostt launch -c   # auto-detect
```

### No Transcription in Clipboard

Make sure `wl-clipboard` (Wayland) or `xclip` (X11) is installed:

```bash
# Check which display server you're on
echo $XDG_SESSION_TYPE

# Install the right clipboard tool
sudo apt install -y wl-clipboard   # for wayland
sudo apt install -y xclip          # for x11
```

### Hotkey Not Working

GNOME custom shortcuts sometimes need a logout/login to take effect. If the shortcut doesn't work immediately, try logging out and back in.

Also verify the command works when run directly from a terminal first:

```bash
ostt launch -c
```

### Window Has a Titlebar

The default gnome-terminal always shows a titlebar. Install kitty, Ghostty, or alacritty for a borderless popup. See [Terminal Selection](#terminal-selection) above.
