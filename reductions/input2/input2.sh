#!/bin/bash

set -euo pipefail

cd "$(dirname "$0")"

BREW_PREFIX="$(brew --prefix)"
export PATH="$BREW_PREFIX/opt/llvm/bin:$PATH"

clang-23 -O3 -c -emit-llvm -o input2.ll input2.c -DNK_IMPLEMENTATION=1
llvm-reduce input2.ll --test="test2.sh"
