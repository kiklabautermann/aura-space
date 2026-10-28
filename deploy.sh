#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGET_DIR="/home/mill/.clap/mill"
mkdir -p "$TARGET_DIR"

cd "$SCRIPT_DIR"

echo "Building AuraVerb (Rust Vintage Verb)..."
cargo build --release

echo "Deploying to $TARGET_DIR/aura_verb.clap..."
cp target/release/libaura_verb.so "$TARGET_DIR/aura_verb.clap"

echo "Bundling with xtask..."
cargo xtask bundle aura-verb --release || true
if [ -d target/bundled/aura-verb.clap ]; then
    cp -r target/bundled/aura-verb.clap "$TARGET_DIR/" || true
fi

echo "Done! AuraVerb successfully deployed to $TARGET_DIR/"
