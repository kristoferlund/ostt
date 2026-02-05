# Hyprland / Omarchy - Floating Window Integration

## About Hyprland

[Hyprland](https://hyprland.org/) is a dynamic tiling Wayland compositor that provides smooth animations and extensive customization options. [Omarchy](https://omakub.org/omakub-omarchy) is a Hyprland-based desktop configuration that combines aesthetics with productivity.

With ostt integrated into Hyprland, you get instant voice-to-text transcription accessible from any application through a global hotkey.

## Setup

### One-Time Configuration

On first run, ostt automatically detects Hyprland and creates the integration script at `~/.local/bin/ostt-float`.

Add the following to your `~/.config/hypr/hyprland.conf`:

```hyprland
# ostt - Speech-to-Text hotkey (clipboard output)
bindd = SUPER, R, ostt, exec, bash ~/.local/bin/ostt-float -c

# OSTT window overrides
# Float the window
windowrule = float on, match:title ostt
# Resize with dynamic expressions (14% width, 8% height)
windowrule = size (monitor_w*0.14) (monitor_h*0.08), match:title ostt
# Position centered horizontally at bottom (90% from top)
windowrule = move ((monitor_w*0.5)-(window_w*0.5)) (monitor_h*0.9), match:title ostt
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

By default, the Hyprland integration copies transcriptions to the clipboard. You can customize this by passing flags to ostt:

**Clipboard (default):**
```hyprland
bindd = SUPER, R, ostt, exec, bash ~/.local/bin/ostt-float -c
```

**Stdout (for piping to other commands):**
```hyprland
bindd = SUPER, R, ostt, exec, bash ~/.local/bin/ostt-float
```

**File output:**
```hyprland
bindd = SUPER, R, ostt, exec, bash ~/.local/bin/ostt-float -o ~/transcription.txt
```

## Customization

### Window Position and Size

Adjust the window rules in your `hyprland.conf` to change size and position. Use dynamic expressions with `monitor_w`, `monitor_h`, `window_w`, and `window_h` for responsive sizing:

```hyprland
# Default: 14% width, 8% height, centered horizontally at bottom
windowrule = size (monitor_w*0.14) (monitor_h*0.08), match:title ostt
windowrule = move ((monitor_w*0.5)-(window_w*0.5)) (monitor_h*0.9), match:title ostt

# Example: larger centered window (50% width, 30% height, centered)
windowrule = size (monitor_w*0.5) (monitor_h*0.3), match:title ostt
windowrule = move ((monitor_w*0.5)-(window_w*0.5)) ((monitor_h*0.5)-(window_h*0.5)), match:title ostt
```

### Terminal Appearance

The terminal appearance can be customized by editing `~/.config/ostt/alacritty-float.toml`.

### Different Hotkey

Change `SUPER, R` to your preferred key combination in `hyprland.conf`:

```hyprland
# Example: Use Ctrl+Alt+R instead
bindd = CTRL_ALT, R, ostt, exec, bash ~/.local/bin/ostt-float
```

## Upgrading from 0.0.5

If you're upgrading from ostt 0.0.5, you only need to update your Hyprland window rules. Everything else is handled automatically!

> ⚠️ **BREAKING CHANGE:** Hyprland window rules syntax has changed. The old `windowrule` syntax is deprecated in recent Hyprland versions. You **must** update your window rules to the new syntax or the floating window will not appear correctly.

### Update Hyprland Window Rules (REQUIRED)

The window rule syntax has changed from the old format to a new format using `match:` patterns and dynamic expressions. Update your `hyprland.conf`:

**Old syntax (0.0.5):**
```hyprland
windowrule = float, title:ostt
windowrule = size 14% 8%, title:ostt
windowrule = move 43% 90%, title:ostt
```

**New syntax (0.0.7+):**
```hyprland
windowrule = float on, match:title ostt
windowrule = size (monitor_w*0.14) (monitor_h*0.08), match:title ostt
windowrule = move ((monitor_w*0.5)-(window_w*0.5)) (monitor_h*0.9), match:title ostt
```

Key changes:
- Use `match:title ostt` instead of `title:ostt`
- Add `on` parameter to the float rule: `float on`
- Use dynamic expressions for responsive sizing: `(monitor_w*0.14)` instead of `14%`
- Centering now uses expressions: `((monitor_w*0.5)-(window_w*0.5))` for horizontal centering

Then reload your Hyprland configuration:

```bash
hyprctl reload
```

## Troubleshooting

### Window Not Appearing

```bash
# Test the script directly
bash ~/.local/bin/ostt-float

# Verify Hyprland config loaded
hyprctl reload
```

### Window Appears in Wrong Position

Make sure the window rules in `hyprland.conf` are placed before any catch-all rules that might override them.
