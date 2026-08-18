#!/bin/bash
# Full build (wasm + Astro site) → dist/, used by the deploy (see vercel.json).
# The wasm step lives in scripts/build-wasm.sh (also runnable standalone via
# `pnpm run build:wasm`); its trunk config is in wasm/Trunk.toml.
set -euo pipefail

WEB_DIR="$(cd "$(dirname "$0")/.." && pwd)"

echo "==> Building WASM..."
bash "$WEB_DIR/scripts/build-wasm.sh"

echo "==> Building Astro..."
(cd "$WEB_DIR" && pnpm run build:site)

echo "==> Done! Output in dist/"
