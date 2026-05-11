# macOS Setup

## Quick Setup (Shortcuts.app)

The recommended way to set up OSTT on macOS. No third-party tools required.

### Install OSTT

```bash
curl -fsSL https://ostt.ai/install | bash
ostt auth
```

Prefer a normal package-manager install? Use Homebrew instead:

```bash
brew install kristoferlund/ostt/ostt
ostt auth
```

The Homebrew route installs OSTT as a managed package and allows normal uninstall with `brew uninstall ostt`.

For popup mode, OSTT auto-detects Ghostty, kitty, or Alacritty. The installer warns you if none are installed. [Ghostty](https://ghostty.org/) is recommended.

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

The toggle works because pressing the hotkey a second time sends a signal (SIGUSR1) to the running OSTT process, which finishes the recording. You never need to focus the popup window.

### Multiple Shortcuts

Create additional shortcuts in Shortcuts.app for different workflows:

| Shortcut | Shell command | What it does |
|----------|--------------|--------------|
| OSTT | `ostt launch -c` | Record, transcribe, copy |
| OSTT Clean | `ostt launch -c -p clean` | Record, transcribe, clean up text, copy |
| OSTT Translate | `ostt launch -c -p translate` | Record, transcribe, translate, copy |

Each shortcut gets its own keyboard binding.

## Output Options

The `ostt launch` command passes arguments through to ostt:

```bash
ostt launch -c              # Copy to clipboard
ostt launch -o file.txt     # Write to file
ostt launch                 # Output to stdout (not useful in popup)
```

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
