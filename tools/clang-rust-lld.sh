#!/bin/sh

set -eu

sysroot="$(rustc --print sysroot)"
linker="$sysroot/lib/rustlib/aarch64-apple-darwin/bin/gcc-ld/ld64.lld"

exec xcrun clang "-fuse-ld=$linker" "$@"
