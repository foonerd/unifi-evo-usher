#!/bin/bash
# The core as an image for both architectures, from what scripts/build.sh
# made under out/ (packaging/docker/Dockerfile: the static program on an
# empty image).
#
#   image.sh --check   builds the image for every architecture, then this
#                      machine's own once more, loaded and run: the image
#                      must name the program as the binary does
#   image.sh --push    builds it for every architecture and pushes it as
#                      $IMAGE:<version> (default ghcr.io/foonerd/unifi-evo-usher);
#                      the login is the caller's
#
# No tag but the version is pushed: which version is the latest is said by
# hand, once a release has been tried.
set -euo pipefail
ROOT=$(CDPATH='' cd -- "$(dirname "$0")/.." && pwd)
cd "$ROOT"
. scripts/targets.sh

mode=${1:?--check or --push}
IMAGE=${IMAGE:-ghcr.io/foonerd/unifi-evo-usher}
version=$(workspace_version)

platforms=""
for arch in $ARCHES; do
  [[ -x "out/$arch/bin/evo-usher-core" ]] || { echo "image: out/$arch/bin/evo-usher-core is missing; build '$arch' first (scripts/build.sh $arch)" >&2; exit 1; }
  platforms="${platforms:+$platforms,}linux/$arch"
done
commit=$(sed -n 's/^ *"commit": "\(.*\)",$/\1/p' out/amd64/build.json)
[ -n "$commit" ] || { echo "image: out/amd64/build.json names no commit" >&2; exit 1; }

# A builder that assembles an image of several architectures in one go.
builder=unifi-evo-usher
docker buildx inspect "$builder" > /dev/null 2>&1 || docker buildx create --name "$builder" --driver docker-container > /dev/null

build=(docker buildx build --builder "$builder" -f packaging/docker/Dockerfile
  --label "org.opencontainers.image.title=unifi-evo-usher"
  --label "org.opencontainers.image.description=The core of unifi-evo-usher: guest Wi-Fi vouchers for UniFi sites"
  --label "org.opencontainers.image.source=https://github.com/foonerd/unifi-evo-usher"
  --label "org.opencontainers.image.licenses=MIT"
  --label "org.opencontainers.image.version=$version"
  --label "org.opencontainers.image.revision=$commit")

case "$mode" in
  --check)
    echo "image: $platforms"
    "${build[@]}" --platform "$platforms" --output type=cacheonly out
    arch=$(this_arch)
    [ -n "$arch" ] || { echo "image: a $(uname -m) machine runs neither architecture" >&2; exit 1; }
    echo "image: linux/$arch, as it runs"
    "${build[@]}" --platform "linux/$arch" --load -t "unifi-evo-usher:check" out
    want="evo-usher-core $version (commit $commit, $(arch_triple "$arch"))"
    got=$(docker run --rm "unifi-evo-usher:check" --version)
    docker image rm "unifi-evo-usher:check" > /dev/null
    [ "$got" = "$want" ] || { echo "image: the image says '$got', the build is '$want'" >&2; exit 1; }
    echo "  ok    $got"
    echo "[OK] image"
    ;;
  --push)
    echo "image: $IMAGE:$version for $platforms"
    "${build[@]}" --platform "$platforms" -t "$IMAGE:$version" --push out
    echo "[OK] pushed $IMAGE:$version"
    ;;
  *)
    echo "image: '$mode' is neither --check nor --push" >&2
    exit 1
    ;;
esac
