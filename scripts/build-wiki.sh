#!/usr/bin/env bash
# Builds the wiki in wiki/: item icons and biome views rendered by the game (Mesa lavapipe
# when there is no GPU), then the pages, strict (fails on any entry without text).
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --release -p strata-app -p wikigen
./target/release/strata --render-icons wiki-images/icons
./target/release/strata --render-biomes wiki-images/biomes
./target/release/wikigen --strict --out wiki --images wiki-images
echo "Wiki ready: wiki/index.html"
