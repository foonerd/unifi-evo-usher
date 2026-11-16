# shellcheck shell=bash
# What unifi-evo-usher is built as, sourced by the scripts that build, prove
# and pack it. An architecture goes by its Debian name, which is also the
# name Docker gives it:
#
#   amd64  x86-64
#   arm64  64-bit ARM
#
# Every program is built static against musl, so it depends on nothing the
# machine has or may lose, and the same file serves a bare machine, a
# Debian package and an image with nothing else in it.

# shellcheck disable=SC2034  # read by the scripts that source this file
ARCHES="amd64 arm64"
# shellcheck disable=SC2034
PROGRAMS="evo-usher-core evo-usher-station"

# The Rust target an architecture is built for.
arch_triple() {
  case "$1" in
    amd64) echo "x86_64-unknown-linux-musl" ;;
    arm64) echo "aarch64-unknown-linux-musl" ;;
    *)     return 1 ;;
  esac
}

# What `uname -m` says on a machine of this architecture.
arch_machine() {
  case "$1" in
    amd64) echo "x86_64" ;;
    arm64) echo "aarch64" ;;
    *)     return 1 ;;
  esac
}

# What `file` must report for a binary of this architecture.
arch_elf_rule() {
  case "$1" in
    amd64) echo "x86-64" ;;
    arm64) echo "ARM aarch64" ;;
    *)     return 1 ;;
  esac
}

# The architecture of the machine this runs on, or nothing where it is not
# one unifi-evo-usher is built for.
this_arch() {
  case "$(uname -m)" in
    x86_64)  echo "amd64" ;;
    aarch64) echo "arm64" ;;
    *)       echo "" ;;
  esac
}

# One line on what a program is, for its package.
program_summary() {
  case "$1" in
    evo-usher-core)    echo "unifi-evo-usher core: mints, stocks and accounts for UniFi guest vouchers" ;;
    evo-usher-station) echo "unifi-evo-usher station: hands a guest a voucher, on a screen or a label" ;;
    *)                 return 1 ;;
  esac
}

# The version the workspace carries.
workspace_version() {
  sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n1
}
