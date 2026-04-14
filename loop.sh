#!/bin/bash
set -euo pipefail

# ---------------------------------------------------------------------------
# loop.sh — Run an AI coding prompt in a loop, pushing after each iteration.
#
# Usage:
#   ./loop.sh <prompt-file> [max-iterations]
#
# Examples:
#   ./loop.sh PROMPT.md        # Run until interrupted
#   ./loop.sh PROMPT.md 5      # Run at most 5 iterations
# ---------------------------------------------------------------------------

if [ $# -lt 1 ]; then
  echo "Usage: ./loop.sh <prompt-file> [max-iterations]"
  exit 1
fi

PROMPT_FILE="$1"
MAX_ITERATIONS="${2:-0}"
BRANCH=$(git branch --show-current)
ITERATION=0

if [ ! -f "$PROMPT_FILE" ]; then
  echo "Error: file not found: $PROMPT_FILE"
  exit 1
fi

# Header
echo ""
echo "  prompt   $PROMPT_FILE"
echo "  branch   $BRANCH"
if [ "$MAX_ITERATIONS" -gt 0 ] 2>/dev/null; then
  echo "  limit    $MAX_ITERATIONS"
fi
echo ""
echo "  ────────────────────────────────────"
echo ""

while true; do
  if [ "$MAX_ITERATIONS" -gt 0 ] 2>/dev/null && [ "$ITERATION" -ge "$MAX_ITERATIONS" ]; then
    echo "  done ($MAX_ITERATIONS iterations)"
    echo ""
    break
  fi

  ITERATION=$((ITERATION + 1))

  if [ "$MAX_ITERATIONS" -gt 0 ] 2>/dev/null; then
    echo "  [$ITERATION/$MAX_ITERATIONS]"
  else
    echo "  [$ITERATION]"
  fi
  echo ""

  cat "$PROMPT_FILE" | opencode run --model anthropic/claude-opus-4-6

  git push origin "$BRANCH" 2>/dev/null || git push -u origin "$BRANCH"

  echo ""
  echo "  ────────────────────────────────────"
  echo ""
done
