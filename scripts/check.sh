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
scripts/release-notes.sh "$version" | grep -q . || { echo "check: CHANGELOG.md has no section for $version" >&2; exit 1; }

echo "check: notices"
scripts/notices.sh --check

echo "check: shellcheck"
shellcheck -x -S warning scripts/*.sh

echo "check: actionlint"
if command -v actionlint > /dev/null; then
  actionlint -no-color .github/workflows/*.yml
elif command -v docker > /dev/null && docker info > /dev/null 2>&1; then
  docker run --rm -v "$PWD:/repo" -w /repo rhysd/actionlint:latest -no-color .github/workflows/ci.yml .github/workflows/release.yml
else
  echo "check: neither actionlint nor a running docker is here; the workflows were not linted" >&2
  exit 1
fi

echo "[OK] check"
