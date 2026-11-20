#!/bin/bash
# The notices of the libraries the usher's programs contain, each with
# the licence it comes under: packaging/licences/THIRD-PARTY-NOTICES.txt,
# which every archive, package and image carries. The licences of those
# libraries ask that their notices go wherever the programs go.
#
# The file is made by cargo-about from what Cargo.lock names for the
# programs as they are built (about.toml says for which targets, and
# which licences are taken), out of the libraries' own files, and is
# kept in the repository, so that packing needs nothing but the tree.
#
#   scripts/notices.sh            write the file anew
#   scripts/notices.sh --check    say whether the file is what would be
#                                 written; scripts/check.sh asks this
#
# Run it whenever the libraries change, which is whenever Cargo.lock
# does. It needs cargo-about, in the version named here:
#   cargo install cargo-about --locked --version 0.9.3 --features cli
set -euo pipefail
ROOT=$(CDPATH='' cd -- "$(dirname "$0")/.." && pwd)
cd "$ROOT"

ABOUT=0.9.3
NOTICES=packaging/licences/THIRD-PARTY-NOTICES.txt

have=$(cargo about --version 2> /dev/null | awk '{ print $2 }' || true)
[ "$have" = "$ABOUT" ] || { echo "notices: cargo-about $ABOUT is not on this machine (found: ${have:-none}); it comes with: cargo install cargo-about --locked --version $ABOUT --features cli" >&2; exit 1; }

made=$(mktemp)
trap 'rm -f "$made"' EXIT
# Every library of every target is fetched first; the notices are then
# made from those files alone, so that they are the same wherever they
# are made.
cargo fetch --locked > /dev/null
cargo about generate --locked --offline --output-file "$made" packaging/licences/notices.hbs

if [ "${1:-}" = --check ]; then
  cmp -s "$made" "$NOTICES" || { echo "notices: $NOTICES is not what the libraries now ask for; run scripts/notices.sh and look at what changed" >&2; exit 1; }
  echo "notices: $NOTICES names what the programs contain"
else
  install -m 644 "$made" "$NOTICES"
  echo "notices: $NOTICES written, $(grep -c -E '^  [A-Za-z0-9_-]+ [0-9]' "$NOTICES") libraries under $(grep -c -E '^-{72}$' "$NOTICES") licence texts"
fi
