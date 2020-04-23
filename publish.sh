#!/bin/bash
set -euo pipefail

cargo publish
cargo build --release --target x86_64-unknown-linux-musl
mv target/x86_64-unknown-linux-musl/release/desed target/x86_64-unknown-linux-musl/release/desed-x86_64-unknown-linux-musl
strip target/x86_64-unknown-linux-musl/release/desed-x86_64-unknown-linux-musl
echo "Binary ready"
