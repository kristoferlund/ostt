#!/bin/bash
#
# ostt Hyprland Integration Script
#
# Usage: ostt-float.sh [ostt arguments]
#
# - First execution: opens floating ostt window and starts recording
# - Second execution: sends SIGUSR1 to trigger transcription, closes window
# - Arguments are passed directly to ostt (e.g., -c for clipboard, -o file for output)

# --- Configuration -----------------------------------------------------------
OSTT_BIN="${OSTT_BIN:-ostt}"
# If OSTT_BIN is just a command name (not a path), use 'which' to find it
if [[ "$OSTT_BIN" != */* ]]; then
    OSTT_BIN="$(which "$OSTT_BIN" 2>/dev/null || echo "$OSTT_BIN")"
fi
# Convert to absolute path if it exists
if [ -e "$OSTT_BIN" ]; then
    OSTT_BIN="$(cd "$(dirname "$OSTT_BIN")" && pwd)/$(basename "$OSTT_BIN")"
fi

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ALACRITTY_CONFIG="${HOME}/.config/ostt/alacritty-float.toml"
STATE_FILE="/tmp/ostt_active.pid"

# --- Helpers -----------------------------------------------------------------
register_pid() {
    # Find the newly spawned ostt process (match binary path first)
    local NEW_PID
    NEW_PID=$(pgrep -f "^$OSTT_BIN" | tail -1)
    if [ -z "$NEW_PID" ]; then
        NEW_PID=$(ps aux | grep -v grep | grep "$(basename "$OSTT_BIN")" | awk '{print $2}' | tail -1)
    fi
    [ -n "$NEW_PID" ] && echo "$NEW_PID" > "$STATE_FILE"
}

# --- Sanity check -----------------------------------------------------------
if [ ! -x "$OSTT_BIN" ]; then
    hyprctl notify -1 2 5000 "rgb(ff0000)" "ostt: Binary not found at $OSTT_BIN"
    exit 1
fi

# --- Main logic -------------------------------------------------------------
if [ -f "$STATE_FILE" ]; then
    PID=$(cat "$STATE_FILE" 2>/dev/null)

    if [ -n "$PID" ] && kill -0 "$PID" 2>/dev/null; then
        # ostt is running → send SIGUSR1 to trigger transcription and wait for it to exit
        kill -USR1 "$PID" 2>/dev/null || true
        wait "$PID" 2>/dev/null || true

        rm -f "$STATE_FILE"
        exit 0
    else
        # stale state file; clean up and fall through to start fresh
        rm -f "$STATE_FILE"
    fi
fi

# ostt not running → spawn new window with Alacritty config
# Pass all script arguments to ostt
hyprctl dispatch exec \
  "[float] alacritty --config-file \"$ALACRITTY_CONFIG\" --title ostt -e \"$OSTT_BIN\" $*"

# small delay so the process exists and we can grab PID
sleep 0.5
register_pid
