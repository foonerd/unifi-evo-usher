#!/bin/bash
# The release's packages, from what scripts/build.sh made under out/, for
# every architecture:
#
#   dist/unifi-evo-usher-<version>-<arch>.tar.gz   every program under bin/;
#                                                  what stands beside the
#                                                  steward on a unit (ui/,
#                                                  catalogue/, etc/, systemd/,
#                                                  sysusers.d/); build.json,
#                                                  README.md, CHANGELOG.md and
#                                                  the licences
#   dist/evo-usher-steward_<version>_<arch>.deb    the steward as a unit: the
#                                                  program, its face, its
#                                                  catalogue, its
#                                                  configuration and the
#                                                  service that runs it
#   dist/<program>_<version>_<arch>.deb            another program, on its
#                                                  own, under /opt/evo/bin
#
# A licence goes wherever what it covers goes. The steward contains the
# evo framework, so evo's licence is in the archive and in the steward's
# package; the station carries the typeface Inter, so the licence Inter
# comes under is in the archive and in the station's package.
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
. scripts/unit.sh
# Whoever packs, what is packed is the same: no mode is the packer's own.
umask 022

# The licence of the typeface the station carries.
FONT_LICENCE=crates/render/assets/fonts/OFL.txt

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

# A Debian package of the tree in <root>, which has its DEBIAN/control but
# for the size: the same package for the same tree.
#
#   pack_deb <root> <file>
pack_deb() {
  local root="$1" deb="$2" size
  size=$(du -sk --exclude=DEBIAN "$root" | cut -f1)
  echo "Installed-Size: $size" >> "$root/DEBIAN/control"
  find "$root" -exec touch -h -d "@$epoch" {} +
  SOURCE_DATE_EPOCH="$epoch" dpkg-deb --root-owner-group -Zxz --build "$root" "$deb" > /dev/null
  echo "[OK] $deb: $(stat -c %s "$deb") bytes"
}

# What every package of unifi-evo-usher says of itself.
#
#   control <program> <arch>
control() {
  echo "Package: $1"
  echo "Version: $version"
  echo "Architecture: $2"
  echo "Maintainer: Just a nerd <157698061+foonerd@users.noreply.github.com>"
  echo "Section: net"
  echo "Priority: optional"
  echo "Homepage: https://github.com/foonerd/unifi-evo-usher"
  echo "Description: $(program_summary "$1")"
}

for arch in $arches; do
  top="unifi-evo-usher-$version-$arch"
  mkdir -p "$stage/$top/bin"
  for program in $PROGRAMS; do
    install -m 755 "$from/$arch/bin/$program" "$stage/$top/bin/$program"
  done
  # What stands beside the steward on a unit, for a machine that takes no
  # Debian package: README.md says where each goes.
  while IFS= read -r -d '' file; do
    install -D -m 644 "$file" "$stage/$top/$file"
  done < <(git ls-files -z -- ui catalogue)
  install -D -m 644 packaging/etc/evo.toml "$stage/$top/etc/evo.toml"
  install -D -m 644 packaging/systemd/evo.service "$stage/$top/systemd/evo.service"
  install -D -m 644 packaging/sysusers.d/evo.conf "$stage/$top/sysusers.d/evo.conf"
  install -m 644 "$from/$arch/build.json" README.md CHANGELOG.md LICENSE "$stage/$top/"
  install -m 644 packaging/licences/LICENSE-evo.txt "$stage/$top/LICENSE-evo.txt"
  install -m 644 "$FONT_LICENCE" "$stage/$top/OFL-Inter.txt"
  tar -C "$stage" --sort=name --owner=0 --group=0 --numeric-owner --mtime="@$epoch" -czf "dist/$top.tar.gz" "$top"
  echo "[OK] dist/$top.tar.gz: $(stat -c %s "dist/$top.tar.gz") bytes"

  for program in $PROGRAMS; do
    root="$stage/deb-$arch-$program"
    mkdir -p "$root/DEBIAN"
    if [[ "$program" == "$SERVICE" ]]; then
      # The steward as a unit: the tree the image is made of too, and the
      # service that runs it on a machine.
      unit_tree "$from/$arch/bin/$program" "$root"
      install -D -m 644 packaging/systemd/evo.service "$root/usr/lib/systemd/system/evo.service"
      install -D -m 644 packaging/sysusers.d/evo.conf "$root/usr/lib/sysusers.d/$program.conf"
      install -m 755 packaging/deb/postinst packaging/deb/prerm packaging/deb/postrm "$root/DEBIAN/"
      # The operator's to change, and kept when a newer package comes.
      echo "/etc/evo/evo.toml" > "$root/DEBIAN/conffiles"
      {
        control "$program" "$arch"
        echo " The evo steward with the usher's plugins, as one static program under"
        echo " /opt/evo, with the usher's face, its catalogue, its configuration"
        echo " (/etc/evo/evo.toml) and the service that runs it (evo.service) as an"
        echo " account of its own. The face is served on port 8443."
        # The account is made with one of the two.
        echo "Depends: systemd | adduser"
      } > "$root/DEBIAN/control"
    else
      install -D -m 755 "$from/$arch/bin/$program" "$root/opt/evo/bin/$program"
      install -D -m 644 LICENSE "$root/usr/share/doc/$program/copyright"
      if [[ "$program" == evo-usher-station ]]; then
        install -D -m 644 "$FONT_LICENCE" "$root/usr/share/doc/$program/OFL-Inter.txt"
      fi
      {
        control "$program" "$arch"
        echo " One static program, installed as /opt/evo/bin/$program. It depends on"
        echo " no other package."
      } > "$root/DEBIAN/control"
    fi
    pack_deb "$root" "dist/${program}_${version}_${arch}.deb"
  done
done
