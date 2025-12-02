# Hyprland / Omarchy - Floating Window Integration

## About Hyprland

[Hyprland](https://hyprland.org/) is a dynamic tiling Wayland compositor that provides smooth animations and extensive customization options. [Omarchy](https://omakub.org/omakub-omarchy) is a Hyprland-based desktop configuration that combines aesthetics with productivity.

With ostt integrated into Hyprland, you get instant voice-to-text transcription accessible from any application through a global hotkey.

## Setup

### One-Time Configuration

On first run, ostt automatically detects Hyprland and creates the integration script at `~/.local/bin/ostt-float`.

Add the following to your `~/.config/hypr/hyprland.conf`:

```hyprland
# ostt - Speech-to-Text hotkey
bindd = SUPER, R, ostt, exec, bash ~/.local/bin/ostt-float

# Window appearance (optional but recommended)
windowrule = float, title:ostt
windowrule = size 14% 8%, title:ostt
windowrule = move 43% 90%, title:ostt
```

Then reload your Hyprland configuration:

```bash
hyprctl reload
```

That's it!

## Usage

1. **Press `Super+R`**: Opens ostt in a floating window and starts recording
2. **Speak your text**: Watch the real-time waveform visualization
3. **Press `Enter`**: Stops recording, transcribes, and copies to clipboard
4. **Press `Ctrl+V`**: Paste the transcribed text anywhere

Alternatively, you can press `Super+R` again instead of `Enter` to stop recording and transcribe.

## Customization

### Window Position and Size

Adjust the window rules in your `hyprland.conf` to change size and position:

```hyprland
# Default: small window at bottom-center
windowrule = size 14% 8%, title:ostt
windowrule = move 43% 90%, title:ostt

# Example: larger centered window
windowrule = size 50% 30%, title:ostt
windowrule = move 25% 35%, title:ostt
```

### Terminal Appearance

The terminal appearance can be customized by editing `~/.config/ostt/alacritty-float.toml`.

### Different Hotkey

Change `SUPER, R` to your preferred key combination in `hyprland.conf`:

```hyprland
# Example: Use Ctrl+Alt+R instead
bindd = CTRL_ALT, R, ostt, exec, bash ~/.local/bin/ostt-float
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
