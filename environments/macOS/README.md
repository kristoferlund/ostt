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

-- Define hotkeys. Each entry maps a key combo to ostt arguments.
-- Hotkeys without a `window` field use DEFAULT_WINDOW settings below.
--
-- Toggle behavior: pressing the same hotkey while ostt is already recording
-- sends SIGUSR1 to the running ostt process, which finishes the recording
-- and triggers transcription (same as pressing Enter in the UI).
local HOTKEYS = {
	-- Option+Space: Record and copy raw transcription
	{
		mods = { "alt" },
		key = "space",
		ostt_args = "-c",
	},
	-- Option+Shift+Space: Record, clean up, copy
	{
		mods = { "alt", "shift" },
		key = "space",
		ostt_args = "-c -p clean",
	},
	-- Cmd+Shift+R: Record and copy (backwards compatibility)
	{
		mods = { "cmd", "shift" },
		key = "R",
		ostt_args = "-c",
	},
}

-- Default window settings (used when a hotkey doesn't specify its own)
local DEFAULT_WINDOW = {
	x = 630,
	y = 790,
	width = 50,
	height = 10,
	font_size = 8,
}

-- Track running ostt tasks per hotkey index
local runningTasks = {}

local function osttExists()
	local attr = hs.fs.attributes(OSTT_BIN)
	return attr ~= nil and attr.mode == "file"
end

--- Send SIGUSR1 to the ostt process running inside the given Ghostty task.
--- Ghostty spawns: login -> bash (exec'd into ostt), so we walk the
--- process tree to find the leaf process.
local function signalRunningOstt(task)
	local pid = task:pid()
	if not pid then return end

	-- Walk down the process tree to find the actual ostt process.
	-- Ghostty -> login -> ostt (via exec from bash)
	for _ = 1, 5 do
		local output, ok = hs.execute("pgrep -P " .. pid)
		if not ok or not output or output == "" then break end
		local child = output:match("(%d+)")
		if not child then break end
		pid = child
	end

	hs.execute("kill -USR1 " .. pid)
end

local function spawnOsttPopup(hotkeyIndex, ostt_args, window)
	if not osttExists() then
		hs.alert.show("OSTT not found or not executable:\n" .. OSTT_BIN)
		return
	end

	-- If this hotkey already has a running ostt, send SIGUSR1 to finish recording
	local existing = runningTasks[hotkeyIndex]
	if existing and existing:isRunning() then
		signalRunningOstt(existing)
		return
	end

	local w = window or DEFAULT_WINDOW

	-- Remember the currently focused app to restore later
	local frontApp = hs.application.frontmostApplication()

	-- Build shell command: source profile for full PATH, run ostt.
	-- The profile is needed so that bash actions inside ostt (e.g. invoking opencode)
	-- can find tools installed via Homebrew or other package managers.
	local cmd = "source ~/.bash_profile 2>/dev/null || source ~/.zprofile 2>/dev/null || source ~/.profile 2>/dev/null; clear; exec "
		.. OSTT_BIN
	if ostt_args and ostt_args ~= "" then
		cmd = cmd .. " " .. ostt_args
	end

	local args = {
		"--window-position-x=" .. (w.x or DEFAULT_WINDOW.x),
		"--window-position-y=" .. (w.y or DEFAULT_WINDOW.y),
		"--window-width=" .. (w.width or DEFAULT_WINDOW.width),
		"--window-height=" .. (w.height or DEFAULT_WINDOW.height),
		"--font-size=" .. (w.font_size or DEFAULT_WINDOW.font_size),
		"--background=#000000",
		"--window-decoration=none",
		"--macos-window-shadow=false",
		"-e",
		"/bin/bash",
		"-c",
		cmd,
	}

	-- Start Ghostty running OSTT with window position/size flags
	local task = hs.task.new(GHOSTTY_BIN, function(exitCode, stdOut, stdErr)
		runningTasks[hotkeyIndex] = nil
		-- When Ghostty/OSTT exits, go back to the previous app
		if frontApp then
			frontApp:activate()
		end
	end, args)

	task:start()
	runningTasks[hotkeyIndex] = task
end

-- Bind all configured hotkeys
for i, hk in ipairs(HOTKEYS) do
	hs.hotkey.bind(hk.mods, hk.key, function()
		spawnOsttPopup(i, hk.ostt_args, hk.window)
	end)
end
```

4. **Reload Hammerspoon** - Click the menu bar icon → "Reload Config"

That's it!

## Usage

### Basic Usage (Clipboard Output)

1. **Press `Option+Space`**: Opens ostt in a popup window and starts recording
2. **Speak your text**: Watch the real-time waveform visualization
3. **Press `Option+Space` again** (or `Enter` in the popup): Stops recording, transcribes, and copies to clipboard
4. **Press `Cmd+V`**: Paste the transcribed text anywhere

For cleaned-up text, use **`Option+Shift+Space`** instead — it records, transcribes, runs the "clean" processing action, and copies the result.

### Toggle Behavior

Each hotkey works as a toggle. Pressing it once starts recording, pressing it again finishes the recording and triggers transcription. This means you never need to focus the popup window to stop recording — just press the same hotkey from any application.

Under the hood, the second press sends a `SIGUSR1` signal to the running ostt process, which is equivalent to pressing Enter in the UI. The popup window closes automatically after transcription completes and the result is copied to the clipboard.

### Multiple Hotkeys for Different Workflows

Add entries to the `HOTKEYS` table for each workflow you want a dedicated hotkey for:

```lua
local HOTKEYS = {
	-- Cmd+Shift+R: Record and copy raw transcription
	{
		mods = { "cmd", "shift" },
		key = "R",
		ostt_args = "-c",
	},
	-- Cmd+Shift+E: Record, clean up text, copy
	{
		mods = { "cmd", "shift" },
		key = "E",
		ostt_args = "-p clean -c",
	},
	-- Cmd+Shift+T: Record, translate, copy
	{
		mods = { "cmd", "shift" },
		key = "T",
		ostt_args = "-p translate -c",
	},
	-- Cmd+Shift+Q: Record, answer question, copy
	{
		mods = { "cmd", "shift" },
		key = "Q",
		ostt_args = "-p question -c",
	},
}
```

Each hotkey runs ostt with its own arguments — no picker, no interaction beyond recording and pressing Enter.

### Output Options

Customize output per hotkey by changing the `ostt_args` value:

**Clipboard (default):**
```lua
ostt_args = "-c"
```

**File output:**
```lua
ostt_args = "-o ~/transcription.txt"
```

**Process and copy:**
```lua
ostt_args = "-p clean -c"
```

## Customization

### Window Position and Size

Adjust the `DEFAULT_WINDOW` table in `init.lua`:

```lua
local DEFAULT_WINDOW = {
	x = 630,         -- Horizontal position (pixels from left)
	y = 790,         -- Vertical position (pixels from top)
	width = 50,      -- Width in columns
	height = 10,     -- Height in rows
	font_size = 8,   -- Font size
}
```

**Note:** These values are static and don't adapt to different screen sizes. Adjust based on your display resolution.

### Per-Hotkey Window Settings

Each hotkey can optionally override the window position and size. This is useful if you want a larger window for the action picker:

```lua
{
	mods = { "cmd", "shift" },
	key = "P",
	ostt_args = "-p -c",  -- show action picker
	window = {
		x = 430,
		y = 400,
		width = 60,
		height = 20,
		font_size = 12,
	},
},
```

Hotkeys without a `window` field use the `DEFAULT_WINDOW` settings.

## Upgrading from Previous Versions

Replace your existing ostt Hammerspoon configuration entirely with the updated version from the "One-Time Configuration" section above, then reload Hammerspoon.

The new configuration uses a `HOTKEYS` table instead of a single `OSTT_ARGS` variable, supporting multiple hotkeys with different ostt parameters.

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
