#!/bin/bash
# The proof of the steward's RPM on a machine that is not an RPM system:
# scripts/prove-unit.sh, run in a container of AlmaLinux 9 that systemd
# runs, so that the package is installed by dnf and started by systemd
# as on a unit.
#
#   scripts/prove-unit-rpm.sh <evo-usher-steward-<version>-1.<machine>.rpm>
#
# It is for a machine of CI's, made for the run and thrown away after it:
# a container that systemd runs is given every privilege of the machine
# it is on. The image is named by its digest, which holds every
# architecture the usher is built for, so the proof is of the same system
# until the digest is changed here. A machine that is an RPM system needs
# none of this: there the proof is scripts/prove-unit.sh itself.
set -euo pipefail
ROOT=$(CDPATH='' cd -- "$(dirname "$0")/.." && pwd)

# AlmaLinux 9 with systemd as its first process (almalinux/9-init:9).
IMAGE=docker.io/almalinux/9-init@sha256:6de7473609dba81c4d4dd93e47deebb7fbb0fa2c5bc79f965b024b67ad8ec5b9

rpm=${1:?the package of the steward, an .rpm}
[ -f "$rpm" ] || { echo "prove-unit-rpm: no file $rpm" >&2; exit 1; }
rpm=$(readlink -f "$rpm")
command -v docker > /dev/null || { echo "prove-unit-rpm: docker is not on this machine" >&2; exit 1; }

work=$(mktemp -d)
said=$(mktemp)
name="usher-unit-$$"
cp "$rpm" "$ROOT/scripts/prove-unit.sh" "$work/"
ended() {
  docker rm --force "$name" > /dev/null 2>&1 || true
  rm -rf "$work" "$said"
}
trap ended EXIT

# systemd in a container manages the control groups of its services
# itself, and so is given them, with the privileges that takes.
docker run --detach --name "$name" --privileged --cgroupns=host \
  --volume /sys/fs/cgroup:/sys/fs/cgroup:rw \
  --volume "$work:/proof:ro" "$IMAGE" > /dev/null

# Up when systemd has started what the image starts; a service of the
# image's own that did not start is not the usher's to prove.
tries=0
until state=$(docker exec "$name" systemctl is-system-running 2> /dev/null || true); [ "$state" = running ] || [ "$state" = degraded ]; do
  tries=$((tries + 1))
  [ "$tries" -lt 120 ] || { echo "prove-unit-rpm: systemd did not come up in the container (it says '${state:-nothing}')" >&2; docker logs "$name" >&2 || true; exit 1; }
  sleep 1
done
echo "prove-unit-rpm: $(docker exec "$name" cat /etc/redhat-release), systemd $(docker exec "$name" systemctl --version | head -n 1 | cut -d' ' -f2), $state"

# What the proof asks with that the image does not have.
docker exec "$name" dnf install -y jq > "$said" 2>&1 || { cat "$said" >&2; echo "prove-unit-rpm: jq was not installed in the container" >&2; exit 1; }

docker exec "$name" bash /proof/prove-unit.sh "/proof/$(basename "$rpm")"
