# macOS - Hammerspoon Popup Integration

## About Hammerspoon

[Hammerspoon](https://www.hammerspoon.org/) is a powerful, well-established open-source automation tool for macOS. It allows you to control your Mac using Lua scripts, enabling deep system integration and custom workflows. Hammerspoon has been actively maintained for years and is trusted by thousands of macOS power users.

With ostt integrated via Hammerspoon, you get instant voice-to-text transcription accessible from any application through a global hotkey.

**Alternative:** If you use [iTerm2](https://iterm2.com/), you can achieve similar functionality using its built-in [Hotkey Window](https://iterm2.com/features.html) feature without needing Hammerspoon.

## Setup

### Prerequisites

1. **Install Hammerspoon:**
   - Download from [hammerspoon.org](https://www.hammerspoon.org/)
   - Or install via Homebrew: `brew install --cask hammerspoon`

2. **Install ostt:**
   ```bash
   brew install kristoferlund/ostt/ostt
   ```

3. **Install Ghostty terminal:**
   - Download from [ghostty.org](https://ghostty.org/)
   - Follow the installation instructions on their website

### One-Time Configuration

1. **Launch Hammerspoon** - It will appear in your menu bar
2. **Open Hammerspoon config** - Click the Hammerspoon menu bar icon → "Open Config"
3. **Add the following to your `init.lua`:**

```lua
-- === ostt Configuration ===
local OSTT_BIN = "/opt/homebrew/bin/ostt"
local GHOSTTY_BIN = "/Applications/Ghostty.app/Contents/MacOS/ghostty"
local OSTT_ARGS = "-c"  -- Copy to clipboard by default. Use "" for stdout, or "-o file" for file output

local function osttExists()
	local attr = hs.fs.attributes(OSTT_BIN)
	return attr ~= nil and attr.mode == "file"
end

local function spawnOsttPopup()
	if not osttExists() then
		hs.alert.show("OSTT not found or not executable:\n" .. OSTT_BIN)
		return
	end

	-- Remember the currently focused app to restore later
	local frontApp = hs.application.frontmostApplication()

	-- Build the command with args
	local args = {
		"--window-position-x=630",
		"--window-position-y=790",
		"--window-width=50",
		"--window-height=10",
		"--font-size=8",
		"--background=#000000",
		"--window-decoration=none",
		"--macos-window-shadow=false",
		"-e",
		OSTT_BIN,
	}
	
	-- Add ostt arguments if specified
	if OSTT_ARGS ~= "" then
		for arg in string.gmatch(OSTT_ARGS, "%S+") do
			table.insert(args, arg)
		end
	end

	-- Start Ghostty running OSTT with window position/size flags
	local task = hs.task.new(GHOSTTY_BIN, function(exitCode, stdOut, stdErr)
		-- When Ghostty/OSTT exits, go back to the previous app
		if frontApp then
			frontApp:activate()
		end
	end, args)

	task:start()
end

-- Hotkey: Cmd+Shift+R
hs.hotkey.bind({ "cmd", "shift" }, "R", spawnOsttPopup)
```

4. **Reload Hammerspoon** - Click the menu bar icon → "Reload Config"

That's it!

## Usage

### Basic Usage (Clipboard Output)

1. **Press `Cmd+Shift+R`**: Opens ostt in a popup window and starts recording
2. **Speak your text**: Watch the real-time waveform visualization
3. **Press `Enter`**: Stops recording, transcribes, and copies to clipboard
4. **Press `Cmd+V`**: Paste the transcribed text anywhere

### Output Options

By default, the Hammerspoon integration copies transcriptions to the clipboard. You can customize this by changing the `OSTT_ARGS` variable in your `init.lua`:

**Clipboard (default):**
```lua
local OSTT_ARGS = "-c"
```

**Stdout (for piping to other commands):**
```lua
local OSTT_ARGS = ""
```

**File output:**
```lua
local OSTT_ARGS = "-o ~/transcription.txt"
```

## Customization

### Window Position and Size

Adjust the window parameters in `init.lua`:

```lua
{
	"--window-position-x=630",  -- Horizontal position (pixels from left)
	"--window-position-y=790",  -- Vertical position (pixels from top)
	"--window-width=50",        -- Width in columns
	"--window-height=10",       -- Height in rows
	"--font-size=8",            -- Font size
	-- ...
}
```

**Note:** These values are static and don't adapt to different screen sizes. Adjust based on your display resolution.

### Different Hotkey

Change the hotkey binding in `init.lua`:

```lua
-- Example: Use Ctrl+Alt+R instead
hs.hotkey.bind({ "ctrl", "alt" }, "R", spawnOsttPopup)
```

## Upgrading from 0.0.5

If you're upgrading from ostt 0.0.5, you need to update your Hammerspoon configuration to support the new output flags.

### Update Your init.lua

Edit your `~/.hammerspoon/init.lua` (or wherever you placed the ostt configuration) and add the `OSTT_ARGS` variable:

**1. Add the OSTT_ARGS variable after the other config variables:**

```lua
local OSTT_BIN = "/opt/homebrew/bin/ostt"
local GHOSTTY_BIN = "/Applications/Ghostty.app/Contents/MacOS/ghostty"
local OSTT_ARGS = "-c"  -- Add this line for clipboard output
```

**2. Update the `spawnOsttPopup` function to build args dynamically:**

Replace the existing function with the updated version from this README (see the "One-Time Configuration" section above), which includes:
- Building an `args` table
- Parsing and adding `OSTT_ARGS` to the command

**3. Reload Hammerspoon:**

Click the Hammerspoon menu bar icon → "Reload Config"

**Alternative:** If you prefer, you can copy the entire updated configuration from the [init.lua template](init.lua) in this repository.

**Note:** Without the `-c` flag in `OSTT_ARGS`, transcriptions will output to stdout instead of clipboard (which won't be visible in the popup window).

## Troubleshooting

### Popup Not Appearing

```bash
# Check if ostt is installed at the expected path
ls -l /opt/homebrew/bin/ostt

# Check Hammerspoon console for errors
# Click Hammerspoon menu bar icon → Console
```

### Wrong ostt Path

If ostt is installed elsewhere (e.g., via shell installer):

```bash
# Find ostt location
which ostt

# Update OSTT_BIN in init.lua with the correct path
```

### Popup Not Working with Full-Screen Apps

Due to macOS window manager limitations, full-screen apps run in their own Space, preventing other windows from appearing on top. Consider using ostt in a regular terminal window or switching out of full-screen mode when needed.
