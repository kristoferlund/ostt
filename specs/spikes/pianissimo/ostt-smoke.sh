#!/usr/bin/env bash
# Native CLI integration checks using the public FLEURS fixture, never user config.
set -euo pipefail
ostt=${1:?usage: bash ostt-smoke.sh OSTT_BINARY INT8_CACHE FLEURS_WAV}
cache=${2:?INT8 cache directory required}
fixture=${3:?public FLEURS fixture required}
printf '%s  %s\n' 82e5cefc3793e1352ae1f4cebd3848ab57e96f91799192b7d1b7d027151c5ff2 "$fixture" | sha256sum --check
root=$(mktemp -d /tmp/opencode/ostt-parakeet-smoke.XXXXXX)
export XDG_CONFIG_HOME="$root/config" XDG_DATA_HOME="$root/data" XDG_STATE_HOME="$root/state"
model="$XDG_DATA_HOME/ostt/models/pianissimo-sv-int8.onnx-bundle"
mkdir -p "$model"
trap '"$ostt" daemon stop >/dev/null 2>&1' EXIT
for file in encoder-model.int8.onnx decoder_joint-model.int8.onnx vocab.txt config.json nemo128.onnx; do
    cp --reflink=auto "$cache/$file" "$model/$file"
done
"$ostt" model local download parakeet/pianissimo-sv-int8
"$ostt" model select parakeet/pianissimo-sv-int8
expected='Hongkongön ger Hongkongs territorium dess namn och är den plats som många turister betraktar som huvudfokus.'
"$ostt" transcribe "$fixture" > "$root/direct.txt"
[[ $(cat "$root/direct.txt") == "$expected" ]]
if "$ostt" transcribe "$fixture" --param language=sv > "$root/unsupported.txt" 2>&1; then
    echo 'FAIL: accepted a Whisper-only param' >&2
    exit 1
fi
"$ostt" daemon start
"$ostt" daemon status > "$root/status.txt"
grep -q 'Backend: parakeet/cpu' "$root/status.txt"
"$ostt" transcribe "$fixture" > "$root/daemon.txt"
cmp "$root/direct.txt" "$root/daemon.txt"
ffmpeg -v error -stream_loop 7 -i "$fixture" -ar 16000 -ac 1 -c:a pcm_s16le "$root/long.wav"
"$ostt" transcribe "$root/long.wav" > "$root/long.txt"
long_expected="$expected"
for _ in {1..7}; do long_expected+=" $expected"; done
[[ $(cat "$root/long.txt") == "$long_expected" ]]
"$ostt" daemon stop
unshare --user --map-root-user --net "$ostt" transcribe "$fixture" > "$root/offline.txt"
cmp "$root/direct.txt" "$root/offline.txt"
"$ostt" model local remove parakeet/pianissimo-sv-int8
[[ ! -e "$model" ]]
[[ $("$ostt" model current) == 'No model selected.' ]]
echo "PASS: download, selection, direct/daemon inference, params, boundaries, offline, removal ($root)"
