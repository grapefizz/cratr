#!/bin/bash
set -e

echo "Building WASM frontend..."

# Install wasm-pack if not already installed
if ! command -v wasm-pack &> /dev/null; then
    echo "Installing wasm-pack..."
    curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
fi

# Build the WASM package with frontend feature and no default features
wasm-pack build --target web --out-dir pkg --out-name cratr --no-default-features --features frontend

# Copy WASM files to static directory
mkdir -p static
cp pkg/cratr.js static/
cp pkg/cratr_bg.wasm static/

echo "WASM build complete!"
echo "Files copied to static directory:"
echo "  - static/cratr.js"
echo "  - static/cratr_bg.wasm"
