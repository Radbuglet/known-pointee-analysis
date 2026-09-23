#!/bin/bash

set -euo pipefail
cd "$(dirname "$0")"

BREW_PREFIX="$(brew --prefix)"
export PATH="$BREW_PREFIX/opt/llvm/bin:$PATH"

pushd res/
clang-23 -O3 -c -emit-llvm -o input.ll input.cpp
cargo run -- input.ll > output.txt
popd > /dev/null
