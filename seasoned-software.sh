#!/bin/bash

export CFLAGS=
export CXXFLAGS=

# Cleanout any old build artifacts
(cd fuzz && cargo clean)

for FUZZ_TARGET in defaults truncate; do
    # Build the fuzz target in release mode by asking it to run it for 0 iterations
    cargo +nightly fuzz run $FUZZ_TARGET --release -- -runs=0

    # Find the executable for the fuzz target
    EXE="$(find fuzz/target -name $FUZZ_TARGET -executable)"

    # Upload the executable for the fuzz target
    upload-binary "$EXE"
done
