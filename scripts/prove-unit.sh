#!/bin/bash
# Proof that the steward's package makes a machine a unit: installed on
# this machine, started by systemd under every protection of its service,
# asked as a browser asks, started again, removed, purged.
#
#   sudo scripts/prove-unit.sh <evo-usher-steward_<version>_<arch>.deb>
#
# It is for a machine that is made for the proof and thrown away after
# it: one of CI's, or a virtual one. It installs a service and makes an
# account, so it refuses a machine that has evo already, and it leaves
# the machine without the package, the account apart.
set -euo pipefail

deb=${1:?the package of the steward, a .deb}
[ -f "$deb" ] || { echo "prove-unit: no file $deb" >&2; exit 1; }
[ "$(id -u)" -eq 0 ] || { echo "prove-unit: installing a package is root's; run it with sudo" >&2; exit 1; }
[ -d /run/systemd/system ] || { echo "prove-unit: systemd does not run this machine" >&2; exit 1; }
for path in /opt/evo /etc/evo /var/lib/evo; do
  [ ! -e "$path" ] || { echo "prove-unit: this machine has $path already; the proof is for a machine without evo" >&2; exit 1; }
done
for tool in curl jq base64 dpkg-deb; do
  command -v "$tool" > /dev/null || { echo "prove-unit: $tool is not on this machine" >&2; exit 1; }
done
deb=$(readlink -f "$deb")
version=$(dpkg-deb -f "$deb" Version)

FACE=https://127.0.0.1:8443
AUTHORITY=/var/lib/evo/state/https/ca.crt
CODE=/run/evo/pairing-code
work=$(mktemp -d)

fail() {
  echo "prove-unit: $*" >&2
  echo "prove-unit: what the service said:" >&2
  journalctl -u evo.service --no-pager -n 60 >&2 || true
  exit 1
}
trap 'rm -rf "$work"' EXIT

# One of the steward's operations, asked over HTTPS as a browser asks:
# the status goes to standard output, the answer into $work/answer.
#
#   ask <name> [<payload> [<bearer>]]
ask() {
  local name="$1" payload="${2:-}" bearer="${3:-}"
  local -a with=(--silent --show-error --cacert "$AUTHORITY" --max-time 40 --output "$work/answer" --write-out '%{http_code}')
  [ -z "$bearer" ] || with+=(--header "Authorization: Bearer $bearer")
  [ -z "$payload" ] || with+=(--header "Content-Type: application/json" --data "$payload")
  curl "${with[@]}" "$FACE/api/v1/$name"
}

# A request to a shelf, as `ask` takes it.
request() {
  jq -cn --arg shelf "$1" --arg request "$2" --arg payload "$(printf '%s' "$3" | base64 -w0)" \
    '{shelf: $shelf, request_type: $request, payload_b64: $payload}'
}

# What a shelf answered, out of $work/answer.
answered() {
  jq -r '.payload_b64' "$work/answer" | base64 -d
}

# Wait until the service runs and its face answers.
up() {
  local tries=0
  until systemctl is-active --quiet evo.service && [ -s "$AUTHORITY" ] && [ "$(curl --silent --cacert "$AUTHORITY" --max-time 5 --output /dev/null --write-out '%{http_code}' "$FACE/" || true)" = 200 ]; do
    tries=$((tries + 1))
    [ "$tries" -lt 120 ] || fail "the service did not come up"
    sleep 0.5
  done
}

echo "prove-unit: installing $(basename "$deb")"
DEBIAN_FRONTEND=noninteractive apt-get install -y "$deb" > "$work/install.log" 2>&1 || { cat "$work/install.log" >&2; fail "the package did not install"; }

echo "prove-unit: the service"
[ "$(systemctl is-enabled evo.service)" = enabled ] || fail "the service is not set to start with the machine"
up
echo "  ok    started by the package, and set to start with the machine"
said=$(/opt/evo/bin/evo-usher-steward --version)
case "$said" in
  "evo-usher-steward $version (commit "*) echo "  ok    $said" ;;
  *) fail "the program says '$said', the package is $version" ;;
esac
pid=$(systemctl show -p MainPID --value evo.service)
[ "$(ps -o user= -p "$pid" | tr -d ' ')" = evo ] || fail "the steward runs as $(ps -o user= -p "$pid"), not as evo"
[ "$(stat -c '%U %a' /var/lib/evo/state)" = "evo 700" ] || fail "what the steward keeps is $(stat -c '%U %a' /var/lib/evo/state), not evo's alone"
echo "  ok    runs as evo, which alone reads what it keeps"
# The protections are in force for the process, not only written down.
grep -q '^NoNewPrivs:[[:space:]]*1$' "/proc/$pid/status" || fail "the steward may gain privileges"
grep -q '^CapBnd:[[:space:]]*0000000000000000$' "/proc/$pid/status" || fail "the steward has capabilities within reach"
grep -q '^Seccomp:[[:space:]]*2$' "/proc/$pid/status" || fail "the steward's system calls are not filtered"
echo "  ok    no privilege to gain, no capability, its system calls filtered"

echo "prove-unit: the face"
curl --silent --cacert "$AUTHORITY" --max-time 10 "$FACE/" | grep -q 'door.js' || fail "the door is not served"
[ "$(ask describe_ui_stockings)" = 200 ] || fail "the steward does not say what is stocked: $(cat "$work/answer")"
for kind in usher.counter usher.types usher.controller usher.vouchers usher.networks usher.reserve; do
  grep -q "\"$kind\"" "$work/answer" || fail "nothing is drawn as $kind: $(cat "$work/answer")"
done
echo "  ok    the door is served, and every widget kind has its place"
for part in org.evoframework.usher.shell/shell.js org.evoframework.usher.widgets/index.js org.evoframework.usher.theme.frost/theme.css; do
  [ "$(curl --silent --cacert "$AUTHORITY" --max-time 10 --output /dev/null --write-out '%{http_code}' "$FACE/$part")" = 200 ] || fail "$part is not served"
done
echo "  ok    the shell, the widgets and the theme are served"

echo "prove-unit: who may ask"
info=$(request access.controller info '{}')
[ "$(ask request "$info")" = 403 ] || fail "a caller who presents nothing is answered: $(cat "$work/answer")"
echo "  ok    a caller who presents nothing is refused"
[ "$(stat -c '%U %a' "$CODE")" = "evo 600" ] || fail "the pairing code is $(stat -c '%U %a' "$CODE"), not the steward's alone"
code=$(cat "$CODE")
[[ "$code" =~ ^[0-9a-f]{12}$ ]] || fail "the pairing code is not twelve hexadecimal digits"
[ "$(ask pair_complete "$(jq -cn --arg code "$code" '{pair_id: "bootstrap", code: $code}')")" = 200 ] || fail "the code the unit made pairs no browser: $(cat "$work/answer")"
bearer=$(jq -r '.token' "$work/answer")
[ "$(ask request "$info" "$bearer")" = 200 ] || fail "a paired caller is refused: $(cat "$work/answer")"
[ "$(answered)" = '{"set":false}' ] || fail "the controller plugin answered $(answered)"
echo "  ok    the code the unit made pairs a browser, and it is answered"

echo "prove-unit: the voucher types"
PREDEFINED=/etc/evo/plugins.d/org.evoframework.usher.counter.toml
[ -f "$PREDEFINED" ] || fail "$PREDEFINED is not installed"
[ "$(ask request "$(request issue.counter types '{}')" "$bearer")" = 200 ] || fail "the voucher types are not told: $(cat "$work/answer")"
answered | jq -e '.predefined == ["One hour", "One day", "One week"] and (.types | map(.name)) == .predefined' > /dev/null || fail "the unit does not offer the voucher types its configuration predefines: $(answered)"
[ "$(ask request "$(request issue.counter drop_type '{"name":"One day"}')" "$bearer")" != 200 ] || fail "a predefined voucher type was dropped from the screen: $(cat "$work/answer")"
echo "  ok    the unit offers the voucher types its configuration predefines, and the screen drops none"
# The administrator predefines one more, where the others are written.
printf '\n[[types]]\nname = "Weekend"\nminutes = 4320\nguests = 2\n' >> "$PREDEFINED"

echo "prove-unit: what is kept"
kept='{"name":"Day","minutes":1440,"guests":1}'
[ "$(ask request "$(request issue.counter put_type "$kept")" "$bearer")" = 200 ] || fail "a voucher type is not kept: $(cat "$work/answer")"
systemctl restart evo.service
up
[ "$(systemctl show -p NRestarts --value evo.service)" = 0 ] || fail "the service was started again by itself"
[ "$(cat "$CODE")" != "$code" ] || fail "the pairing code is the same after a restart"
[ "$(ask pair_complete "$(jq -cn --arg code "$code" '{pair_id: "bootstrap", code: $code}')")" = 403 ] || fail "the code of the last start still pairs: $(cat "$work/answer")"
echo "  ok    started again, the unit has a new code and takes the old one no more"
[ "$(ask request "$(request issue.counter types '{}')" "$bearer")" = 200 ] || fail "the paired browser is refused after a restart: $(cat "$work/answer")"
answered | jq -e '.types | map(.name) == ["One hour", "One day", "One week", "Weekend", "Day"]' > /dev/null || fail "the voucher types are not the predefined and the one kept, through a restart: $(answered)"
answered | jq -e '.predefined == ["One hour", "One day", "One week", "Weekend"]' > /dev/null || fail "the voucher type predefined since is not on offer: $(answered)"
echo "  ok    the paired browser is still answered, the voucher type is still kept, and the one predefined since is on offer"

echo "prove-unit: the protections"
systemd-analyze security evo.service --no-pager | tail -n 1
# What the service wrote of warning or worse is shown; an error is one
# too many for a unit that was only installed and asked.
journalctl -u evo.service --no-pager -q -p warning -o cat | sed 's/^/  said  /' || true
errors=$(journalctl -u evo.service --no-pager -q -p err -o cat | grep -c . || true)
[ "$errors" -eq 0 ] || fail "the service wrote $errors line(s) of error"
echo "  ok    the service wrote no error"

echo "prove-unit: stopping and removing"
systemctl stop evo.service
[ "$(systemctl show -p Result --value evo.service)" = success ] || fail "the service did not stop cleanly: $(systemctl show -p Result --value evo.service)"
[ ! -e /run/evo ] || fail "/run/evo is still there after the service stopped"
echo "  ok    stopped cleanly, and its socket and its code are gone"
systemctl start evo.service
up
DEBIAN_FRONTEND=noninteractive apt-get remove -y evo-usher-steward > "$work/remove.log" 2>&1 || { cat "$work/remove.log" >&2; fail "the package was not removed"; }
! systemctl is-active --quiet evo.service || fail "the service still runs after the package was removed"
[ ! -e /opt/evo/bin/evo-usher-steward ] || fail "the program is still there"
[ -s /var/lib/evo/state/evo.db ] || fail "what the steward kept went with the package"
echo "  ok    removed: the service is gone, what it kept is still there"
DEBIAN_FRONTEND=noninteractive apt-get purge -y evo-usher-steward > "$work/purge.log" 2>&1 || { cat "$work/purge.log" >&2; fail "the package was not purged"; }
for path in /etc/evo /var/lib/evo; do
  [ ! -e "$path" ] || fail "$path is still there after the purge"
done
echo "  ok    purged: its configuration and what it kept are gone"

echo "[OK] prove-unit"
