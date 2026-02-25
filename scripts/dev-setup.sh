#!/usr/bin/env bash
# Pre-dev setup: build bat-agent so it's available alongside bat-shell during dev
set -e
echo "Building bat-agent..."
cargo build -p bat-agent
echo "bat-agent ready."
# Start the UI dev server (Tauri needs this to stay running)
cd crates/bat-shell/ui && npm run dev
