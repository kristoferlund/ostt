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
