#!/usr/bin/env bash
set -euo pipefail

# Install cargo tooling
cargo install cargo-nextest --locked
cargo install rust-script --locked
cargo install just --locked

# Pre-fetch dependencies so first build is fast
cargo fetch --all-targets

echo "langchainx devcontainer ready."
echo "Run 'cargo build --all-features' to verify the full build."
