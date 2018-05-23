#!/bin/bash

# Cargo-fuzz needs nightly rust, so switch this project to nightly
rustup override set nightly

# Cleanout any old build artifacts
(cd fuzz && cargo clean)

# Name of the fuzz target
FUZZ_TARGET=fuzz_target_1

# Build the fuzz target in release mode by asking it to run it for 0 iterations
cargo fuzz run $FUZZ_TARGET --release -- -runs=0

# Find the executable for the fuzz target
EXE="$(find fuzz/target -name $FUZZ_TARGET -executable)"

# Upload the executable for the fuzz target
upload-binary "$EXE"
