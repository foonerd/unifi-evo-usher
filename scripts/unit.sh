# shellcheck shell=bash
# The steward as it is installed: one tree, laid out from the root of a
# file system, which the steward's package and the image are both made
# of, so that a unit is the same however it came to be one. Sourced by
# the scripts that pack; they stand in the root of the repository.
#
#   unit_tree <program> <root>
#
# puts into <root>:
#
#   opt/evo/bin/evo-usher-steward     the program, from the file <program>
#   opt/evo/ui/                       the UI root: the door, the shell,
#                                     the widgets and the theme
#   opt/evo/catalogue/default.toml    the usher's catalogue, where evo
#                                     looks for one
#   etc/evo/evo.toml                  the steward's configuration
#   usr/share/doc/evo-usher-steward/  the licences
#
# The paths are evo's own defaults, so the configuration says little.
unit_tree() {
  local program="$1" root="$2" file
  install -D -m 755 "$program" "$root/opt/evo/bin/evo-usher-steward"
  # The UI root as the repository carries it, and nothing that only
  # lies about in a checkout.
  while IFS= read -r -d '' file; do
    install -D -m 644 "$file" "$root/opt/evo/$file"
  done < <(git ls-files -z -- ui)
  [ -f "$root/opt/evo/ui/index.html" ] || { echo "unit: the UI root has no door (ui/index.html)" >&2; return 1; }
  install -D -m 644 catalogue/usher.toml "$root/opt/evo/catalogue/default.toml"
  install -D -m 644 packaging/etc/evo.toml "$root/etc/evo/evo.toml"
  unit_licences "$root/usr/share/doc/evo-usher-steward"
}

# The licences that go wherever the steward goes, into a directory:
# `copyright`, which says what is whose and carries the MIT licence the
# usher's own is under, and `LICENSE-evo.txt`, the licence of the evo
# framework the program contains.
unit_licences() {
  local dir="$1"
  mkdir -p "$dir"
  cat packaging/licences/copyright LICENSE > "$dir/copyright"
  chmod 644 "$dir/copyright"
  install -m 644 packaging/licences/LICENSE-evo.txt "$dir/LICENSE-evo.txt"
}
