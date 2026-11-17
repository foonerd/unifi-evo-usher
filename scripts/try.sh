#!/bin/bash
# Run the usher's steward with its face on a directory of its own, on the
# loopback address, and say where to look. For looking at the face while
# working on it; this is not how a unit is installed.
#
#   scripts/try.sh [port [directory]]
#
# The port is 8443 unless another is named. What the steward keeps goes
# into the directory named, which is kept; with none named, into a
# directory made for the run and removed after it.
set -euo pipefail
ROOT=$(CDPATH='' cd -- "$(dirname "$0")/.." && pwd)
cd "$ROOT"

PORT=${1:-8443}
if [ $# -ge 2 ]; then
  DIR=$2
  mkdir -p "$DIR"
  DIR=$(CDPATH='' cd -- "$DIR" && pwd)
else
  # Under /tmp: the path of a socket may not be longer than 107 bytes.
  DIR=$(mktemp -d /tmp/usher.XXXXXX)
  trap 'rm -rf "$DIR"' EXIT
fi

cargo build --locked -p evo-usher-steward

mkdir -p "$DIR/state"
if [ ! -s "$DIR/pairing-code" ]; then
  od -An -N6 -tx1 /dev/urandom | tr -d ' \n' > "$DIR/pairing-code"
fi

cat > "$DIR/evo.toml" <<CONFIG
[steward]
log_level = "info"
socket_path = "$DIR/evo.sock"

[catalogue]
path = "$ROOT/catalogue/usher.toml"
lkg_path = "$DIR/state/catalogue.lkg.toml"

[persistence]
path = "$DIR/state/evo.db"

[plugins]
allow_unsigned = true
plugin_data_root = "$DIR/plugins-data"
runtime_dir = "$DIR/plugins-run"
config_dir = "$DIR/plugins.d"
search_roots = ["$ROOT/ui"]
bundled_roots = []
trust_dir_opt = "$DIR/trust-opt"
trust_dir_etc = "$DIR/trust-etc"
revocations_path = "$DIR/revocations.toml"

[plugins.stage]
dir = "$DIR/stage"

[plans]
storage_root = "$DIR/plans"

[multiroom]
discovery_enabled = false
CONFIG

echo "try: the face is at https://127.0.0.1:$PORT/"
echo "try: the steward's certificate is signed by its own authority, $DIR/state/https/ca.crt; a browser that does not have it warns"
echo "try: the pairing code, where the steward asks for one, is $(cat "$DIR/pairing-code")"
echo "try: Ctrl-C stops the steward"

EVO_HTTPS_LISTEN_ADDR="127.0.0.1:$PORT" \
  EVO_HTTPS_STATIC_DIR="$ROOT/ui" \
  EVO_PAIR_PRESEED_FILE="$DIR/pairing-code" \
  target/debug/evo-usher-steward --config "$DIR/evo.toml"
