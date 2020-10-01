#!/bin/bash
set -euo pipefail

cargo publish || true
cargo build --release --target x86_64-unknown-linux-gnu
mv target/x86_64-unknown-linux-gnu/release/desed target/x86_64-unknown-linux-gnu/release/desed-x86_64-unknown-linux-gnu
strip target/x86_64-unknown-linux-gnu/release/desed-x86_64-unknown-linux-gnu
echo "Binary ready"
