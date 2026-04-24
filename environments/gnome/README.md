# GNOME Setup

## Quick Setup

Bind `ostt launch -c` to a global hotkey using GNOME's built-in custom shortcuts. No third-party tools required.

### Prerequisites

1. **Install ostt** (see main README for distribution-specific instructions)

2. **Install dependencies:**
   ```bash
   sudo apt install -y ffmpeg wl-clipboard   # Ubuntu/Debian (Wayland)
   sudo apt install -y ffmpeg xclip          # Ubuntu/Debian (X11)
   sudo dnf install -y ffmpeg wl-clipboard   # Fedora (Wayland)
   ```

3. **Run initial setup:**
   ```bash
   ostt auth
   ```

### Bind to a Hotkey

1. Open **Settings** > **Keyboard** > **View and Customize Shortcuts**
2. Scroll to the bottom and click **Custom Shortcuts**
3. Click **+** (Add Shortcut)
4. Fill in:
   - **Name:** `OSTT`
   - **Command:** `ostt launch -c`
5. Click **Set Shortcut** and press your desired key combination (e.g. `Super+Space` or `Ctrl+Alt+R`)
6. Click **Add**

That's it. Press the hotkey from any application to start recording.

### Usage

1. **Press your hotkey** — a popup terminal opens and recording starts
2. **Speak**
3. **Press the hotkey again** — recording stops, transcription runs, result is copied to clipboard
4. **Ctrl+V** — paste the transcription

The toggle works because pressing the hotkey a second time sends a signal to the running ostt process, which finishes the recording. You never need to focus the popup window.

### Multiple Hotkeys

Add additional custom shortcuts for different workflows:

| Name | Command | Hotkey |
|------|---------|--------|
| OSTT | `ostt launch -c` | `Super+Space` |
| OSTT Clean | `ostt launch -c -p clean` | `Super+Shift+Space` |
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

If not set, ostt auto-detects in this order: Ghostty > kitty > alacritty > foot > konsole > gnome-terminal > xfce4-terminal. The first one found is used.

## Popup Window Configuration

Configure window size, position, and appearance in `~/.config/ostt/ostt.toml`:

```toml
[popup]
# terminal = "kitty"

# Window position (pixels from top-left corner)
# Note: GNOME Wayland ignores client-side window positioning.
# The compositor decides where the window appears.
x = 630
y = 790

# Window size (terminal columns and rows)
width = 50
height = 10

# Font size
font_size = 8

# Hide window decorations (requires a terminal that supports it)
borderless = true
```

**GNOME Wayland note:** Window position (`x`, `y`) is ignored — GNOME Wayland does not allow applications to choose their window position. The window will appear wherever the compositor places it (usually centered or cascaded). Window size works as expected.

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
