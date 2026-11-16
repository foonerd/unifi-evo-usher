#!/bin/bash
# The toolchain rust-toolchain.toml names, on this machine: rustup itself
# where the machine has none (the pinned release, held against its digest),
# then the toolchain with its components and targets. What a machine has
# already is left as it is, so a build never depends on what an image
# happens to carry.
set -euo pipefail
ROOT=$(CDPATH='' cd -- "$(dirname "$0")/.." && pwd)
cd "$ROOT"

RUSTUP_VERSION=1.29.1

if ! command -v rustup > /dev/null; then
  case "$(uname -m)" in
    x86_64)
      host=x86_64-unknown-linux-gnu
      sha=dda7234360b7f578ca8b0ddcb80145646fa61a67c1720a5abc7051b35c9fcb71
      ;;
    aarch64)
      host=aarch64-unknown-linux-gnu
      sha=15f6e4ce9f583b929c996c91562bad6d4454f3281de858b02cdfdef615fac433
      ;;
    *)
      echo "toolchain: no rustup is pinned for a $(uname -m) machine" >&2
      exit 1
      ;;
  esac
  work=$(mktemp -d)
  trap 'rm -rf "$work"' EXIT
  scripts/fetch.sh "https://static.rust-lang.org/rustup/archive/$RUSTUP_VERSION/$host/rustup-init" "$work/rustup-init" "$sha"
  chmod +x "$work/rustup-init"
  "$work/rustup-init" -y --no-modify-path --profile minimal --default-toolchain none
  export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"
  # The steps after this one find it too.
  if [ -n "${GITHUB_PATH:-}" ]; then
    echo "${CARGO_HOME:-$HOME/.cargo}/bin" >> "$GITHUB_PATH"
  fi
fi

rustup toolchain install
rustup show active-toolchain
