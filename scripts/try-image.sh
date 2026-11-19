#!/bin/bash
# Run the usher as its image runs it, on this machine, and say where to
# look: the unit as it is installed, in a container, and not the working
# tree, which scripts/try.sh runs. The image is the one scripts/image.sh
# --here keeps.
#
#   scripts/try-image.sh [port [directory]]
#
# The port is 8444 unless another is named, on the loopback address. What
# the steward keeps goes into the directory named, which is kept; with
# none named, into a directory made for the run and removed after it. The
# container runs as whoever runs this, so what it keeps is theirs.
#
# Whoever opens the face first sets the usher up, with a name and a
# password. With USHER_SECURITY=extended in the environment it runs at
# the extended level, where setting it up takes the unit's code as well.
set -euo pipefail
ROOT=$(CDPATH='' cd -- "$(dirname "$0")/.." && pwd)
cd "$ROOT"
. scripts/targets.sh

PORT=${1:-8444}
if [ $# -ge 2 ]; then
  DIR=$2
  mkdir -p "$DIR"
  DIR=$(CDPATH='' cd -- "$DIR" && pwd)
else
  DIR=$(mktemp -d /tmp/usher.XXXXXX)
  trap 'rm -rf "$DIR"' EXIT
fi

image="unifi-evo-usher:$(workspace_version)"
docker image inspect "$image" > /dev/null 2>&1 || { echo "try-image: $image is not on this machine; build it first (scripts/build.sh $(this_arch), then scripts/image.sh --here)" >&2; exit 1; }

# A unit in a container makes no code of its own: at the extended level
# it is given one, as a file.
mkdir -p "$DIR/kept"
LEVEL=${USHER_SECURITY:-standard}
extended=()
if [ "$LEVEL" = extended ]; then
  if [ ! -s "$DIR/unit-code" ]; then
    (umask 077 && od -An -N6 -tx1 /dev/urandom | tr -d ' \n' > "$DIR/unit-code")
  fi
  extended=(--volume "$DIR/unit-code:/etc/evo/unit-code:ro")
fi
# The steward asks the name of the account it runs as, and the image
# knows only its own: here it runs as whoever runs this.
me="$(id -u):$(id -g)"
echo "evo:x:$me:evo steward:/var/lib/evo:/sbin/nologin" > "$DIR/passwd"

echo "try-image: $(docker run --rm "$image" --version)"
echo "try-image: the face is at https://127.0.0.1:$PORT/"
echo "try-image: the steward's certificate is signed by its own authority, $DIR/kept/state/https/ca.crt; a browser that does not have it warns"
if [ -s "$DIR/kept/state/usher/people.json" ]; then
  echo "try-image: this usher is set up: sign in with the name and the password it was set up with"
  echo "try-image: to set it up anew, remove $DIR/kept/state/usher/people.json"
else
  echo "try-image: nobody has set this usher up yet: whoever opens the face first does, with a name and a password"
fi
if [ "$LEVEL" = extended ]; then
  echo "try-image: it runs at the extended level; the unit's code, which setting it up takes, is $(cat "$DIR/unit-code"), kept in $DIR/unit-code"
fi
echo "try-image: Ctrl-C stops the container"

docker run --rm --name "usher-try-$PORT" \
  --user "$me" \
  --publish "127.0.0.1:$PORT:8443" \
  --env "USHER_SECURITY=$LEVEL" \
  --volume "$DIR/kept:/var/lib/evo" \
  "${extended[@]}" \
  --volume "$DIR/passwd:/etc/passwd:ro" \
  --tmpfs "/run/evo:mode=0755,uid=$(id -u),gid=$(id -g)" \
  "$image"
