#!/bin/bash

set -euo pipefail
cd "$(dirname "$0")"

BREW_PREFIX="$(brew --prefix)"
export PATH="$BREW_PREFIX/opt/llvm/bin:$PATH"

pushd res/
clang-23 -O3 -S -emit-llvm -o input.ll input.cpp
cargo run -- input.ll output.ll > output.txt
llc output.ll -o output.S
cc output.S -o output
./output
popd > /dev/null
