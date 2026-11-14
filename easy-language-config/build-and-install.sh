#!/usr/bin/env sh
set -eu

# This script builds and installs the app to archiso airootfs

cd "$(dirname "$0")"

cargo build --release --locked
install -Dm755 target/release/easy-language-config ../airootfs/usr/local/bin/easy-language-config
