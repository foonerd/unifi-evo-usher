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
# USHER_SITE_NAME, USHER_TIME_ZONE and USHER_LANGUAGE in the environment
# are given to the unit as its first name, time zone and language.
#
# With USHER_DEMO=1 in the environment it runs as a demo: on a simulated
# controller, with voucher types to hand out at once, and with vouchers
# that open no network. A directory that was a demo's stays a demo's,
# and one that was a unit's is not started as a demo.
#
# With USHER_PROFILE in the environment naming a profile's file, as the
# page Site of an usher exports one, the unit is given the file and
# applies it when it starts: once for each file.
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
case "${USHER_DEMO:-}" in
  1 | yes | on | true) echo "try-image: it runs as a demo: its controller is simulated, and the vouchers it hands out open no network" ;;
esac
# A profile is given to the unit as a file it can read, under the name
# the unit is told.
profile=()
if [ -n "${USHER_PROFILE:-}" ]; then
  [ -r "$USHER_PROFILE" ] || { echo "try-image: there is no file $USHER_PROFILE to give the unit as its profile" >&2; exit 1; }
  profile=(--volume "$(readlink -f "$USHER_PROFILE"):/etc/evo/profile.json:ro" --env USHER_PROFILE=/etc/evo/profile.json)
  echo "try-image: it is started with the profile $USHER_PROFILE, which it applies once"
fi
echo "try-image: Ctrl-C stops the container"

docker run --rm --name "usher-try-$PORT" \
  --user "$me" \
  --publish "127.0.0.1:$PORT:8443" \
  --env "USHER_SECURITY=$LEVEL" \
  --env USHER_SITE_NAME --env USHER_TIME_ZONE --env USHER_LANGUAGE \
  --env USHER_DEMO \
  --volume "$DIR/kept:/var/lib/evo" \
  "${extended[@]}" \
  "${profile[@]}" \
  --volume "$DIR/passwd:/etc/passwd:ro" \
  --tmpfs "/run/evo:mode=0755,uid=$(id -u),gid=$(id -g)" \
  "$image"
