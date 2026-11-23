#!/bin/bash
# The steward as an image for both architectures, from what
# scripts/build.sh made under out/ (packaging/docker/Dockerfile: the unit
# as the steward's package installs it, on an empty image).
#
#   image.sh --check   builds the image for every architecture, then this
#                      machine's own once more, loaded and run: the image
#                      must name the program as the binary does
#   image.sh --here    builds the image for this machine's architecture
#                      alone and keeps it, as unifi-evo-usher:<version>,
#                      for scripts/try-image.sh
#   image.sh --push    builds it for every architecture and pushes it as
#                      $IMAGE:<version> (default ghcr.io/foonerd/unifi-evo-usher);
#                      the login is the caller's
#
# No tag but the version is pushed: which version is the latest is said by
# hand, once a release has been tried, and .github/workflows/latest.yml
# then lays the tag latest on that version's image.
set -euo pipefail
ROOT=$(CDPATH='' cd -- "$(dirname "$0")/.." && pwd)
cd "$ROOT"
. scripts/targets.sh
. scripts/unit.sh
# Whoever packs, what is packed is the same: no mode is the packer's own.
umask 022

mode=${1:?--check, --here or --push}
IMAGE=${IMAGE:-ghcr.io/foonerd/unifi-evo-usher}
version=$(workspace_version)

case "$mode" in
  --check | --push) arches="$ARCHES" ;;
  --here)
    arches=$(this_arch)
    [ -n "$arches" ] || { echo "image: a $(uname -m) machine runs neither architecture" >&2; exit 1; }
    ;;
  *) echo "image: '$mode' is none of --check, --here and --push" >&2; exit 1 ;;
esac

# The context: the unit laid out for every architecture that is built.
context=$(mktemp -d)
trap 'rm -rf "$context"' EXIT
platforms=""
commit=""
evo_core=""
for arch in $arches; do
  [[ -x "out/$arch/bin/$SERVICE" ]] || { echo "image: out/$arch/bin/$SERVICE is missing; build '$arch' first (scripts/build.sh $arch)" >&2; exit 1; }
  built=$(sed -n 's/^ *"commit": "\(.*\)",$/\1/p' "out/$arch/build.json")
  [ -n "$built" ] || { echo "image: out/$arch/build.json names no commit" >&2; exit 1; }
  [ -z "$commit" ] || [ "$commit" = "$built" ] || { echo "image: out/$arch is built from $built, another from $commit" >&2; exit 1; }
  commit=$built
  # The evo-core the programs stand on, as their build wrote it down:
  # its tag and the commit the tag stood on.
  stands_on=$(sed -n 's/^ *"evo_core": "\(.*\)",$/\1/p' "out/$arch/build.json")
  [ -n "$stands_on" ] || { echo "image: out/$arch/build.json does not say which evo-core it stands on" >&2; exit 1; }
  [ -z "$evo_core" ] || [ "$evo_core" = "$stands_on" ] || { echo "image: out/$arch stands on evo-core $stands_on, another on $evo_core" >&2; exit 1; }
  evo_core=$stands_on
  unit_tree "out/$arch/bin/$SERVICE" "$context/$arch/rootfs"
  # Where evo looks for its socket; a machine has the same link.
  mkdir -p "$context/$arch/rootfs/var"
  ln -s /run "$context/$arch/rootfs/var/run"
  # The one account of the image, which the steward asks the name of.
  echo "evo:x:10001:10001:evo steward:/var/lib/evo:/sbin/nologin" > "$context/$arch/rootfs/etc/passwd"
  echo "evo:x:10001:" > "$context/$arch/rootfs/etc/group"
  mkdir -p "$context/$arch/kept/var/lib/evo" "$context/$arch/kept/run/evo" "$context/$arch/kept/tmp"
  chmod 750 "$context/$arch/kept/var/lib/evo"
  chmod 755 "$context/$arch/kept/run/evo"
  chmod 1777 "$context/$arch/kept/tmp"
  platforms="${platforms:+$platforms,}linux/$arch"
done

# The usher's own is MIT; the evo framework in the program is not.
build=(docker buildx build -f packaging/docker/Dockerfile
  --label "org.opencontainers.image.title=unifi-evo-usher"
  --label "org.opencontainers.image.description=unifi-evo-usher: guest Wi-Fi vouchers for UniFi sites, the steward with its face"
  --label "org.opencontainers.image.source=https://github.com/foonerd/unifi-evo-usher"
  --label "org.opencontainers.image.licenses=MIT AND BUSL-1.1"
  --label "org.opencontainers.image.version=$version"
  --label "org.opencontainers.image.revision=$commit"
  --label "org.evoframework.usher.evo-core=$evo_core")

# An image of several architectures is assembled in one go by a builder
# of its own, made where there is none; the image for this machine alone
# is built by the one docker has.
if [ "$mode" != --here ]; then
  builder=unifi-evo-usher
  docker buildx inspect "$builder" > /dev/null 2>&1 || docker buildx create --name "$builder" --driver docker-container > /dev/null
  build+=(--builder "$builder")
fi

# The image for this machine, loaded under <tag>, must name the program
# as the binary does.
load() {
  local tag="$1" arch want got
  arch=$(this_arch)
  [ -n "$arch" ] || { echo "image: a $(uname -m) machine runs neither architecture" >&2; exit 1; }
  echo "image: linux/$arch, as it runs"
  "${build[@]}" --platform "linux/$arch" --load -t "$tag" "$context"
  want="$SERVICE $version (commit $commit, $(arch_triple "$arch"))"
  got=$(docker run --rm "$tag" --version)
  [ "$got" = "$want" ] || { echo "image: the image says '$got', the build is '$want'" >&2; exit 1; }
  echo "  ok    $got"
}

case "$mode" in
  --check)
    echo "image: $platforms"
    "${build[@]}" --platform "$platforms" --output type=cacheonly "$context"
    load "unifi-evo-usher:check"
    docker image rm "unifi-evo-usher:check" > /dev/null
    echo "[OK] image"
    ;;
  --here)
    load "unifi-evo-usher:$version"
    echo "[OK] unifi-evo-usher:$version is kept on this machine"
    ;;
  --push)
    echo "image: $IMAGE:$version for $platforms"
    "${build[@]}" --platform "$platforms" -t "$IMAGE:$version" --push "$context"
    echo "[OK] pushed $IMAGE:$version"
    ;;
esac
