# KDE Plasma Setup

## Quick Setup

Bind `ostt launch -c` to a global hotkey using KDE's built-in Custom Shortcuts. No third-party tools required.

### Install OSTT

```bash
curl -fsSL https://ostt.ai/install | bash
ostt auth
```

Prefer a normal package install? On supported x86_64 Linux distributions, install the release package with your system package manager instead:

```bash
# Debian, Ubuntu, Kubuntu
curl -sLO https://github.com/kristoferlund/ostt/releases/latest/download/ostt_latest_amd64.deb
sudo apt install ./ostt_latest_amd64.deb

# Fedora KDE, RHEL, Rocky Linux
sudo dnf install https://github.com/kristoferlund/ostt/releases/latest/download/ostt-latest.x86_64.rpm

# openSUSE
sudo zypper install https://github.com/kristoferlund/ostt/releases/latest/download/ostt-latest.x86_64.rpm
```

The package-manager route installs OSTT as a system package, installs declared dependencies, and allows normal uninstall commands such as `sudo apt remove ostt`, `sudo dnf remove ostt`, or `sudo zypper remove ostt`.

### Bind to a Hotkey

1. Open **System Settings** > **Shortcuts** > **Custom Shortcuts**
2. Click **Edit** > **New** > **Global Shortcut** > **Command/URL**
3. Name it **OSTT**
4. **Trigger tab:** click the shortcut button and press your desired key combination (e.g. `Alt+Space`)
5. **Action tab:** enter the command:
   ```
   ostt launch -c
   ```
   If KDE cannot find `ostt`, use the full path from `which ostt`.
6. Click **Apply**

That's it. Press the hotkey from any application to start recording.

### Usage

1. **Press your hotkey** — a popup terminal opens and recording starts
2. **Speak**
3. **Press the hotkey again** — recording stops, transcription runs, result is copied to clipboard
4. **Ctrl+V** — paste the transcription

The toggle works because pressing the hotkey a second time sends a signal to the running OSTT process, which finishes the recording. You never need to focus the popup window.

### Multiple Hotkeys

Add additional Custom Shortcuts for different workflows:

| Name | Command | Hotkey |
|------|---------|--------|
| OSTT | `ostt launch -c` | `Alt+Space` |
| OSTT Process | `ostt launch -c -p` | `Alt+Ctrl+Space` |
| OSTT Translate | `ostt launch -c -p translate` | `Ctrl+Alt+T` |

## Terminal Selection

By default, `ostt launch` auto-detects a terminal emulator. On a stock KDE desktop, it will find Konsole which works well.

For a borderless popup, install one of the preferred terminals:

```bash
sudo apt install -y kitty        # or alacritty, ghostty
```

Then optionally set it in `~/.config/ostt/ostt.toml`:

```toml
[popup]
terminal = "kitty"
```

If not set, OSTT auto-detects a supported terminal. See the main README for the shared popup configuration.

### Window Rules (Optional)

KDE supports window rules for fine-grained control over popup appearance. If you use kitty, alacritty, or foot (which set window class `ostt-popup`):

1. **System Settings** > **Window Management** > **Window Rules**
2. Click **Add New**
3. Set **Window class** to `ostt-popup`
4. Add rules:
   - **Position:** Force → your preferred x, y
   - **Size:** Force → your preferred width, height
   - **No titlebar:** Force → Yes
   - **Keep above:** Force → Yes
5. Click **Apply**

This gives you precise window placement that works even on Wayland.

## Troubleshooting

### Clipboard Not Working

Check your session type and install the right tool:

```bash
echo $XDG_SESSION_TYPE
# If "x11":
sudo apt install -y xclip
# If "wayland":
sudo apt install -y wl-clipboard
```

### Hotkey Not Working

Verify the command works from a terminal first:

```bash
ostt launch -c
```

Make sure the path in the Custom Shortcut Action tab is the full absolute path (not `~/` or relative). Use `which ostt` to find it.

### Popup Has a Titlebar

Konsole always shows a titlebar. Install kitty or alacritty for a borderless popup, or use KDE Window Rules to force "No titlebar" on the `ostt-popup` window class.
