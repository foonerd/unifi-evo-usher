#!/bin/bash
# Build the programs for one architecture, with their proof:
#
#   build.sh <arch>          amd64 or arm64
#
# into out/<arch>/: bin/<program> for every program, and build.json, which
# names the version, the commit, the target, the compiler, and every file
# with its digest and size. The programs are proven built for the
# architecture and static, and, on a machine of that architecture, each is
# run and must name itself as this build: nothing is emulated, so a build
# for another architecture is proven where it is built, not here.
set -euo pipefail
ROOT=$(CDPATH='' cd -- "$(dirname "$0")/.." && pwd)
cd "$ROOT"
. scripts/targets.sh
unset CARGO_TARGET_DIR

arch=${1:?an architecture: amd64 or arm64}
triple=$(arch_triple "$arch") || { echo "build: '$arch' is not an architecture unifi-evo-usher is built for ($ARCHES)" >&2; exit 1; }
version=$(workspace_version)
commit=$(git rev-parse --short HEAD 2> /dev/null) || { echo "build: this is not a checkout with a commit; a build names the commit it is made from" >&2; exit 1; }

echo "build: $arch ($triple), $version at $commit"
packages=()
for program in $PROGRAMS; do
  packages+=(-p "$program")
done
EVO_USHER_COMMIT="$commit" cargo build --release --locked --target "$triple" "${packages[@]}"

out="out/$arch"
rm -rf "$out"
for program in $PROGRAMS; do
  install -D -m 755 "target/$triple/release/$program" "$out/bin/$program"
done

echo "build: architecture and linkage"
scripts/check-elf.sh "$arch" "$out/bin"

echo "build: the programs as they run"
if [ "$(this_arch)" = "$arch" ]; then
  for program in $PROGRAMS; do
    want="$program $version (commit $commit, $triple)"
    got=$("$out/bin/$program" --version)
    [ "$got" = "$want" ] || { echo "build: $program says '$got', the build is '$want'" >&2; exit 1; }
    echo "  ok    $got"
    # Anything else is refused: no output, the usage on standard error.
    set +e
    said=$("$out/bin/$program" 2> /dev/null)
    status=$?
    set -e
    [ "$status" -eq 2 ] && [ -z "$said" ] || { echo "build: $program with nothing asked exited $status and wrote '$said'" >&2; exit 1; }
  done
else
  echo "  this machine is $(uname -m): the $arch programs were not run here"
fi

echo "build: build.json"
{
  printf '{\n'
  printf '  "name": "unifi-evo-usher",\n'
  printf '  "version": "%s",\n' "$version"
  printf '  "commit": "%s",\n' "$commit"
  printf '  "arch": "%s",\n' "$arch"
  printf '  "target": "%s",\n' "$triple"
  printf '  "rustc": "%s",\n' "$(rustc --version)"
  printf '  "files": {\n'
  last=${PROGRAMS##* }
  for program in $PROGRAMS; do
    file="$out/bin/$program"
    comma=","
    [ "$program" = "$last" ] && comma=""
    printf '    "bin/%s": { "sha256": "%s", "size": %s }%s\n' \
      "$program" "$(sha256sum "$file" | cut -d' ' -f1)" "$(stat -c %s "$file")" "$comma"
  done
  printf '  }\n'
  printf '}\n'
} > "$out/build.json"
cat "$out/build.json"
echo "[OK] build $arch"
