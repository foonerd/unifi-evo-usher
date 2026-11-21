#!/bin/bash
# Run the usher's steward with its face on a directory of its own, on the
# loopback address, and say where to look. For looking at the face while
# working on it; this is not how a unit is installed.
#
#   scripts/try.sh [port [directory]]
#
# The port is 8443 unless another is named. What the steward keeps goes
# into the directory named, which is kept, its people among it; with
# none named, into a directory made for the run and removed after it.
#
# Whoever opens the face first sets the usher up, with a name and a
# password. With USHER_SECURITY=extended in the environment it runs at
# the extended level, where setting it up takes the unit's code as well.
# USHER_SITE_NAME, USHER_TIME_ZONE and USHER_LANGUAGE in the environment
# are given to the unit as its first name, time zone and language.
#
# With USHER_DEMO=1 in the environment it runs as a demo: on a simulated
# controller, with voucher types to hand out at once, and with vouchers
# that open no network. A directory that was a demo's stays a demo's,
# and one that was a unit's is not started as a demo.
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
LEVEL=${USHER_SECURITY:-standard}
if [ "$LEVEL" = extended ] && [ ! -s "$DIR/unit-code" ]; then
  (umask 077 && od -An -N6 -tx1 /dev/urandom | tr -d ' \n' > "$DIR/unit-code")
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
if [ -s "$DIR/state/usher/people.json" ]; then
  echo "try: this usher is set up: sign in with the name and the password it was set up with"
  echo "try: to set it up anew, remove $DIR/state/usher/people.json"
else
  echo "try: nobody has set this usher up yet: whoever opens the face first does, with a name and a password"
fi
if [ "$LEVEL" = extended ]; then
  echo "try: it runs at the extended level; the unit's code, which setting it up takes, is $(cat "$DIR/unit-code"), kept in $DIR/unit-code"
fi
case "${USHER_DEMO:-}" in
  1 | yes | on | true) echo "try: it runs as a demo: its controller is simulated, and the vouchers it hands out open no network" ;;
esac
echo "try: Ctrl-C stops the steward"

USHER_SECURITY="$LEVEL" \
  EVO_HTTPS_LISTEN_ADDR="127.0.0.1:$PORT" \
  EVO_HTTPS_STATIC_DIR="$ROOT/ui" \
  EVO_PAIR_PRESEED_FILE="$DIR/unit-code" \
  EVO_HTTPS_STATE_DIR="$DIR/state" \
  target/debug/evo-usher-steward --config "$DIR/evo.toml"
