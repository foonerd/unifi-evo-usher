#!/bin/bash
# Proof that the steward's package makes a machine a unit: installed on
# this machine, started by systemd under every protection of its service,
# set up and signed in to as a browser does it, asked as a browser asks,
# given its site's name and time zone where it is started, started
# again, run at the extended level, removed, and, where the package is
# Debian's, purged.
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
CODE=/run/evo/unit-code
PEOPLE=/var/lib/evo/state/usher/people.json
SITE=/var/lib/evo/plugins/org.evoframework.usher.site/state/site.json
LEVEL=/etc/systemd/system/evo.service.d/level.conf
GIVEN=/etc/systemd/system/evo.service.d/site.conf
# Who the proof sets the unit up as, and signs in as. Made up.
NAME="Anna"
PASSWORD="three small owls sing"
work=$(mktemp -d)

fail() {
  echo "prove-unit: $*" >&2
  echo "prove-unit: what the service said:" >&2
  journalctl -u evo.service --no-pager -n 60 >&2 || true
  exit 1
}
trap 'rm -rf "$work"; rm -f "$LEVEL" "$GIVEN"; rmdir "$(dirname "$LEVEL")" 2> /dev/null || true' EXIT

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

# One of the usher's own routes, asked the same way.
#
#   usher <route> [<payload> [<bearer>]]
usher() {
  local route="$1" payload="${2:-}" bearer="${3:-}"
  local -a with=(--silent --show-error --cacert "$AUTHORITY" --max-time 40 --output "$work/answer" --write-out '%{http_code}')
  [ -z "$bearer" ] || with+=(--header "Authorization: Bearer $bearer")
  [ -z "$payload" ] || with+=(--header "Content-Type: application/json" --data "$payload")
  curl "${with[@]}" "$FACE/usher/v1/$route"
}

# A name and a password, as the routes that take them do.
named() {
  jq -cn --arg name "$1" --arg password "$2" '{name: $name, password: $password}'
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
[ "$(ask describe_active_ui_selection)" = 200 ] || fail "the door is not told which shell is active: $(cat "$work/answer")"
echo "  ok    the door is served, and is told which shell to open"
for part in org.evoframework.usher.shell/index.html org.evoframework.usher.shell/shell.css org.evoframework.usher.theme.frost/theme.css; do
  [ "$(curl --silent --cacert "$AUTHORITY" --max-time 10 --output /dev/null --write-out '%{http_code}' "$FACE/$part")" = 200 ] || fail "$part is not served"
done
# Every module a browser loads for the face is served: the shell's and
# the widget pack's own, and whatever each imports from beside it. One
# that is missing leaves a browser with an empty page.
modules=0
for pack in org.evoframework.usher.shell/shell.js org.evoframework.usher.widgets/index.js; do
  dir=${pack%/*}
  seen=" "
  queue=${pack##*/}
  while [ -n "$queue" ]; do
    module=${queue%% *}
    queue=${queue#"$module"}
    queue=${queue# }
    case "$seen" in *" $module "*) continue ;; esac
    seen="$seen$module "
    [ "$(curl --silent --cacert "$AUTHORITY" --max-time 10 --output "$work/module" --write-out '%{http_code}' "$FACE/$dir/$module")" = 200 ] || fail "$dir/$module, which the face loads, is not served"
    modules=$((modules + 1))
    for named in $(grep -oE 'from "\./[A-Za-z0-9_-]+\.js"' "$work/module" | sed -e 's/^from "\.\///' -e 's/"$//'); do
      queue="${queue:+$queue }$named"
    done
  done
done
[ "$modules" -ge 12 ] || fail "the face is made of $modules modules; it was not walked"
echo "  ok    the shell, the widgets and the theme are served, every one of the $modules modules the face loads among them"

echo "prove-unit: who may ask"
info=$(request access.controller info '{}')
[ "$(ask request "$info")" = 401 ] || fail "a caller who presents nothing is answered: $(cat "$work/answer")"
# Nor is a bearer made for one for the asking, which evo by itself does.
mint='{"name":"for nobody","reason":"asked for with no bearer","scopes":[{"kind":"step_up","scope":"network_admin"}]}'
[ "$(ask create_bearer_token "$mint")" = 401 ] || fail "a caller who presents nothing was made a bearer: $(cat "$work/answer")"
[ "$(ask list_bearer_tokens)" = 401 ] || fail "a caller who presents nothing is told of the unit's bearers: $(cat "$work/answer")"
[ "$(ask describe_ui_stockings)" = 401 ] || fail "a caller who presents nothing is told what is stocked: $(cat "$work/answer")"
echo "  ok    a caller who presents nothing is refused, and no bearer is made for one"

echo "prove-unit: setting up and signing in"
[ "$(usher state)" = 200 ] || fail "the unit does not say what state it is in: $(cat "$work/answer")"
jq -e '.set_up == false and .level == "standard" and .least_password == 12 and .keeps == true' "$work/answer" > /dev/null || fail "a new unit says of itself: $(cat "$work/answer")"
jq -e '.site == {name: "", logo: null}' "$work/answer" > /dev/null || fail "a new unit says of its site: $(cat "$work/answer")"
[ ! -e "$CODE" ] || fail "a unit of the ordinary level made a code, which it asks nobody for"
echo "  ok    a new unit says that nobody has set it up, and at the ordinary level it makes no code"
[ "$(usher sign_in "$(named "$NAME" "$PASSWORD")")" = 409 ] || fail "a unit nobody set up signs someone in: $(cat "$work/answer")"
[ "$(usher set_up "$(named "$NAME" "owls")")" = 400 ] || fail "a unit is set up with a password of four characters: $(cat "$work/answer")"
[ "$(usher set_up "$(named "$NAME" "$PASSWORD")")" = 200 ] || fail "whoever opens a new unit first does not set it up: $(cat "$work/answer")"
bearer=$(jq -r '.token' "$work/answer")
[ "$(usher set_up "$(named "Mallory" "$PASSWORD")")" = 409 ] || fail "a unit is set up a second time: $(cat "$work/answer")"
[ "$(usher me '' "$bearer")" = 200 ] || fail "whoever set the unit up is not signed in: $(cat "$work/answer")"
jq -e --arg name "$NAME" '.name == $name and .role == "administrator"' "$work/answer" > /dev/null || fail "the unit says of who set it up: $(cat "$work/answer")"
echo "  ok    whoever opens it first sets it up with a name and a password, is its administrator and is signed in; nobody sets it up a second time"
[ "$(stat -c '%U %a' "$PEOPLE")" = "evo 600" ] || fail "the people are kept as $(stat -c '%U %a' "$PEOPLE"), not for the steward alone"
! grep -q -F "$PASSWORD" "$PEOPLE" || fail "the password stands in the file of the people as it was typed"
grep -q -F '$argon2id$' "$PEOPLE" || fail "the password is not kept as an Argon2id hash"
echo "  ok    the people are kept for the steward alone, a password as a hash of it"
[ "$(usher sign_in "$(named "$NAME" "three small owls sang")")" = 401 ] || fail "a wrong password signs in: $(cat "$work/answer")"
[ "$(usher sign_in "$(named "Nobody" "$PASSWORD")")" = 401 ] || fail "a name nobody has signs in: $(cat "$work/answer")"
[ "$(usher sign_in "$(named "anna" "$PASSWORD")")" = 200 ] || fail "the name and the password do not sign in: $(cat "$work/answer")"
second=$(jq -r '.token' "$work/answer")
[ "$(usher people '' "$second")" = 200 ] || fail "who has signed in is not told of the people: $(cat "$work/answer")"
[ "$(usher sign_out '{}' "$second")" = 200 ] || fail "signing out is refused: $(cat "$work/answer")"
[ "$(usher people '' "$second")" = 401 ] || fail "a session is answered after it was signed out of: $(cat "$work/answer")"
[ "$(ask describe_ui_stockings '' "$second")" = 401 ] || fail "the steward answers a session that was signed out of: $(cat "$work/answer")"
echo "  ok    the name and the password sign in, a wrong one does not, and a session is refused once it is signed out of"
[ "$(ask pair_complete '{"pair_id":"bootstrap","code":"000000000000"}' "")" = 401 ] || fail "evo's pairing is open beside signing in: $(cat "$work/answer")"
[ "$(ask create_bearer_token "$mint" "$bearer")" != 200 ] || fail "who has signed in has a bearer of evo's made: $(cat "$work/answer")"
[ "$(ask request "$info" "$bearer")" = 200 ] || fail "who has signed in is refused: $(cat "$work/answer")"
[ "$(answered)" = '{"set":false}' ] || fail "the controller plugin answered $(answered)"
[ "$(ask describe_ui_stockings '' "$bearer")" = 200 ] || fail "the steward does not tell who has signed in what is stocked: $(cat "$work/answer")"
for kind in usher.counter usher.types usher.controller usher.vouchers usher.networks usher.reserve usher.site; do
  grep -q "\"$kind\"" "$work/answer" || fail "nothing is drawn as $kind: $(cat "$work/answer")"
done
echo "  ok    who has signed in is answered, and every widget kind has its place; there is no other way in"

echo "prove-unit: the voucher types"
# Ask the counter as a browser someone is signed in at does.
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

echo "prove-unit: the site"
# Ask the site's shelf as a browser someone is signed in at does.
site() { ask request "$(request site.settings "$1" "$2")" "$bearer"; }
[ "$(site get '{}')" = 200 ] || fail "the site is not told: $(cat "$work/answer")"
answered | jq -e '.name == "" and .time_zone == null and .logo == null and .set_here == false' > /dev/null || fail "a new unit has a site nobody set: $(answered)"
# A unit is given its site's name and time zone where it is started, as
# its service file says how: in a drop-in.
service=$(systemctl show -p FragmentPath --value evo.service)
grep -q '^#Environment="USHER_SITE_NAME=Parish Hall"$' "$service" || fail "the service file does not say how a unit is given its site's name"
grep -q '^#Environment=USHER_TIME_ZONE=Europe/Warsaw$' "$service" || fail "the service file does not say how a unit is given its site's time zone"
mkdir -p "$(dirname "$GIVEN")"
printf '[Service]\nEnvironment="USHER_SITE_NAME=Parish Hall"\nEnvironment=USHER_TIME_ZONE=Europe/Warsaw\n' > "$GIVEN"
systemctl daemon-reload
systemctl restart evo.service
up
[ "$(site get '{}')" = 200 ] || fail "the site is not told: $(cat "$work/answer")"
answered | jq -e '.name == "Parish Hall" and .time_zone == "Europe/Warsaw" and .set_here == false' > /dev/null || fail "the unit does not go by the name and the time zone it was started with: $(answered)"
# Anyone is told the name, for the form a person signs in by to wear,
# and nothing else of the site.
[ "$(usher state)" = 200 ] || fail "the unit does not say what state it is in: $(cat "$work/answer")"
jq -e '.site == {name: "Parish Hall", logo: null}' "$work/answer" > /dev/null || fail "before anyone has signed in, the unit says of its site: $(cat "$work/answer")"
echo "  ok    given a name and a time zone where it is started, the unit goes by them, and anyone is told the name"
[ "$(ask request "$(request site.settings set '{"name":"Mallory","time_zone":null}')" "")" = 401 ] || fail "a caller who has not signed in sets the site: $(cat "$work/answer")"
[ "$(site set '{"name":"Hall of St Mary","time_zone":"London time"}')" != 200 ] || fail "a time zone that is none is taken: $(cat "$work/answer")"
[ "$(site set '{"name":" Hall of St Mary ","time_zone":"Europe/London"}')" = 200 ] || fail "the site's name and time zone are not set: $(cat "$work/answer")"
answered | jq -e '.name == "Hall of St Mary" and .time_zone == "Europe/London"' > /dev/null || fail "the site was set as: $(answered)"
[ "$(stat -c '%U %a' "$SITE" 2> /dev/null)" = "evo 600" ] || fail "the site is kept as $(stat -c '%U %a' "$SITE" 2>&1), not in $SITE for the steward alone"
[ "$(usher state)" = 200 ] || fail "the unit does not say what state it is in: $(cat "$work/answer")"
jq -e '.site == {name: "Hall of St Mary", logo: null}' "$work/answer" > /dev/null || fail "its site set, the unit says of it: $(cat "$work/answer")"
echo "  ok    its administrator sets another name and time zone, kept for the steward alone; nobody who has not signed in sets them"

echo "prove-unit: what is kept"
systemctl restart evo.service
up
[ "$(systemctl show -p NRestarts --value evo.service)" = 0 ] || fail "the service was started again by itself"
[ "$(usher me '' "$bearer")" = 200 ] || fail "who was signed in is signed out by a restart: $(cat "$work/answer")"
[ "$(ask request "$(request issue.counter types '{}')" "$bearer")" = 200 ] || fail "who was signed in is refused after a restart: $(cat "$work/answer")"
answered | jq -e '(.types | map(.name)) == ["Stay", "Day"] and .default == "Day" and .disabled == ["Stay"]' > /dev/null || fail "the voucher types, their order, the default and what is disabled were not kept through a restart: $(answered)"
[ "$(usher people '' "$second")" = 401 ] || fail "a session that was signed out of is answered after a restart: $(cat "$work/answer")"
[ "$(site get '{}')" = 200 ] || fail "the site is not told after a restart: $(cat "$work/answer")"
# What was set of the site stands, though the unit is started with the
# first name and time zone as before.
answered | jq -e '.name == "Hall of St Mary" and .time_zone == "Europe/London" and .set_here == true' > /dev/null || fail "the site's name and time zone were not kept through a restart: $(answered)"
[ "$(usher state)" = 200 ] || fail "the unit does not say what state it is in: $(cat "$work/answer")"
jq -e '.site.name == "Hall of St Mary"' "$work/answer" > /dev/null || fail "started again, the unit says of its site: $(cat "$work/answer")"
rm "$GIVEN"
rmdir "$(dirname "$GIVEN")"
systemctl daemon-reload
echo "  ok    started again, who was signed in still is, who signed out is not, and the voucher types and the site are kept as they were set"

echo "prove-unit: the extended level"
# Whoever has the unit's files takes the people away, and the unit asks
# to be set up again. It is started at the extended level for that, as
# its service file says how.
grep -q '^#Environment=USHER_SECURITY=extended$' "$(systemctl show -p FragmentPath --value evo.service)" || fail "the service file does not say how the extended level is chosen"
systemctl stop evo.service
rm "$PEOPLE"
mkdir -p "$(dirname "$LEVEL")"
printf '[Service]\nEnvironment=USHER_SECURITY=extended\n' > "$LEVEL"
systemctl daemon-reload
systemctl start evo.service
up
[ "$(usher state)" = 200 ] || fail "the unit does not say what state it is in: $(cat "$work/answer")"
jq -e '.set_up == false and .level == "extended" and .least_password == 16 and .keeps == false' "$work/answer" > /dev/null || fail "a unit of the extended level whose people were taken away says of itself: $(cat "$work/answer")"
# The form it is set up by wears the site's name still.
jq -e '.site.name == "Hall of St Mary"' "$work/answer" > /dev/null || fail "its people taken away, the unit says of its site: $(cat "$work/answer")"
[ "$(usher me '' "$bearer")" = 401 ] || fail "a session outlives the people it was a person's among: $(cat "$work/answer")"
echo "  ok    its people taken away, the unit asks to be set up again, and nobody who was signed in still is"
[ "$(stat -c '%U %a' "$CODE")" = "evo 600" ] || fail "the unit's code is $(stat -c '%U %a' "$CODE"), not the steward's alone"
code=$(cat "$CODE")
[[ "$code" =~ ^[0-9a-f]{12}$ ]] || fail "the unit's code is not twelve hexadecimal digits"
with_code() {
  jq -cn --arg name "$NAME" --arg password "$1" --arg code "$2" '{name: $name, password: $password, code: $code}'
}
[ "$(usher set_up "$(named "$NAME" "$PASSWORD")")" = 403 ] || fail "a unit of the extended level is set up without its code: $(cat "$work/answer")"
[ "$(usher set_up "$(with_code "$PASSWORD" "000000000000")")" = 403 ] || fail "a unit of the extended level is set up with a wrong code: $(cat "$work/answer")"
[ "$(usher set_up "$(with_code "fifteen letters" "$code")")" = 400 ] || fail "the extended level takes a password of fifteen characters: $(cat "$work/answer")"
[ "$(usher set_up "$(with_code "$PASSWORD" "$code")")" = 200 ] || fail "the code read on the unit does not set it up: $(cat "$work/answer")"
bearer=$(jq -r '.token' "$work/answer")
[ "$(ask request "$(request issue.counter types '{}')" "$bearer")" = 200 ] || fail "who set the unit up anew is refused: $(cat "$work/answer")"
answered | jq -e '(.types | map(.name)) == ["Stay", "Day"]' > /dev/null || fail "what the unit kept went with its people: $(answered)"
echo "  ok    at the extended level the unit makes a code for the steward alone, and is set up with it and a longer password; what it kept is as it was"
rm "$LEVEL"
rmdir "$(dirname "$LEVEL")"
systemctl daemon-reload
systemctl restart evo.service
up
[ "$(usher state)" = 200 ] || fail "the unit does not say what state it is in: $(cat "$work/answer")"
jq -e '.set_up == true and .level == "standard"' "$work/answer" > /dev/null || fail "back at the ordinary level the unit says of itself: $(cat "$work/answer")"
[ ! -e "$CODE" ] || fail "back at the ordinary level the unit made a code"
[ "$(usher me '' "$bearer")" = 200 ] || fail "who was signed in is signed out by a change of level: $(cat "$work/answer")"
echo "  ok    back at the ordinary level, its people are who they were"

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
echo "  ok    stopped cleanly, and its socket is gone"
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
