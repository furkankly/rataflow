#!/usr/bin/env bash
# Build the rataflow website wasm bin into web/public/wasm/, so the Astro
# site can load it (see src/pages/index.astro). Safe to run from anywhere — it
# resolves its own paths.
set -euo pipefail

cd "$(dirname "$0")/../wasm"  # -> web/wasm/ (where Trunk.toml + index.html live)

# Config (target / dist / filehash / release) lives in wasm/Trunk.toml.
trunk build

# Trunk also emits its own index.html next to the artifacts. The real page is
# Astro's, so drop the stray one (it would otherwise ship at /wasm/).
rm -f ../public/wasm/index.html

echo "wasm build -> web/public/wasm/ (rataflow-web.js, _bg.wasm)"
