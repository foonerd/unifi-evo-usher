#!/bin/bash
# Proof that the steward's package makes a machine a unit: installed on
# this machine, started by systemd under every protection of its service,
# set up and signed in to as a browser does it, asked as a browser asks,
# by its administrator and by a person at its front desk, who is refused
# what is the administrator's; a screen adopted as a satellite, refused
# everything but its own, disabled and refused, then protected by a PIN
# and given voucher types and a logo of its own; given its site's name
# and time zone where it is started, started again, its administrator
# given a new password on the unit itself, run at the extended level,
# removed, and, where the package is Debian's, purged.
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
for tool in curl jq base64 runuser $manager; do
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
SATELLITES=/var/lib/evo/state/usher/satellites.json
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

# Start the service, or start it again, and wait until its face answers.
# The proof starts it more often within ten seconds than systemd lets a
# service start, which is five times, so the count is cleared first. A
# unit in service is not started that often, and needs no such thing.
start() {
  systemctl reset-failed evo.service 2> /dev/null || true
  systemctl "$1" evo.service
  up
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
# The licences go wherever the program goes: the usher's own, evo's, and
# the notices of the libraries of others the program contains.
case "$format" in
  deb) licences=/usr/share/doc/evo-usher-steward; own=copyright ;;
  rpm) licences=/usr/share/licenses/evo-usher-steward; own=LICENSE ;;
esac
for file in "$own" LICENSE-evo.txt THIRD-PARTY-NOTICES.txt; do
  [ -s "$licences/$file" ] || fail "$licences/$file is not on the machine"
done
grep -q '^Apache License 2.0 (Apache-2.0)$' "$licences/THIRD-PARTY-NOTICES.txt" || fail "the notices of the libraries name no licence"
echo "  ok    the licences are on the machine: the usher's, evo's, and the notices of $(grep -c -E '^  [A-Za-z0-9_-]+ [0-9]' "$licences/THIRD-PARTY-NOTICES.txt") libraries the program contains"
# Every screen joined to the unit holds a connection, and a connection is
# an open file: the service is given as many as a large venue takes.
files=$(awk '/^Max open files/ { print $4 }' "/proc/$pid/limits")
[ "$files" = 65536 ] || fail "the steward may have $files files open, not 65536"
echo "  ok    as many open files as a large venue takes: $files"

echo "prove-unit: the face"
curl --silent --cacert "$AUTHORITY" --max-time 10 "$FACE/" | grep -q 'door.js' || fail "the door is not served"
[ "$(ask describe_active_ui_selection)" = 200 ] || fail "the door is not told which shell is active: $(cat "$work/answer")"
echo "  ok    the door is served, and is told which shell to open"
for part in org.evoframework.usher.shell/index.html org.evoframework.usher.shell/shell.css org.evoframework.usher.theme.frost/theme.css org.evoframework.usher.shell/lang/languages.json; do
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
# The words the face says: a file of texts for every language it lists,
# English the first of them.
spoken=$(curl --silent --cacert "$AUTHORITY" --max-time 10 "$FACE/org.evoframework.usher.shell/lang/languages.json" | jq -r 'map(.tag) | join(" ")')
[ "${spoken%% *}" = en ] || fail "the face's languages are $spoken, and English is not the first"
for language in $spoken; do
  curl --silent --cacert "$AUTHORITY" --max-time 10 "$FACE/org.evoframework.usher.shell/lang/$language.json" | jq -e '."common.save" | type == "string"' > /dev/null || fail "the texts of $language are not served"
done
echo "  ok    the shell, the widgets and the theme are served, every one of the $modules modules the face loads among them, and the texts of every language it speaks: $spoken"

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
jq -e '.site == {name: "", logo: null, language: "en", follows_browser: false}' "$work/answer" > /dev/null || fail "a new unit says of its site: $(cat "$work/answer")"
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

echo "prove-unit: the front desk"
# A person for the front desk is added by the administrator. Nobody
# chooses their PIN: the usher makes it, and says it this once.
[ "$(usher add_person '{"name":"Ben","role":"front_desk"}' "$bearer")" = 200 ] || fail "a person is not added for the front desk: $(cat "$work/answer")"
pin=$(jq -r '.pin' "$work/answer")
[[ "$pin" =~ ^[0-9]{6}$ ]] || fail "the PIN the usher made is not six digits: $(cat "$work/answer")"
! grep -q -F "$pin" "$PEOPLE" || fail "the PIN stands in the file of the people as it was made"
[ "$(usher add_person '{"name":"Cara","role":"front_desk","password":"three small owls sing"}' "$bearer")" = 400 ] || fail "a PIN is chosen for a person at the front desk: $(cat "$work/answer")"
[ "$(usher sign_in "$(named "Ben" "$pin")")" = 200 ] || fail "the name and the PIN do not sign in: $(cat "$work/answer")"
jq -e '.name == "Ben" and .role == "front_desk"' "$work/answer" > /dev/null || fail "the unit says of who signed in with a PIN: $(cat "$work/answer")"
desk=$(jq -r '.token' "$work/answer")
# Ask a shelf as the browser of a person at the front desk does.
at_the_desk() { ask request "$(request "$1" "$2" "${3:-{\}}")" "$desk"; }
# What the counter's page asks is answered: who they are, what is
# stocked, the site, how many vouchers are set aside, the types on offer.
[ "$(usher me '' "$desk")" = 200 ] || fail "a person at the front desk is not told who they are: $(cat "$work/answer")"
[ "$(ask describe_ui_stockings '' "$desk")" = 200 ] || fail "a person at the front desk is not told what is stocked: $(cat "$work/answer")"
[ "$(at_the_desk site.settings get)" = 200 ] || fail "a person at the front desk is not told of the site: $(cat "$work/answer")"
[ "$(at_the_desk stock.reserve count)" = 200 ] || fail "a person at the front desk is not told how many vouchers are set aside: $(cat "$work/answer")"
[ "$(at_the_desk issue.counter offered)" = 200 ] || fail "a person at the front desk is not told the types on offer: $(cat "$work/answer")"
answered | jq -e '(.types | map(.name)) == ["Day"]' > /dev/null || fail "a person at the front desk is offered: $(answered)"
# A voucher is theirs to hand out: the counter is asked, and says that
# no network is chosen yet, which is the administrator's to do.
[ "$(at_the_desk issue.counter hand '{"type":"Day"}')" != 403 ] || fail "a person at the front desk is kept from handing a voucher out: $(cat "$work/answer")"
jq -e '.error.details.subclass == "no_network_chosen"' "$work/answer" > /dev/null || fail "the counter answered a person at the front desk: $(cat "$work/answer")"
# Everything else is an administrator's, and is refused by name: of the
# shelves, of the usher's own routes, and of the steward's operations.
while read -r shelf verb; do
  [ "$(at_the_desk "$shelf" "$verb")" = 403 ] || fail "a person at the front desk is answered $shelf $verb: $(cat "$work/answer")"
  jq -e '.error.details.subclass == "not_for_front_desk"' "$work/answer" > /dev/null || fail "$shelf $verb is refused a person at the front desk as: $(cat "$work/answer")"
done << 'ASKED'
issue.counter types
issue.counter put_type
issue.counter handed
access.controller info
access.controller set
networks.directory networks
stock.reserve held
stock.reserve take
site.settings set
ASKED
[ "$(usher people '' "$desk")" = 403 ] || fail "a person at the front desk is told of the people: $(cat "$work/answer")"
[ "$(usher add_person "$(named "Mallory" "three small owls sing")" "$desk")" = 403 ] || fail "a person at the front desk adds a person: $(cat "$work/answer")"
[ "$(usher make_pin '{"name":"Ben"}' "$desk")" = 403 ] || fail "a person at the front desk makes a PIN: $(cat "$work/answer")"
[ "$(ask list_bearer_tokens '' "$desk")" = 403 ] || fail "a person at the front desk is told of the unit's bearers: $(cat "$work/answer")"
[ "$(ask create_bearer_token "$mint" "$desk")" = 403 ] || fail "a person at the front desk has a bearer made: $(cat "$work/answer")"
# A new PIN, which only an administrator has made, ends their session.
[ "$(usher make_pin '{"name":"Ben"}' "$bearer")" = 200 ] || fail "a new PIN is not made: $(cat "$work/answer")"
pin=$(jq -r '.pin' "$work/answer")
[ "$(usher me '' "$desk")" = 401 ] || fail "a session is answered after a new PIN was made: $(cat "$work/answer")"
# Where the administrator would sooner choose the PIN, they set one: it
# is held to what keeps it from being guessed, is not answered back, and
# is the person's from then on.
[ "$(usher set_pin '{"name":"Ben","pin":"123456"}' "$bearer")" = 400 ] || fail "a PIN that counts up is set: $(cat "$work/answer")"
jq -e '.error.details.subclass == "pin_runs"' "$work/answer" > /dev/null || fail "a PIN that counts up is refused as: $(cat "$work/answer")"
[ "$(usher set_pin '{"name":"Ben","pin":"48027"}' "$bearer")" = 400 ] || fail "a PIN of five digits is set: $(cat "$work/answer")"
jq -e '.error.details.subclass == "pin_too_short" and .error.details.values == {least: 6, has: 5}' "$work/answer" > /dev/null || fail "a PIN of five digits is refused as: $(cat "$work/answer")"
[ "$(usher sign_in "$(named "Ben" "$pin")")" = 200 ] || fail "a PIN that was refused took the place of the one before: $(cat "$work/answer")"
desk=$(jq -r '.token' "$work/answer")
chosen=480273
[ "$chosen" != "$pin" ] || chosen=480274
[ "$(usher set_pin "{\"name\":\"Ben\",\"pin\":\"$chosen\"}" "$desk")" = 403 ] || fail "a person at the front desk sets a PIN: $(cat "$work/answer")"
[ "$(usher set_pin "{\"name\":\"Ben\",\"pin\":\"$chosen\"}" "$bearer")" = 200 ] || fail "a PIN the administrator chose is not set: $(cat "$work/answer")"
! grep -q -F "$chosen" "$work/answer" || fail "the PIN that was set is answered back: $(cat "$work/answer")"
! grep -q -F "$chosen" "$PEOPLE" || fail "the PIN that was set stands in the file of the people as it was typed"
[ "$(usher me '' "$desk")" = 401 ] || fail "a session is answered after a PIN was set: $(cat "$work/answer")"
[ "$(usher sign_in "$(named "Ben" "$pin")")" = 401 ] || fail "the PIN before signs in after another was set: $(cat "$work/answer")"
pin=$chosen
[ "$(usher sign_in "$(named "Ben" "$pin")")" = 200 ] || fail "the name and the PIN that was set do not sign in: $(cat "$work/answer")"
desk=$(jq -r '.token' "$work/answer")
echo "  ok    a person at the front desk signs in with a PIN the usher made, is answered what the counter's page asks, and is refused the rest by name; a new PIN, made by the usher or set by the administrator, ends their session"

echo "prove-unit: a satellite"
# The page of a satellite is served to anyone, beside the shell: a
# screen that nobody is signed in at opens it.
for part in satellite/ satellite/satellite.js satellite/satellite.css; do
  [ "$(curl --silent --cacert "$AUTHORITY" --max-time 10 --output /dev/null --write-out '%{http_code}' "$FACE/$part")" = 200 ] || fail "$part is not served"
done
# Such a screen asks for a code, and is given a secret beside it, which
# is all it keeps. Until it is adopted the secret is good for nothing
# but being told that the screen still waits.
[ "$(usher satellite_code '{}')" = 200 ] || fail "a screen is not given a code: $(cat "$work/answer")"
code=$(jq -r '.code' "$work/answer")
secret=$(jq -r '.secret' "$work/answer")
[[ "$code" =~ ^[0-9]{6}$ ]] || fail "the code of a screen is not six digits: $code"
[[ "$secret" =~ ^[0-9a-f]{64}$ ]] || fail "the secret of a screen is not 256 bits, written in hex"
presenting=$(jq -cn --arg secret "$secret" '{secret: $secret}')
[ "$(usher satellite_sign_in "$presenting")" = 200 ] || fail "a screen that waits is not told so: $(cat "$work/answer")"
jq -e --arg code "$code" '.adopted == false and .code == $code and (has("token") | not)' "$work/answer" > /dev/null || fail "a screen nobody adopted is answered: $(cat "$work/answer")"
# It is an administrator who adopts it, by the code it shows, and
# nobody else.
adopting=$(jq -cn --arg code "$code" '{code: $code, name: "Lobby", place: "By the door"}')
[ "$(usher adopt_satellite "$adopting")" = 401 ] || fail "a caller who presents nothing adopts a screen: $(cat "$work/answer")"
[ "$(usher adopt_satellite "$adopting" "$desk")" = 403 ] || fail "a person at the front desk adopts a screen: $(cat "$work/answer")"
[ "$(usher satellites '' "$desk")" = 403 ] || fail "a person at the front desk is told of the satellites: $(cat "$work/answer")"
other=$(printf '%06d' $(((10#$code + 1) % 1000000)))
[ "$(usher adopt_satellite "$(jq -cn --arg code "$other" '{code: $code, name: "Lobby"}')" "$bearer")" = 400 ] || fail "a screen is adopted by a code no screen shows: $(cat "$work/answer")"
jq -e '.error.details.subclass == "satellite_no_such_code"' "$work/answer" > /dev/null || fail "a code no screen shows is refused as: $(cat "$work/answer")"
[ "$(usher adopt_satellite "$adopting" "$bearer")" = 200 ] || fail "a screen is not adopted by its code: $(cat "$work/answer")"
jq -e '.adopted.name == "Lobby" and .adopted.place == "By the door" and .adopted.enabled == true and .adopted.limit == 20' "$work/answer" > /dev/null || fail "of the screen it adopted the unit says: $(cat "$work/answer")"
satellite=$(jq -r '.adopted.id' "$work/answer")
[ "$(usher adopt_satellite "$adopting" "$bearer")" = 400 ] || fail "a code adopts a second time: $(cat "$work/answer")"
# What the unit keeps of it is the SHA-256 of its secret, for itself
# alone, and never the secret.
[ "$(stat -c '%U %a' "$SATELLITES")" = "evo 600" ] || fail "the satellites are kept as $(stat -c '%U %a' "$SATELLITES"), not for the steward alone"
! grep -q -F "$secret" "$SATELLITES" || fail "the secret of a satellite stands in the file of the satellites"
grep -q -F "$(printf '%s' "$secret" | sha256sum | cut -d ' ' -f 1)" "$SATELLITES" || fail "the file of the satellites does not hold the SHA-256 of the secret"
# Adopted, the screen exchanges its secret for a session: an ordinary
# one of the usher's, good for twelve hours.
[ "$(usher satellite_sign_in "$presenting")" = 200 ] || fail "a satellite's secret is not exchanged for a session: $(cat "$work/answer")"
jq -e '.adopted == true and .name == "Lobby"' "$work/answer" > /dev/null || fail "a satellite that presents its secret is answered: $(cat "$work/answer")"
screen=$(jq -r '.token' "$work/answer")
# Ask a shelf as a satellite's screen does.
at_the_screen() { ask request "$(request "$1" "$2" "${3:-{\}}")" "$screen"; }
# What its page asks is answered: that it is still a satellite, and
# what the site is called; the site's settings; and the default
# voucher, of which the counter says that no network is chosen yet,
# which is the administrator's to do.
[ "$(usher satellite '' "$screen")" = 200 ] || fail "a satellite is not told that it is one: $(cat "$work/answer")"
jq -e '.name == "Lobby" and .place == "By the door" and .limit == 20 and .handed == 0 and (.site.name | type == "string")' "$work/answer" > /dev/null || fail "a satellite that says it is there is answered: $(cat "$work/answer")"
[ "$(at_the_screen site.settings get)" = 200 ] || fail "a satellite is not told of the site: $(cat "$work/answer")"
[ "$(at_the_screen issue.counter hand_default)" != 403 ] || fail "a satellite is kept from handing out the default voucher: $(cat "$work/answer")"
jq -e '.error.details.subclass == "no_network_chosen"' "$work/answer" > /dev/null || fail "the counter answered a satellite: $(cat "$work/answer")"
# A voucher that was not handed out is not counted against its hour.
[ "$(usher satellite '' "$screen")" = 200 ] || fail "a satellite is not told that it is one: $(cat "$work/answer")"
jq -e '.handed == 0' "$work/answer" > /dev/null || fail "a voucher that was refused is counted against a satellite's hour: $(cat "$work/answer")"
# Everything else is refused it by name: of the shelves, the front
# desk's own verbs among them, of the usher's routes, and of the
# steward's operations.
while read -r shelf verb; do
  [ "$(at_the_screen "$shelf" "$verb")" = 403 ] || fail "a satellite is answered $shelf $verb: $(cat "$work/answer")"
  jq -e '.error.details.subclass == "not_for_satellite"' "$work/answer" > /dev/null || fail "$shelf $verb is refused a satellite as: $(cat "$work/answer")"
done << 'ASKED'
issue.counter hand
issue.counter offered
issue.counter types
issue.counter put_type
issue.counter handed
access.controller info
access.controller set
networks.directory networks
stock.reserve count
stock.reserve held
stock.reserve take
site.settings set
ASKED
for route in me people satellites; do
  [ "$(usher "$route" '' "$screen")" = 403 ] || fail "a satellite is answered $route: $(cat "$work/answer")"
done
[ "$(usher add_person "$(named "Mallory" "three small owls sing")" "$screen")" = 403 ] || fail "a satellite adds a person: $(cat "$work/answer")"
[ "$(usher adopt_satellite "$adopting" "$screen")" = 403 ] || fail "a satellite adopts a screen: $(cat "$work/answer")"
[ "$(usher set_satellite "$(jq -cn --arg id "$satellite" '{id: $id, limit: 1000}')" "$screen")" = 403 ] || fail "a satellite sets its own limit: $(cat "$work/answer")"
[ "$(ask describe_ui_stockings '' "$screen")" = 403 ] || fail "a satellite is told what is stocked: $(cat "$work/answer")"
[ "$(ask list_bearer_tokens '' "$screen")" = 403 ] || fail "a satellite is told of the unit's bearers: $(cat "$work/answer")"
[ "$(ask create_bearer_token "$mint" "$screen")" = 403 ] || fail "a satellite has a bearer made: $(cat "$work/answer")"
# Nobody but a satellite says that it is one.
[ "$(usher satellite '' "$bearer")" = 403 ] || fail "an administrator is answered as a satellite: $(cat "$work/answer")"
[ "$(usher satellite '')" = 401 ] || fail "a caller who presents nothing is answered as a satellite: $(cat "$work/answer")"
# The administrator sees it among the satellites, seen a moment ago,
# and holds it to fewer vouchers an hour.
[ "$(usher satellites '' "$bearer")" = 200 ] || fail "the satellites are not listed: $(cat "$work/answer")"
jq -e --arg id "$satellite" '(.satellites | length) == 1 and .satellites[0].id == $id and .satellites[0].name == "Lobby" and .satellites[0].enabled == true and (.satellites[0].last_seen_ms | type == "number")' "$work/answer" > /dev/null || fail "of its satellites the unit says: $(cat "$work/answer")"
! grep -q -F "$secret" "$work/answer" || fail "the secret of a satellite is told to an administrator"
[ "$(usher set_satellite "$(jq -cn --arg id "$satellite" '{id: $id, limit: 5}')" "$bearer")" = 200 ] || fail "a satellite's limit is not set: $(cat "$work/answer")"
jq -e '.set.limit == 5' "$work/answer" > /dev/null || fail "a satellite's limit set, the unit says: $(cat "$work/answer")"
[ "$(usher set_satellite "$(jq -cn --arg id "$satellite" '{id: $id, limit: 0}')" "$bearer")" = 400 ] || fail "a satellite is held to no voucher an hour: $(cat "$work/answer")"
# Disabled, it is signed out at once, and its secret is refused until
# it is enabled again.
[ "$(usher set_satellite "$(jq -cn --arg id "$satellite" '{id: $id, enabled: false}')" "$bearer")" = 200 ] || fail "a satellite is not disabled: $(cat "$work/answer")"
[ "$(usher satellite '' "$screen")" = 401 ] || fail "a satellite that was disabled is answered: $(cat "$work/answer")"
[ "$(at_the_screen issue.counter hand_default)" = 401 ] || fail "a satellite that was disabled hands out a voucher: $(cat "$work/answer")"
[ "$(usher satellite_sign_in "$presenting")" = 403 ] || fail "the secret of a satellite that was disabled is exchanged: $(cat "$work/answer")"
jq -e '.error.details.subclass == "satellite_disabled"' "$work/answer" > /dev/null || fail "a satellite that was disabled is refused as: $(cat "$work/answer")"
[ "$(usher set_satellite "$(jq -cn --arg id "$satellite" '{id: $id, enabled: true}')" "$bearer")" = 200 ] || fail "a satellite is not enabled again: $(cat "$work/answer")"
[ "$(usher satellite_sign_in "$presenting")" = 200 ] || fail "a satellite that was enabled again is not given a session: $(cat "$work/answer")"
screen=$(jq -r '.token' "$work/answer")
[ "$(usher satellite '' "$screen")" = 200 ] || fail "a satellite that was enabled again is not answered: $(cat "$work/answer")"
echo "  ok    a screen is adopted by the code it shows, hands out the default voucher and nothing else, and is refused the rest by name; disabled, it is signed out and refused until it is enabled again"

echo "prove-unit: a protected satellite"
# What the administrator changes of a satellite, by what the usher knows
# it by.
change() { usher set_satellite "$(jq -cn --arg id "$satellite" --argjson change "$1" '{id: $id} + $change')" "$bearer"; }
# Protected, a satellite gives a voucher for its PIN, typed on the
# screen for each one. The PIN is held to what a typed PIN is held to,
# and is kept as a hash and never as itself. The screen learns that it
# is protected on the session it has.
[ "$(change '{"mode": "protected"}')" = 400 ] || fail "a satellite is protected with no PIN: $(cat "$work/answer")"
jq -e '.error.details.subclass == "satellite_pin_needed"' "$work/answer" > /dev/null || fail "a satellite protected with no PIN is refused as: $(cat "$work/answer")"
[ "$(change '{"mode": "protected", "pin": "123456"}')" = 400 ] || fail "a PIN that counts up is a satellite's: $(cat "$work/answer")"
jq -e '.error.details.subclass == "pin_runs"' "$work/answer" > /dev/null || fail "a PIN that counts up is refused a satellite as: $(cat "$work/answer")"
[ "$(change '{"mode": "protected", "pin": "480273"}')" = 200 ] || fail "a satellite is not protected: $(cat "$work/answer")"
jq -e '.set.mode == "protected"' "$work/answer" > /dev/null || fail "of a satellite it protected the unit says: $(cat "$work/answer")"
! grep -q -F 480273 "$work/answer" || fail "the PIN of a satellite is answered back"
! grep -q -F 480273 "$SATELLITES" || fail "the PIN of a satellite stands in the file of the satellites as it was typed"
grep -q -F '$argon2id$' "$SATELLITES" || fail "the PIN of a satellite is not kept as an Argon2id hash"
[ "$(usher satellite '' "$screen")" = 200 ] || fail "a satellite that was protected is signed out: $(cat "$work/answer")"
jq -e '.protected == true and .pin == {least: 6, most: 12}' "$work/answer" > /dev/null || fail "a satellite that was protected is told: $(cat "$work/answer")"
# With no PIN, or with another, no voucher; with its own, the counter
# is asked, and says that no network is chosen yet.
[ "$(at_the_screen issue.counter hand_default)" = 403 ] || fail "a protected satellite hands out a voucher with no PIN: $(cat "$work/answer")"
jq -e '.error.details.subclass == "satellite_pin_required"' "$work/answer" > /dev/null || fail "a protected satellite asked with no PIN is refused as: $(cat "$work/answer")"
[ "$(at_the_screen issue.counter hand_default '{"pin": "111111"}')" = 403 ] || fail "a protected satellite hands out a voucher for a wrong PIN: $(cat "$work/answer")"
jq -e '.error.details.subclass == "satellite_pin_wrong"' "$work/answer" > /dev/null || fail "a wrong PIN is refused a satellite as: $(cat "$work/answer")"
[ "$(at_the_screen issue.counter hand_default '{"pin": "480273"}')" != 403 ] || fail "a protected satellite is kept from the default voucher with its PIN: $(cat "$work/answer")"
jq -e '.error.details.subclass == "no_network_chosen"' "$work/answer" > /dev/null || fail "the counter answered a protected satellite: $(cat "$work/answer")"
# It is given voucher types of its own, which an unlocked one is not.
# Its session is then good for more than it was, so it is ended, and the
# screen signs in again; it is told its own types and no others, is let
# ask for one of them by name, and is refused any other.
[ "$(change '{"types": ["Day"]}')" = 200 ] || fail "a protected satellite is not given a type: $(cat "$work/answer")"
[ "$(usher satellite '' "$screen")" = 401 ] || fail "a satellite that was given types keeps the session it had: $(cat "$work/answer")"
[ "$(usher satellite_sign_in "$presenting")" = 200 ] || fail "a satellite that was given types is not given a session: $(cat "$work/answer")"
jq -e '(.types | map(.name)) == ["Day"] and .protected == true' "$work/answer" > /dev/null || fail "a satellite that was given a type is told: $(cat "$work/answer")"
screen=$(jq -r '.token' "$work/answer")
[ "$(at_the_screen issue.counter hand_type '{"type": "Day", "pin": "480273"}')" != 403 ] || fail "a satellite is kept from a type it was given: $(cat "$work/answer")"
jq -e '.error.details.subclass == "no_network_chosen"' "$work/answer" > /dev/null || fail "the counter answered a satellite that asked for its type: $(cat "$work/answer")"
[ "$(at_the_screen issue.counter hand_type '{"type": "Stay", "pin": "480273"}')" = 403 ] || fail "a satellite is given a type it was not given: $(cat "$work/answer")"
jq -e '.error.details.subclass == "satellite_type_not_given"' "$work/answer" > /dev/null || fail "a type it was not given is refused a satellite as: $(cat "$work/answer")"
[ "$(at_the_screen issue.counter hand_default '{"pin": "480273"}')" = 403 ] || fail "a satellite that was given types gives the default: $(cat "$work/answer")"
[ "$(change '{"mode": "unlocked"}')" = 400 ] || fail "an unlocked satellite keeps types of its own: $(cat "$work/answer")"
jq -e '.error.details.subclass == "satellite_types_unlocked"' "$work/answer" > /dev/null || fail "types on an unlocked satellite are refused as: $(cat "$work/answer")"
# A logo of its own: a PNG, kept as a file beside the satellites', for
# the steward alone. The screen is told a stamp, and fetches the
# picture by it.
logo=iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==
[ "$(change '{"logo_png_b64": "bm8gcGljdHVyZQ=="}')" = 400 ] || fail "what is no PNG is a satellite's logo: $(cat "$work/answer")"
[ "$(change "$(jq -cn --arg logo "$logo" '{logo_png_b64: $logo}')")" = 200 ] || fail "a satellite is not given a logo: $(cat "$work/answer")"
stamp=$(jq -r '.set.logo.stamp' "$work/answer")
kept="$(dirname "$SATELLITES")/satellite-logos/$satellite-$stamp.png"
[ "$(stat -c '%U %a' "$kept" 2> /dev/null)" = "evo 600" ] || fail "the logo of a satellite is kept as $(stat -c '%U %a' "$kept" 2>&1), not in $kept for the steward alone"
[ "$(usher satellite '' "$screen")" = 200 ] || fail "a satellite that was given a logo is signed out: $(cat "$work/answer")"
jq -e --arg stamp "$stamp" '.logo == {stamp: $stamp, own: true}' "$work/answer" > /dev/null || fail "a satellite that was given a logo is told: $(cat "$work/answer")"
[ "$(usher "satellite_logo?stamp=$stamp" '' "$screen")" = 200 ] || fail "a satellite is not given its logo: $(cat "$work/answer")"
printf '%s' "$logo" | base64 -d | cmp -s - "$work/answer" || fail "the logo a satellite fetched is not the file it was given"
[ "$(usher "satellite_logo?id=$satellite" '' "$desk")" = 403 ] || fail "a person at the front desk is given a satellite's logo: $(cat "$work/answer")"
[ "$(usher "satellite_logo?id=$satellite" '')" = 401 ] || fail "a caller who presents nothing is given a satellite's logo: $(cat "$work/answer")"
# Unlocked again, with the default type and the site's logo: as it was.
[ "$(change '{"mode": "unlocked", "types": [], "remove_logo": true}')" = 200 ] || fail "a satellite is not unlocked again: $(cat "$work/answer")"
[ ! -e "$kept" ] || fail "the logo of a satellite stays after it was taken away"
[ "$(usher satellite_sign_in "$presenting")" = 200 ] || fail "a satellite that was unlocked again is not given a session: $(cat "$work/answer")"
jq -e '.protected == false and .types == [] and .logo == null' "$work/answer" > /dev/null || fail "a satellite that was unlocked again is told: $(cat "$work/answer")"
screen=$(jq -r '.token' "$work/answer")
[ "$(at_the_screen issue.counter hand_default)" != 403 ] || fail "a satellite that was unlocked again is kept from the default voucher: $(cat "$work/answer")"
echo "  ok    protected, a satellite gives a voucher for its PIN and for no other; it is given voucher types and a logo of its own, is told its own types and a stamp of its logo, and is refused a type it was not given"

echo "prove-unit: the site"
# Ask the site's shelf as a browser someone is signed in at does.
site() { ask request "$(request site.settings "$1" "$2")" "$bearer"; }
[ "$(site get '{}')" = 200 ] || fail "the site is not told: $(cat "$work/answer")"
answered | jq -e '.name == "" and .time_zone == null and .logo == null and .set_here == false and .language == "en" and .follows_browser == false and .language_set_here == false' > /dev/null || fail "a new unit has a site nobody set: $(answered)"
# A unit is given its site's name, time zone and language where it is
# started, as its service file says how: in a drop-in.
service=$(systemctl show -p FragmentPath --value evo.service)
grep -q '^#Environment="USHER_SITE_NAME=Market Hall"$' "$service" || fail "the service file does not say how a unit is given its site's name"
grep -q '^#Environment=USHER_TIME_ZONE=Europe/Warsaw$' "$service" || fail "the service file does not say how a unit is given its site's time zone"
grep -q '^#Environment=USHER_LANGUAGE=de$' "$service" || fail "the service file does not say how a unit is given its site's language"
mkdir -p "$(dirname "$GIVEN")"
printf '[Service]\nEnvironment="USHER_SITE_NAME=Market Hall"\nEnvironment=USHER_TIME_ZONE=Europe/Warsaw\nEnvironment=USHER_LANGUAGE=de\n' > "$GIVEN"
systemctl daemon-reload
start restart
[ "$(site get '{}')" = 200 ] || fail "the site is not told: $(cat "$work/answer")"
answered | jq -e '.name == "Market Hall" and .time_zone == "Europe/Warsaw" and .set_here == false and .language == "de" and .follows_browser == false and .language_set_here == false' > /dev/null || fail "the unit does not go by the name, the time zone and the language it was started with: $(answered)"
# Anyone is told the name, for the form a person signs in by to wear,
# and the language the form speaks, and nothing else of the site.
[ "$(usher state)" = 200 ] || fail "the unit does not say what state it is in: $(cat "$work/answer")"
jq -e '.site == {name: "Market Hall", logo: null, language: "de", follows_browser: false}' "$work/answer" > /dev/null || fail "before anyone has signed in, the unit says of its site: $(cat "$work/answer")"
echo "  ok    given a name, a time zone and a language where it is started, the unit goes by them, and anyone is told the name and the language"
[ "$(ask request "$(request site.settings set '{"name":"Mallory","time_zone":null}')" "")" = 401 ] || fail "a caller who has not signed in sets the site: $(cat "$work/answer")"
[ "$(site set '{"name":"Hall of the Weavers","time_zone":"London time"}')" != 200 ] || fail "a time zone that is none is taken: $(cat "$work/answer")"
[ "$(site set '{"name":"Hall of the Weavers","language":"German"}')" != 200 ] || fail "a language that is none is taken: $(cat "$work/answer")"
jq -e '.error.details.subclass == "language_not_one"' "$work/answer" > /dev/null || fail "a language that is none is refused as: $(cat "$work/answer")"
[ "$(site set '{"name":" Hall of the Weavers ","time_zone":"Europe/London","language":"en","follows_browser":true}')" = 200 ] || fail "the site's name, time zone and language are not set: $(cat "$work/answer")"
answered | jq -e '.name == "Hall of the Weavers" and .time_zone == "Europe/London" and .language == "en" and .follows_browser == true' > /dev/null || fail "the site was set as: $(answered)"
[ "$(stat -c '%U %a' "$SITE" 2> /dev/null)" = "evo 600" ] || fail "the site is kept as $(stat -c '%U %a' "$SITE" 2>&1), not in $SITE for the steward alone"
[ "$(usher state)" = 200 ] || fail "the unit does not say what state it is in: $(cat "$work/answer")"
jq -e '.site == {name: "Hall of the Weavers", logo: null, language: "en", follows_browser: true}' "$work/answer" > /dev/null || fail "its site set, the unit says of it: $(cat "$work/answer")"
echo "  ok    its administrator sets another name, time zone and language, kept for the steward alone; nobody who has not signed in sets them"

echo "prove-unit: what is kept"
start restart
[ "$(systemctl show -p NRestarts --value evo.service)" = 0 ] || fail "the service was started again by itself"
[ "$(usher me '' "$bearer")" = 200 ] || fail "who was signed in is signed out by a restart: $(cat "$work/answer")"
[ "$(ask request "$(request issue.counter types '{}')" "$bearer")" = 200 ] || fail "who was signed in is refused after a restart: $(cat "$work/answer")"
answered | jq -e '(.types | map(.name)) == ["Stay", "Day"] and .default == "Day" and .disabled == ["Stay"]' > /dev/null || fail "the voucher types, their order, the default and what is disabled were not kept through a restart: $(answered)"
[ "$(usher people '' "$second")" = 401 ] || fail "a session that was signed out of is answered after a restart: $(cat "$work/answer")"
# A person at the front desk is still signed in, as what they are.
[ "$(usher me '' "$desk")" = 200 ] || fail "a person at the front desk is signed out by a restart: $(cat "$work/answer")"
jq -e '.name == "Ben" and .role == "front_desk"' "$work/answer" > /dev/null || fail "started again, the unit says of a person at the front desk: $(cat "$work/answer")"
[ "$(at_the_desk issue.counter offered)" = 200 ] || fail "a person at the front desk is refused the types on offer after a restart: $(cat "$work/answer")"
[ "$(at_the_desk issue.counter types)" = 403 ] || fail "a person at the front desk is answered the voucher types after a restart: $(cat "$work/answer")"
# A satellite is one still, under the name and the limit it was given,
# and is held to its own as before.
[ "$(usher satellite '' "$screen")" = 200 ] || fail "a satellite is signed out by a restart: $(cat "$work/answer")"
jq -e '.name == "Lobby" and .limit == 5' "$work/answer" > /dev/null || fail "started again, the unit says of a satellite: $(cat "$work/answer")"
[ "$(at_the_screen issue.counter hand_default)" != 403 ] || fail "a satellite is kept from the default voucher after a restart: $(cat "$work/answer")"
[ "$(at_the_screen issue.counter offered)" = 403 ] || fail "a satellite is answered the types on offer after a restart: $(cat "$work/answer")"
# Taken away, its session ends at once and its secret is nobody's.
[ "$(usher remove_satellite "$(jq -cn --arg id "$satellite" '{id: $id}')" "$bearer")" = 200 ] || fail "a satellite is not removed: $(cat "$work/answer")"
[ "$(usher satellite '' "$screen")" = 401 ] || fail "a satellite that was removed is answered: $(cat "$work/answer")"
[ "$(usher satellite_sign_in "$presenting")" = 401 ] || fail "the secret of a satellite that was removed is exchanged: $(cat "$work/answer")"
jq -e '.error.details.subclass == "satellite_unknown"' "$work/answer" > /dev/null || fail "a satellite that was removed is refused as: $(cat "$work/answer")"
[ "$(jq -c . "$SATELLITES")" = "[]" ] || fail "a satellite that was removed is kept: $(cat "$SATELLITES")"
[ "$(site get '{}')" = 200 ] || fail "the site is not told after a restart: $(cat "$work/answer")"
# What was set of the site stands, though the unit is started with the
# first name, time zone and language as before.
answered | jq -e '.name == "Hall of the Weavers" and .time_zone == "Europe/London" and .set_here == true and .language == "en" and .follows_browser == true and .language_set_here == true' > /dev/null || fail "the site's name, time zone and language were not kept through a restart: $(answered)"
[ "$(usher state)" = 200 ] || fail "the unit does not say what state it is in: $(cat "$work/answer")"
jq -e '.site.name == "Hall of the Weavers" and .site.language == "en"' "$work/answer" > /dev/null || fail "started again, the unit says of its site: $(cat "$work/answer")"
rm "$GIVEN"
rmdir "$(dirname "$GIVEN")"
systemctl daemon-reload
echo "  ok    started again, who was signed in still is, each as what they are, a satellite among them until it is removed, who signed out is not, and the voucher types and the site are kept as they were set"

echo "prove-unit: an administrator let back in"
# The password is lost, and there is no other administrator. Whoever
# keeps the unit gives the administrator a new one on the unit itself,
# with the steward's own program: as root, in a shell that has nothing
# of what the service is started with. The service is not stopped.
STEWARD=/opt/evo/bin/evo-usher-steward
NEW_PASSWORD="a lantern in the yard"
pid=$(systemctl show -p MainPID --value evo.service)
# Run the program with what follows as its arguments and the first as
# the one line it reads: how it ended goes to standard output, what it
# said into $work/said.
given() {
  local line="$1" ended=0
  shift
  printf '%s\n' "$line" | "$@" > "$work/said" 2>&1 || ended=$?
  echo "$ended"
}
# The name is held by five wrong tries, as a name whose password is
# lost soon is: the right password is not looked at either.
for _ in 1 2 3 4 5; do
  [ "$(usher sign_in "$(named "$NAME" "three small owls sang")")" = 401 ] || fail "a wrong password signs in: $(cat "$work/answer")"
done
[ "$(usher sign_in "$(named "$NAME" "$PASSWORD")")" = 429 ] || fail "five wrong tries do not hold a name: $(cat "$work/answer")"
# Asked wrongly, the program says how it is asked; a name that is
# nobody's, a person at the front desk and a password that is too short
# are refused, each with a plain word.
[ "$(given "$NEW_PASSWORD" "$STEWARD" --reset-password)" = 2 ] || fail "asked with no name, the program ends otherwise than with 2: $(cat "$work/said")"
grep -q -- '--reset-password <name>' "$work/said" || fail "asked with no name, the program does not say how it is asked: $(cat "$work/said")"
[ "$(given "$NEW_PASSWORD" "$STEWARD" --reset-password Mallory)" = 1 ] || fail "a name that is nobody's is given a password: $(cat "$work/said")"
grep -q -F "The administrators of this usher are: $NAME." "$work/said" || fail "refusing a name that is nobody's, the program does not name the administrators: $(cat "$work/said")"
[ "$(given "$NEW_PASSWORD" "$STEWARD" --reset-password Ben)" = 1 ] || fail "a person at the front desk is given a password: $(cat "$work/said")"
grep -q -F 'new PIN on the page People' "$work/said" || fail "a person at the front desk is refused as: $(cat "$work/said")"
[ "$(given "eleven char" "$STEWARD" --reset-password "$NAME")" = 1 ] || fail "a password of eleven characters is taken: $(cat "$work/said")"
grep -q -F 'A password is 12 characters at least' "$work/said" || fail "a password that is too short is refused as: $(cat "$work/said")"
[ "$(usher me '' "$bearer")" = 200 ] || fail "refused, the program signed the administrator out: $(cat "$work/answer")"
# Asked rightly, it gives the administrator the new password, says so
# in two lines, and says the password nowhere.
[ "$(given "$NEW_PASSWORD" "$STEWARD" --reset-password "$NAME")" = 0 ] || fail "the program does not give an administrator a new password: $(cat "$work/said")"
[ "$(cat "$work/said")" = "$NAME has a new password, and signs in with it from now on.
Whoever was signed in as $NAME is signed out." ] || fail "having given a new password, the program says: $(cat "$work/said")"
! grep -q -F "$NEW_PASSWORD" "$PEOPLE" || fail "the new password stands in the file of the people as it was typed"
journalctl -u evo.service --no-pager > "$work/journal"
! grep -q -F "$NEW_PASSWORD" "$work/journal" || fail "the new password stands in the service's log"
# Written by root, the file of the people is the steward's own as it
# was, with nothing of the writing left beside it.
[ "$(stat -c '%U %G %a' "$PEOPLE")" = "evo evo 600" ] || fail "written by root, the people are kept as $(stat -c '%U %G %a' "$PEOPLE"), not for the steward alone"
[ "$(stat -c '%U %a' "$(dirname "$PEOPLE")")" = "evo 700" ] || fail "the directory of the people is $(stat -c '%U %a' "$(dirname "$PEOPLE")"), not the steward's alone"
[ ! -e "$PEOPLE.new" ] || fail "the writing left a file beside the people"
# The unit takes it up at once and is not started again: whoever was
# signed in as the administrator is signed out, the hold is lifted, the
# new password signs in and the old one does not. Nobody else is
# touched.
[ "$(usher me '' "$bearer")" = 401 ] || fail "who was signed in under the old password still is: $(cat "$work/answer")"
[ "$(usher sign_in "$(named "$NAME" "$NEW_PASSWORD")")" = 200 ] || fail "the new password does not sign in at once: $(cat "$work/answer")"
bearer=$(jq -r '.token' "$work/answer")
[ "$(usher sign_in "$(named "$NAME" "$PASSWORD")")" = 401 ] || fail "the old password still signs in: $(cat "$work/answer")"
[ "$(usher me '' "$desk")" = 200 ] || fail "a new password for the administrator signed a person at the front desk out: $(cat "$work/answer")"
[ "$(systemctl show -p MainPID --value evo.service)" = "$pid" ] || fail "the service was started again for a new password"
echo "  ok    whoever keeps the unit gives its administrator a new password on the unit itself, as root: it signs in at once and the old one does not, the hold of wrong tries is lifted, who was signed in under the old one is signed out, the people stay the steward's own, and the service is not started again"
# The same as the account the steward runs as, which gives back the
# password the rest of the proof signs in with.
[ "$(given "$PASSWORD" runuser -u evo -- "$STEWARD" --reset-password "$NAME")" = 0 ] || fail "as the steward's own account, the program does not give a new password: $(cat "$work/said")"
[ "$(stat -c '%U %G %a' "$PEOPLE")" = "evo evo 600" ] || fail "written by the steward's account, the people are kept as $(stat -c '%U %G %a' "$PEOPLE")"
[ "$(usher me '' "$bearer")" = 401 ] || fail "who was signed in under the password before still is: $(cat "$work/answer")"
[ "$(usher sign_in "$(named "$NAME" "$PASSWORD")")" = 200 ] || fail "the password given as the steward's account does not sign in: $(cat "$work/answer")"
bearer=$(jq -r '.token' "$work/answer")
# The program goes by the level it is told, as the service does: where
# the unit runs at the extended level, it is named to the program too.
[ "$(given "fifteen letters" env USHER_SECURITY=extended "$STEWARD" --reset-password "$NAME")" = 1 ] || fail "told the extended level, the program takes a password of fifteen characters: $(cat "$work/said")"
grep -q -F 'A password is 16 characters at least' "$work/said" || fail "told the extended level, the program refuses a short password as: $(cat "$work/said")"
[ "$(usher me '' "$bearer")" = 200 ] || fail "refused, the program signed the administrator out: $(cat "$work/answer")"
echo "  ok    the same as the steward's own account; and told the extended level, the program holds a password to it"

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
start start
[ "$(usher state)" = 200 ] || fail "the unit does not say what state it is in: $(cat "$work/answer")"
jq -e '.set_up == false and .level == "extended" and .least_password == 16 and .keeps == false' "$work/answer" > /dev/null || fail "a unit of the extended level whose people were taken away says of itself: $(cat "$work/answer")"
# The form it is set up by wears the site's name still.
jq -e '.site.name == "Hall of the Weavers"' "$work/answer" > /dev/null || fail "its people taken away, the unit says of its site: $(cat "$work/answer")"
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
start restart
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
start start
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
