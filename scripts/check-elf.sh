#!/bin/bash
# Gate: every ELF file under a directory is built for the architecture it is
# shipped as, and linked static.
#
#   check-elf.sh <arch> <directory>
set -euo pipefail
ROOT=$(CDPATH='' cd -- "$(dirname "$0")/.." && pwd)
. "$ROOT/scripts/targets.sh"

arch="$1"
dir="$2"
rule="$(arch_elf_rule "$arch")" || { echo "check-elf: '$arch' is not an architecture unifi-evo-usher is built for" >&2; exit 1; }
fail=0
count=0

while IFS= read -r -d '' f; do
  desc="$(file -b "$f")"
  case "$desc" in ELF*) ;; *) continue ;; esac
  count=$((count + 1))
  rel="${f#"$dir"/}"
  ok=1
  detail="$rule"
  case "$desc" in *"$rule"*) ;; *) ok=0; detail="NOT $rule" ;; esac
  # A static program names no interpreter and needs no shared library.
  case "$desc" in
    *"statically linked"*|*"static-pie linked"*) detail="$detail, static" ;;
    *) ok=0; detail="$detail, NOT static" ;;
  esac
  # What the file needs is read whole before it is looked through: a
  # reader that leaves at the first line it finds ends the writer, and
  # the two together would then say that nothing was found.
  needs=$(readelf -d "$f" 2> /dev/null || true)
  if grep -q '(NEEDED)' <<< "$needs"; then
    ok=0
    detail="$detail, needs a shared library"
  fi
  if [[ "$ok" -eq 1 ]]; then
    echo "  ok    $rel ($detail)"
  else
    echo "  WRONG $rel ($detail; $desc)"
    fail=1
  fi
done < <(find "$dir" -type f -print0 | sort -z)

if [[ "$count" -eq 0 ]]; then
  echo "check-elf: no ELF files under $dir" >&2
  exit 1
fi
if [[ "$fail" -ne 0 ]]; then
  echo "check-elf: binaries not built for '$arch' ($rule), or not static" >&2
  exit 1
fi
echo "[OK] $count ELF file(s) built for '$arch' ($rule), static"
