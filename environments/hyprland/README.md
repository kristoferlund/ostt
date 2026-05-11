# Omarchy / Hyprland Setup

## About Omarchy

[Omarchy](https://omarchy.org/) is an opinionated Arch Linux setup built on [Hyprland](https://hyprland.org/). With OSTT integrated into Omarchy, you get instant voice-to-text transcription accessible from any application through a global hotkey.

## Setup

Install OSTT and run initial authentication first:

```bash
curl -fsSL https://ostt.ai/install | bash
ostt auth
```

### Keybinding

Add the keybinding to `~/.config/hypr/bindings.conf`:

```hyprland
# ostt - Speech-to-Text hotkey (clipboard output)
bindd = SUPER, R, ostt, exec, ostt launch -c
```

### Window Rules

Add the window rules to `~/.config/hypr/hyprland.conf`:

```hyprland
# OSTT window overrides
# Float the window
windowrule = float on, match:title ostt
# Position centered horizontally at bottom (85% from top)
windowrule = move ((monitor_w*0.5)-(window_w*0.5)) (monitor_h*0.85), match:title ostt
```

Then reload your Hyprland configuration:

```bash
hyprctl reload
```

That's it!

## Usage

### Basic Usage (Clipboard Output)

1. **Press `Super+R`**: Opens ostt in a floating window and starts recording
2. **Speak your text**: Watch the real-time waveform visualization
3. **Press `Enter`**: Stops recording, transcribes, and copies to clipboard
4. **Press `Ctrl+V`**: Paste the transcribed text anywhere

Alternatively, you can press `Super+R` again instead of `Enter` to stop recording and transcribe.

### Output Options

By default, the Omarchy integration copies transcriptions to the clipboard. You can customize this by passing flags to `ostt launch` in `~/.config/hypr/bindings.conf`:

**Clipboard (default):**
```hyprland
bindd = SUPER, R, ostt, exec, ostt launch -c
```

**Stdout (for piping to other commands):**
```hyprland
bindd = SUPER, R, ostt, exec, ostt launch
```

**File output:**
```hyprland
bindd = SUPER, R, ostt, exec, ostt launch -o ~/transcription.txt
```

## Customization

### Window Position

Adjust the move rule in `~/.config/hypr/hyprland.conf` to change position. Use dynamic expressions with `monitor_w`, `monitor_h`, `window_w`, and `window_h` for responsive placement:

```hyprland
# Default: centered horizontally at bottom
windowrule = move ((monitor_w*0.5)-(window_w*0.5)) (monitor_h*0.85), match:title ostt

# Example: centered on screen
windowrule = move ((monitor_w*0.5)-(window_w*0.5)) ((monitor_h*0.5)-(window_h*0.5)), match:title ostt
```

### Terminal Appearance

On Omarchy, popup position is controlled by the Hyprland window rules in `~/.config/hypr/hyprland.conf`. Popup terminal selection, size, font size, and borderless behavior are configured in OSTT's `[popup]` settings. See the main README for shared popup configuration.

### Different Hotkey

Change `SUPER, R` to your preferred key combination in `~/.config/hypr/bindings.conf`:

```hyprland
# Example: Use Ctrl+Alt+R instead
bindd = CTRL_ALT, R, ostt, exec, ostt launch -c
```

## Troubleshooting

### Window Not Appearing

```bash
# Test launch directly
ostt launch -c

# Verify Hyprland config loaded
hyprctl reload
```

### Window Appears in Wrong Position

Make sure the window rules in `hyprland.conf` are placed before any catch-all rules that might override them.
