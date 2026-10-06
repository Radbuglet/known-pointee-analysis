#!/bin/bash

set -euo pipefail

opt "$1" -o "$1"
cargo run -- "$1" | grep -F '[!]'
