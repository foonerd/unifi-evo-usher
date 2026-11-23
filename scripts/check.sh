#!/bin/bash
# The workshop pass CI runs: formatting, lints with warnings denied, the
# tests, the documentation with warnings denied, the version and the
# changelog in step, the notices of the libraries in step with what the
# programs contain, the scripts through shellcheck, the workflows through
# actionlint. Run it before a commit.
set -euo pipefail
ROOT=$(CDPATH='' cd -- "$(dirname "$0")/.." && pwd)
cd "$ROOT"
. scripts/targets.sh

echo "check: format"
cargo fmt --all -- --check

echo "check: clippy"
cargo clippy --workspace --all-targets --locked -- -D warnings

echo "check: tests"
cargo test --workspace --locked

echo "check: documentation"
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked

echo "check: version and changelog"
version=$(workspace_version)
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "check: the workspace's version '$version' is not major.minor.patch" >&2; exit 1; }
# The section is read whole before it is asked whether there is one: a
# reader that leaves at the first line ends a writer that has more to
# say, and the check would then say that there is no section.
notes=$(scripts/release-notes.sh "$version")
[ -n "$notes" ] || { echo "check: CHANGELOG.md has no section for $version" >&2; exit 1; }
# The one command on the front page names the image's tag `latest`,
# which is laid on the latest release when it is made public
# (.github/workflows/latest.yml): it works as it stands, whatever
# version the workspace has come to.
grep -q -F "ghcr.io/foonerd/unifi-evo-usher:latest" README.md || { echo "check: the command on README.md's front page does not name ghcr.io/foonerd/unifi-evo-usher:latest" >&2; exit 1; }

echo "check: the evo-core the lock names"
# The usher stands on a tag of evo-core's, and the lock names the commit
# the tag stood on when the lock was made. The tag v0.1.13.2 was laid on
# another commit before the one it stands on now: the lock is held to
# the one it stands on now, every package of evo-core's at it and none
# at any other, so that no build is made on the commit the tag left.
EVO_CORE_TAG=v0.1.13.2
EVO_CORE_COMMIT=d1962a2ccdf579d2c35621ca9331b27554d0198b
of_evo_core=$(grep -F 'github.com/foonerd/evo-core.git' Cargo.lock || true)
[ -n "$of_evo_core" ] || { echo "check: Cargo.lock names no package of evo-core's" >&2; exit 1; }
elsewhere=$(printf '%s\n' "$of_evo_core" | grep -v -F "evo-core.git?tag=$EVO_CORE_TAG#$EVO_CORE_COMMIT\"" || true)
[ -z "$elsewhere" ] || { echo "check: Cargo.lock names evo-core elsewhere than at $EVO_CORE_TAG, commit $EVO_CORE_COMMIT:" >&2; printf '%s\n' "$elsewhere" | sort -u >&2; exit 1; }
grep -q -F "tag = \"$EVO_CORE_TAG\"" Cargo.toml || { echo "check: Cargo.toml does not name evo-core's tag $EVO_CORE_TAG" >&2; exit 1; }
echo "  evo-core $EVO_CORE_TAG at $EVO_CORE_COMMIT, $(printf '%s\n' "$of_evo_core" | wc -l) packages"

echo "check: notices"
scripts/notices.sh --check

echo "check: shellcheck"
shellcheck -x -S warning scripts/*.sh

echo "check: actionlint"
if command -v actionlint > /dev/null; then
  actionlint -no-color .github/workflows/*.yml
elif command -v docker > /dev/null && docker info > /dev/null 2>&1; then
  docker run --rm -v "$PWD:/repo" -w /repo rhysd/actionlint:latest -no-color .github/workflows/*.yml
else
  echo "check: neither actionlint nor a running docker is here; the workflows were not linted" >&2
  exit 1
fi

echo "[OK] check"
