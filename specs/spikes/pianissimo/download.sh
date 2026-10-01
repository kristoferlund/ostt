#!/usr/bin/env bash
set -euo pipefail
destination=${1:?usage: bash download.sh MODEL_DIRECTORY}
revision=63730c6021234f26b9bbae9a07a04fec39e7a52e
base="https://huggingface.co/KlangAI/pianissimo-sv-onnx/resolve/$revision"
mkdir -p "$destination"
curl --fail --location --silent --show-error "$base/manifest.json" -o "$destination/manifest.json"
for file in encoder-model.int8.onnx decoder_joint-model.int8.onnx vocab.txt config.json nemo128.onnx; do
    hash=$(jq -er --arg file "$file" '.files[$file].sha256' "$destination/manifest.json")
    if [[ ! -f "$destination/$file" ]] || ! (cd "$destination" && printf '%s  %s\n' "$hash" "$file" | sha256sum --check --status); then
        curl --fail --location --silent --show-error --retry 2 "$base/$file" -o "$destination/$file.part"
        (cd "$destination" && printf '%s  %s.part\n' "$hash" "$file" | sha256sum --check)
        mv "$destination/$file.part" "$destination/$file"
    fi
done
