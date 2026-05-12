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
brew tap kristoferlund/ostt
brew install ostt
ostt auth
```

The Homebrew route installs OSTT as a managed package and allows normal uninstall with `brew uninstall ostt`.

### Locate the Binary

Shortcuts.app requires the full path to the OSTT binary. Find it with:

```bash
which ostt
```

Typical locations:

| Install method | Path |
| --- | --- |
| Install script | `~/.local/bin/ostt` |
| Homebrew (Intel) | `/usr/local/bin/ostt` |
| Homebrew (Apple Silicon) | `/opt/homebrew/bin/ostt` |

For popup mode, OSTT auto-detects Ghostty, kitty, or Alacritty. The installer warns you if none are installed. [Ghostty](https://ghostty.org/) is recommended.

### Bind to a Hotkey

1. Open **Shortcuts.app** (search "Shortcuts" in Spotlight)
2. Click **+** to create a new shortcut
3. Search for **"Run Shell Script"** in the actions panel and add it
4. Replace the default script text with the full path to OSTT:
   ```
   /opt/homebrew/bin/ostt launch -c
   ```
   Use the path from `which ostt` if yours differs.

5. Name the shortcut (click the title at top), e.g. **"OSTT"**
6. Open the shortcut details (click the info button, the encircled **i**) and select **Add Keyboard Shortcut**
7. Press **Control+Space**

> **Note:** `Control+Space` is the suggested default for macOS because many other key combinations are reserved by the system. If you choose a different shortcut, verify it does not collide with an existing system or application hotkey. The suggested hotkey differs from the one used on Linux platforms because macOS reserves more keyboard shortcuts at the system level.

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
| OSTT | `/path/to/ostt launch -c` | Record, transcribe, copy |
| OSTT Clean | `/path/to/ostt launch -c -p clean` | Record, transcribe, clean up text, copy |
| OSTT Translate | `/path/to/ostt launch -c -p translate` | Record, transcribe, translate, copy |

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
