#!/bin/bash

set -euo pipefail

cd "$(dirname "$0")"

BREW_PREFIX="$(brew --prefix)"
export PATH="$BREW_PREFIX/opt/llvm/bin:$PATH"

clang-23 -O3 -c -emit-llvm -o input1.ll input1.c -DSTB_IMAGE_IMPLEMENTATION=1
cargo run -- input1.ll > output1.txt
