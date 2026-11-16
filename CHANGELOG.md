# Changelog

All notable changes to unifi-evo-usher are recorded here. The format follows Keep a Changelog; versions follow Semantic Versioning.

## [0.1.0] - 2026-11-16

- The workspace and its two programs, `evo-usher-core` and `evo-usher-station`: each answers `--version` with its name, its version, the commit it was built from and the target it was built for, and refuses everything else with the usage and a non-zero exit. Nothing is issued, shown or printed yet
- Built static for amd64 and arm64, each on a machine of its own architecture, proven static and run where it was built
- Packed per architecture as an archive of both programs and as a Debian package per program, installed under `/opt/evo/bin`; the core as an image for both architectures, the static program on an empty image
- A release is signed with the evo release key: `MANIFEST` and its signature in every archive, `SHA256SUMS` and its signature beside the assets
