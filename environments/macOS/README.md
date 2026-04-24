# macOS Setup

## Quick Setup (Shortcuts.app)

The recommended way to set up ostt on macOS. No third-party tools required.

### Prerequisites

1. **Install ostt:**
   ```bash
   brew install kristoferlund/ostt/ostt
   ```

2. **Install a terminal emulator** — ostt auto-detects from: Ghostty, kitty, alacritty. At least one must be installed. [Ghostty](https://ghostty.org/) is recommended.

3. **Run initial setup:**
   ```bash
   ostt auth
   ```

### Bind to a Hotkey

1. Open **Shortcuts.app** (search "Shortcuts" in Spotlight)
2. Click **+** to create a new shortcut
3. Search for **"Run Shell Script"** in the actions panel and add it
4. Replace the default script text with:
   ```
   ostt launch -c
   ```
5. Name the shortcut (click the title at top), e.g. **"OSTT"**
6. Right-click the shortcut in the sidebar (or click the **(i)** button) and select **Add Keyboard Shortcut**
7. Press your desired key combination, e.g. `Option+Space`

That's it. Press the hotkey from any application to start recording.

### Usage

1. **Press your hotkey** — a popup terminal opens and recording starts
2. **Speak**
3. **Press the hotkey again** — recording stops, transcription runs, result is copied to clipboard
4. **Cmd+V** — paste the transcription

The toggle works because pressing the hotkey a second time sends a signal (SIGUSR1) to the running ostt process, which finishes the recording. You never need to focus the popup window.

### Multiple Shortcuts

Create additional shortcuts in Shortcuts.app for different workflows:

| Shortcut | Shell command | What it does |
|----------|--------------|--------------|
| OSTT | `ostt launch -c` | Record, transcribe, copy |
| OSTT Clean | `ostt launch -c -p clean` | Record, transcribe, clean up text, copy |
| OSTT Translate | `ostt launch -c -p translate` | Record, transcribe, translate, copy |

Each shortcut gets its own keyboard binding.

## Configuration

### Popup Window Settings

Configure the popup window size, position, and terminal in `~/.config/ostt/ostt.toml`:

```toml
[popup]
# Terminal emulator to use. Auto-detected if not set.
# Setting this skips auto-detection (faster startup).
#
# Preferred (recommended, cross-platform):
#   ghostty, kitty, alacritty
#
# Platform defaults (used as fallback if none of the above are found):
#   foot, konsole, gnome-terminal, xfce4-terminal
#
# On macOS, the default Terminal.app does not support true color and
# is not suitable for ostt. Install one of the preferred terminals.
# terminal = "ghostty"

# Window position (pixels from top-left corner)
x = 630
y = 790

# Window size (terminal columns and rows)
width = 50
height = 10

# Font size
font_size = 8

# Hide window decorations (titlebar, borders)
borderless = true
```

### Output Options

The `ostt launch` command passes arguments through to ostt:

```bash
ostt launch -c              # Copy to clipboard
ostt launch -o file.txt     # Write to file
ostt launch                 # Output to stdout (not useful in popup)
```

## Advanced: Hammerspoon Integration

For power users who want more control (multiple hotkeys with different window sizes, app-switching behavior, etc.), a Hammerspoon configuration is available.

See [init.lua](init.lua) for a template configuration.

### Hammerspoon Setup

1. Install [Hammerspoon](https://www.hammerspoon.org/): `brew install --cask hammerspoon`
2. Copy the contents of [init.lua](init.lua) to `~/.hammerspoon/init.lua`
3. Reload Hammerspoon (menu bar icon > Reload Config)

The Hammerspoon template includes:
- Multiple hotkeys with different ostt arguments
- Toggle behavior (press hotkey again to finish recording)
- Automatic focus restoration to the previous app
- Per-hotkey window size/position overrides

## Troubleshooting

### Popup Not Appearing

```bash
# Verify ostt is installed
which ostt

# Verify a supported terminal is installed
which ghostty || which kitty || which alacritty

# Test the launch command directly
ostt launch -c
```

### No Transcription in Clipboard

Make sure `-c` is in the shell command. Without it, output goes to stdout which isn't visible in the popup.

### Terminal Not Found

Set the terminal explicitly in `~/.config/ostt/ostt.toml`:

```toml
[popup]
terminal = "ghostty"
```

### Popup Not Working with Full-Screen Apps

macOS full-screen apps run in their own Space. Other windows cannot appear on top. Switch out of full-screen mode or use a different Space.
