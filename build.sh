#!/usr/bin/env bash

set -euo pipefail

# install rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > rustup
sh rustup -y
export PATH=$PATH:$HOME/.cargo/bin

# build
cargo build --release
cp -v target/release/macondo .
