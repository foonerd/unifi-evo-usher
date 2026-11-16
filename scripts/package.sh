#!/bin/bash
# The release's packages, from what scripts/build.sh made under out/, for
# every architecture:
#
#   dist/unifi-evo-usher-<version>-<arch>.tar.gz   both programs, build.json,
#                                                  README.md, CHANGELOG.md
#                                                  and LICENSE
#   dist/<program>_<version>_<arch>.deb            one program, installed
#                                                  under /opt/evo/bin
#
# Every file is checked against the digest build.json recorded for it, so a
# build that came through a workflow's artifact store is the build that was
# proven. The archives and the packages are made the same for the same
# tree: names sorted, owners fixed, every date the commit's.
#
#   scripts/package.sh [--from <dir>] [<arch>...]
#
# <dir> holds the <arch>/ trees (default: out). With no architecture named,
# every one is packed and a missing one is an error.
set -euo pipefail
ROOT=$(CDPATH='' cd -- "$(dirname "$0")/.." && pwd)
cd "$ROOT"
. scripts/targets.sh

from="out"
arches=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --from) from="$2"; shift 2 ;;
    -*) echo "package: unknown option '$1'" >&2; exit 1 ;;
    *)
      arch_triple "$1" > /dev/null || { echo "package: '$1' is not an architecture unifi-evo-usher is built for ($ARCHES)" >&2; exit 1; }
      arches="$arches $1"
      shift
      ;;
  esac
done
[ -n "$arches" ] || arches="$ARCHES"
version=$(workspace_version)
# The commit's time, for every date in what is packed.
epoch=$(git log -1 --format=%ct 2> /dev/null) || { echo "package: this is not a checkout with a commit" >&2; exit 1; }

for arch in $arches; do
  [[ -f "$from/$arch/build.json" ]] || { echo "package: $from/$arch/build.json is missing; build '$arch' first (scripts/build.sh $arch)" >&2; exit 1; }
  grep -q "\"version\": \"$version\"" "$from/$arch/build.json" || { echo "package: $from/$arch was not built as $version" >&2; exit 1; }
  for program in $PROGRAMS; do
    [[ -f "$from/$arch/bin/$program" ]] || { echo "package: $from/$arch/bin/$program is missing" >&2; exit 1; }
  done
  # Each file as build.json has it, and nothing build.json does not name.
  while IFS= read -r -d '' f; do
    rel="${f#"$from/$arch"/}"
    want=$(sed -n "s|^ *\"$rel\": { \"sha256\": \"\([0-9a-f]*\)\".*|\1|p" "$from/$arch/build.json")
    [[ -n "$want" ]] || { echo "package: $arch: $rel is not in build.json" >&2; exit 1; }
    have=$(sha256sum "$f" | cut -d' ' -f1)
    [[ "$have" == "$want" ]] || { echo "package: $arch: $rel is not the file build.json names (digest differs)" >&2; exit 1; }
  done < <(find "$from/$arch" -type f ! -name build.json -print0)
done

stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
rm -rf dist && mkdir -p dist

for arch in $arches; do
  top="unifi-evo-usher-$version-$arch"
  mkdir -p "$stage/$top/bin"
  for program in $PROGRAMS; do
    install -m 755 "$from/$arch/bin/$program" "$stage/$top/bin/$program"
  done
  install -m 644 "$from/$arch/build.json" README.md CHANGELOG.md LICENSE "$stage/$top/"
  tar -C "$stage" --sort=name --owner=0 --group=0 --numeric-owner --mtime="@$epoch" -czf "dist/$top.tar.gz" "$top"
  echo "[OK] dist/$top.tar.gz: $(stat -c %s "dist/$top.tar.gz") bytes"

  for program in $PROGRAMS; do
    root="$stage/deb-$arch-$program"
    install -D -m 755 "$from/$arch/bin/$program" "$root/opt/evo/bin/$program"
    install -D -m 644 LICENSE "$root/usr/share/doc/$program/copyright"
    mkdir -p "$root/DEBIAN"
    size=$(du -sk --exclude=DEBIAN "$root" | cut -f1)
    {
      echo "Package: $program"
      echo "Version: $version"
      echo "Architecture: $arch"
      echo "Maintainer: Just a nerd <157698061+foonerd@users.noreply.github.com>"
      echo "Installed-Size: $size"
      echo "Section: net"
      echo "Priority: optional"
      echo "Homepage: https://github.com/foonerd/unifi-evo-usher"
      echo "Description: $(program_summary "$program")"
      echo " One static program, installed as /opt/evo/bin/$program. It depends on"
      echo " no other package."
    } > "$root/DEBIAN/control"
    find "$root" -exec touch -h -d "@$epoch" {} +
    deb="dist/${program}_${version}_${arch}.deb"
    SOURCE_DATE_EPOCH="$epoch" dpkg-deb --root-owner-group -Zxz --build "$root" "$deb" > /dev/null
    echo "[OK] $deb: $(stat -c %s "$deb") bytes"
  done
done
