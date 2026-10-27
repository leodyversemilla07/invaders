#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

if ! rustup target list --installed | grep -qx "wasm32-unknown-unknown"; then
  echo "Missing WebAssembly target. Install it with:" >&2
  echo "  rustup target add wasm32-unknown-unknown" >&2
  exit 1
fi

cargo build --release --locked --target wasm32-unknown-unknown

rm -rf web/assets
mkdir -p web/assets
cp -R assets/. web/assets/
cp target/wasm32-unknown-unknown/release/invaders.wasm web/invaders.wasm

cat <<'EOF'
Web build complete.

Serve it with:
  python3 -m http.server 8080 --directory web

Then open:
  http://localhost:8080
EOF
