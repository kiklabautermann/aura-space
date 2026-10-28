#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGET_DIR="/home/mill/.clap/mill"
mkdir -p "$TARGET_DIR"

cd "$SCRIPT_DIR"

echo "Building AuraSpace (Vintage Reverb)..."
cargo build --release

echo "Deploying to $TARGET_DIR/aura_space.clap..."
cp target/release/libaura_space.so "$TARGET_DIR/aura_space.clap"
cp target/release/libaura_space.so "$TARGET_DIR/aura_verb.clap" 2>/dev/null || true

echo "Bundling with xtask..."
cargo xtask bundle aura-space --release || true
if [ -d target/bundled/aura-space.clap ]; then
    cp -r target/bundled/aura-space.clap "$TARGET_DIR/" || true
fi

echo "Done! AuraSpace successfully deployed to $TARGET_DIR/"
