#!/bin/bash

# Build crewchief-maproom for current platform
cargo build --release --manifest-path crates/maproom/Cargo.toml

# Optionally cross-compile for other platforms
# e.g., cargo build --release --target x86_64-apple-darwin
# cargo build --release --target x86_64-unknown-linux-gnu
