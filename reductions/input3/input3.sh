#!/bin/bash

set -euo pipefail

cd "$(dirname "$0")"

BREW_PREFIX="$(brew --prefix)"
export PATH="$BREW_PREFIX/opt/llvm/bin:$PATH"

clang-23 -O3 -c -emit-llvm -o input3.ll input3.c -DCGLTF_IMPLEMENTATION=1
llvm-reduce input3.ll --test="test3.sh"
