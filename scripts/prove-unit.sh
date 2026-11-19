#!/bin/bash
# Proof that the steward's package makes a machine a unit: installed on
# this machine, started by systemd under every protection of its service,
# asked as a browser asks, started again, removed, and, where the
# package is Debian's, purged.
#
#   sudo scripts/prove-unit.sh <evo-usher-steward_<version>_<arch>.deb>
#   sudo scripts/prove-unit.sh <evo-usher-steward-<version>-1.<machine>.rpm>
#
# The package is installed and removed with the machine's own package
# manager: apt for a Debian package, dnf for an RPM. An RPM is removed
# and never purged, so the proof of one ends on what removing leaves:
# what the steward kept, the configuration that was changed, and the
# account.
#
# It is for a machine that is made for the proof and thrown away after
# it: one of CI's, or a virtual one. It installs a service and makes an
# account, so it refuses a machine that has evo already, and it leaves
# the machine without the package, the account apart.
set -euo pipefail

package=${1:?the package of the steward, a .deb or an .rpm}
[ -f "$package" ] || { echo "prove-unit: no file $package" >&2; exit 1; }
case "$package" in
  *.deb) format=deb; manager="dpkg-deb apt-get" ;;
  *.rpm) format=rpm; manager="rpm dnf" ;;
  *) echo "prove-unit: $package is neither a .deb nor an .rpm" >&2; exit 1 ;;
esac
[ "$(id -u)" -eq 0 ] || { echo "prove-unit: installing a package is root's; run it with sudo" >&2; exit 1; }
[ -d /run/systemd/system ] || { echo "prove-unit: systemd does not run this machine" >&2; exit 1; }
for path in /opt/evo /etc/evo /var/lib/evo; do
  [ ! -e "$path" ] || { echo "prove-unit: this machine has $path already; the proof is for a machine without evo" >&2; exit 1; }
done
for tool in curl jq base64 $manager; do
  command -v "$tool" > /dev/null || { echo "prove-unit: $tool is not on this machine" >&2; exit 1; }
done
package=$(readlink -f "$package")
case "$format" in
  deb) version=$(dpkg-deb -f "$package" Version) ;;
  rpm) version=$(rpm -qp --queryformat '%{VERSION}' "$package") ;;
esac

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

echo "prove-unit: installing $(basename "$package")"
case "$format" in
  deb) DEBIAN_FRONTEND=noninteractive apt-get install -y "$package" > "$work/install.log" 2>&1 ;;
  rpm) dnf install -y "$package" > "$work/install.log" 2>&1 ;;
esac || { cat "$work/install.log" >&2; fail "the package did not install"; }
# A step of the package's own that failed is one rpm goes on after.
! grep -q -i 'scriptlet failed' "$work/install.log" || { cat "$work/install.log" >&2; fail "a step of the package's own failed"; }

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
# Where the machine confines its programs, the steward is one of them.
if command -v getenforce > /dev/null 2>&1 && [ "$(getenforce)" != Disabled ]; then
  echo "  ok    SELinux is $(getenforce | tr '[:upper:]' '[:lower:]'), and the steward runs under it as $(ps -o label= -p "$pid" | tr -d ' ')"
fi
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
# Ask the counter as the paired browser does.
counter() { ask request "$(request issue.counter "$1" "$2")" "$bearer"; }
[ "$(counter types '{}')" = 200 ] || fail "the voucher types are not told: $(cat "$work/answer")"
answered | jq -e '.types == [] and .default == null and .disabled == []' > /dev/null || fail "a new unit has voucher types nobody defined: $(answered)"
[ "$(counter put_type '{"name":"Day","minutes":1440,"guests":1}')" = 200 ] || fail "a voucher type is not defined: $(cat "$work/answer")"
[ "$(counter put_type '{"name":"Stay","minutes":10080,"up_to_days":true,"guests":2}')" = 200 ] || fail "a voucher type with days to choose is not defined: $(cat "$work/answer")"
[ "$(counter enable_type '{"name":"Stay","enabled":false}')" = 200 ] || fail "a voucher type is not disabled: $(cat "$work/answer")"
[ "$(counter offered '{}')" = 200 ] || fail "what a front desk is offered is not told: $(cat "$work/answer")"
answered | jq -e '(.types | map(.name)) == ["Day"] and .default == "Day"' > /dev/null || fail "a front desk is not offered the enabled types and the default: $(answered)"
[ "$(counter order_types '{"names":["Stay","Day"]}')" = 200 ] || fail "the voucher types are not put in an order: $(cat "$work/answer")"
echo "  ok    a new unit has no voucher type; its administrator defines them and their order, and a front desk is offered the enabled ones"

echo "prove-unit: what is kept"
systemctl restart evo.service
up
[ "$(systemctl show -p NRestarts --value evo.service)" = 0 ] || fail "the service was started again by itself"
[ "$(cat "$CODE")" != "$code" ] || fail "the pairing code is the same after a restart"
[ "$(ask pair_complete "$(jq -cn --arg code "$code" '{pair_id: "bootstrap", code: $code}')")" = 403 ] || fail "the code of the last start still pairs: $(cat "$work/answer")"
echo "  ok    started again, the unit has a new code and takes the old one no more"
[ "$(ask request "$(request issue.counter types '{}')" "$bearer")" = 200 ] || fail "the paired browser is refused after a restart: $(cat "$work/answer")"
answered | jq -e '(.types | map(.name)) == ["Stay", "Day"] and .default == "Day" and .disabled == ["Stay"]' > /dev/null || fail "the voucher types, their order, the default and what is disabled were not kept through a restart: $(answered)"
echo "  ok    the paired browser is still answered, and the voucher types are kept as they were defined"

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
# The configuration that is not as the package brought it: rpm keeps
# each such file when the package goes.
changed=""
if [ "$format" = rpm ]; then
  changed=$(rpm -V evo-usher-steward | awk '$2 == "c" { print $3 }' || true)
fi
case "$format" in
  deb) DEBIAN_FRONTEND=noninteractive apt-get remove -y evo-usher-steward > "$work/remove.log" 2>&1 ;;
  rpm) dnf remove -y evo-usher-steward > "$work/remove.log" 2>&1 ;;
esac || { cat "$work/remove.log" >&2; fail "the package was not removed"; }
! systemctl is-active --quiet evo.service || { cat "$work/remove.log" >&2 || true; fail "the service still runs after the package was removed"; }
[ ! -e /opt/evo/bin/evo-usher-steward ] || fail "the program is still there"
[ -s /var/lib/evo/state/evo.db ] || fail "what the steward kept went with the package"
echo "  ok    removed: the service is gone, what it kept is still there"
if [ "$format" = deb ]; then
  DEBIAN_FRONTEND=noninteractive apt-get purge -y evo-usher-steward > "$work/purge.log" 2>&1 || { cat "$work/purge.log" >&2; fail "the package was not purged"; }
  for path in /etc/evo /var/lib/evo; do
    [ ! -e "$path" ] || fail "$path is still there after the purge"
  done
  echo "  ok    purged: its configuration and what it kept are gone"
else
  # An RPM is removed and never purged. What is left is the operator's:
  # what the steward kept, the account that owns it, and of the
  # configuration the files that were changed, each beside its old place
  # as .rpmsave. The rest of the package is gone.
  ! systemctl cat evo.service > /dev/null 2>&1 || fail "the service is still known to the machine"
  getent passwd evo > /dev/null || fail "the account went with the package"
  for file in $changed; do
    [ -f "$file.rpmsave" ] || fail "$file was changed, and is not kept as $file.rpmsave"
  done
  left=$(find /opt/evo /etc/evo -type f ! -name '*.rpmsave' 2> /dev/null | head -n 1 || true)
  [ -z "$left" ] || fail "$left is still there after the package was removed"
  echo "  ok    left to the operator: what the steward kept, the account, and the configuration that was changed ($(echo "$changed" | grep -c . || true) file(s), as .rpmsave)"
fi

echo "[OK] prove-unit"
