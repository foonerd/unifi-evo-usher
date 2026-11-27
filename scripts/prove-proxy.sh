#!/bin/bash
# Proof that the unit runs in a container as packaging/docker/compose.yaml
# has it, and that the examples under packaging/proxy/ put it behind a
# proxy, as docs/deploying.md says: each is started as it stands in the
# repository, set up and signed in to as a browser does it, and asked as
# a browser asks.
#
#   scripts/prove-proxy.sh [<case>...]
#
# <case> is alone, extended, demo, move, update, satellite, nginx,
# apache, caddy or traefik; with none named, all ten. The usher's image is
# unifi-evo-usher:<version> on this machine, as scripts/image.sh --here
# keeps it, or $USHER_IMAGE:$USHER_VERSION.
#
# Alone: the unit is started by itself, as the image's account, with no
# capability, no way to gain a privilege and a root file system it cannot
# write to, and is asked on its own port, believed by its own authority.
# It holds two thousand connections at once, each asked on as it is
# joined, as a large venue's screens would, and goes on answering. Its
# administrator is given a new
# password by the steward's own program, asked in the container.
#
# In every case the container must come to say that it is healthy: Docker
# asks the usher's own program whether the unit is up, as the Compose
# files and the image have it, and a proxy is started only once it is.
#
# Extended: the same unit with packaging/docker/extended.yaml named after
# its Compose file, which runs it at the extended level of security:
# setting it up takes the unit's code, which it is given as a file.
#
# Demo: the same unit started with USHER_DEMO=1, as a demo: with nothing
# set up but its first administrator it hands a voucher out at the
# counter's request and by a screen that was adopted, on its simulated
# controller; it says that it is a demo; it is given no other
# controller; and its volume is not started as a unit, as a unit's
# volume is not started as a demo.
#
# Move: a unit is moved from one volume to another by its file, as a
# unit is moved from one machine to another. The first gives everything
# it keeps, sealed under a passphrase; the second, which nobody has set
# up, takes the file in and starts again by itself, in the container it
# runs in, as the first: the administrator signs in with the password
# they had, the authority a browser believed the first by is the one
# the second has, and a screen that was adopted by the first hands out
# a voucher at the second with the secret it holds. A wrong passphrase
# is refused and changes nothing.
#
# Update: a unit that ran the newest release is updated to this version,
# and is put back to that release, on the volume it has: what a site
# does with a new version, and what it does where the new version is to
# be left again. The release is the image $USHER_FROM names, as
# <image>:<tag>, or ghcr.io/foonerd/unifi-evo-usher:latest, which is
# fetched where this machine does not have it. Set up on the release,
# with a person at the front desk, a voucher type of the site's own, a
# site's name and a screen that is protected by a PIN, the unit is the
# same after the update and after the way back: its people sign in as
# they did, its types and its site are as they were, the screen hands
# out a voucher with the secret and the PIN it has, a browser believes
# it by the authority it believed it by, and the record keeps every
# hand-out, whichever version wrote it down.
#
# In every case a file of two MiB that is sent to be set up from reaches
# the unit, which answers it: a unit's file is long, and what stands
# before the unit passes it on.
#
# For each proxy: the unit is started and its authority taken from it;
# the proxy is started with a certificate of an authority made for the
# run, which stands in for the site's; then, through the proxy and
# believing nothing but that authority, the face is loaded, a caller who
# has not signed in is refused, the unit is set up with a name and a
# password and signed in to, and with the session that gave is answered
# and hears the counter say over the WebSocket that its offer changed; a
# person is added for the front desk, signs in with the PIN the usher
# made, is told the types on offer and what is stocked for their page,
# is refused what is an administrator's, and hears the counter on their
# line and nothing else; a screen is adopted
# as a satellite by the code it is given, is let ask for the default
# voucher and refused everything else, is protected by a PIN and refused
# a voucher without it, and is disabled and refused.
# Last, the proxy is given another authority than the unit's and must
# then pass nothing on: it believes the unit by the unit's authority and
# by nothing else.
#
# In every case the unit is given a name and a time zone for its site
# where it is started, as the Compose files pass them on: anyone is told
# the name before the unit is set up, its administrator finds both and
# sets others, and a unit that is started again by itself goes by what
# was set and not by what it was started with.
#
# It needs docker with compose, openssl, curl, jq and python3, publishes
# on the loopback address alone, and takes away what it started.
set -euo pipefail
ROOT=$(CDPATH='' cd -- "$(dirname "$0")/.." && pwd)
cd "$ROOT"
. scripts/targets.sh

# The name the examples give the usher.
NAME=usher.example.org

# Who the proof sets a unit up as, and signs in as. Made up.
PERSON="Anna"
PASSWORD="three small owls sing"

# What a unit's site is called, the time zone it keeps and the language
# it speaks where the unit is started, and what its administrator then
# sets. Made up.
export USHER_SITE_NAME="Market Hall" USHER_TIME_ZONE="Europe/Warsaw" USHER_LANGUAGE="de"
SET_NAME="Hall of the Weavers"
SET_ZONE="Europe/London"
SET_LANGUAGE="en"

# Whether a unit is a demo is the proof's to say, case by case, and not
# what the proof was started with.
unset USHER_DEMO

proxies=${*:-alone extended demo move update satellite nginx apache caddy traefik}
# The Compose files of a case, the first the one the unit is in.
files_of() {
  case "$1" in
    alone | demo | move | update) echo packaging/docker/compose.yaml ;;
    satellite) echo packaging/docker/compose.yaml packaging/docker/satellite.yaml ;;
    extended) echo packaging/docker/compose.yaml packaging/docker/extended.yaml ;;
    *) echo "packaging/proxy/$1/compose.yaml" ;;
  esac
}
# The same, as docker compose is told of them.
named_of() {
  local file
  for file in $(files_of "$1"); do
    printf -- '-f %s ' "$file"
  done
}
for proxy in $proxies; do
  for file in $(files_of "$proxy"); do
    [ -f "$file" ] || { echo "prove-proxy: there is no example for '$proxy': no $file" >&2; exit 1; }
  done
done
for tool in docker openssl curl jq python3; do
  command -v "$tool" > /dev/null || { echo "prove-proxy: $tool is not here" >&2; exit 1; }
done
export USHER_IMAGE=${USHER_IMAGE:-unifi-evo-usher}
export USHER_VERSION=${USHER_VERSION:-$(workspace_version)}
docker image inspect "$USHER_IMAGE:$USHER_VERSION" > /dev/null 2>&1 || { echo "prove-proxy: $USHER_IMAGE:$USHER_VERSION is not on this machine; build it first (scripts/image.sh --here)" >&2; exit 1; }

# The unit is the same in every example: as packaging/docker/compose.yaml
# has it, but for the port, which a unit behind a proxy does not publish.
unit_of() {
  # shellcheck disable=SC2046 # one word a file, as they are named
  SITE_CERT=/site.crt SITE_KEY=/site.key USHER_AUTHORITY=/unit-ca.crt \
    docker compose $(named_of "$1") config --format json | jq -S '.services.usher | del(.ports)'
}
# What the proxy of an example waits for of the unit before it is started.
waits_for() {
  # shellcheck disable=SC2046 # one word a file, as they are named
  SITE_CERT=/site.crt SITE_KEY=/site.key USHER_AUTHORITY=/unit-ca.crt \
    docker compose $(named_of "$1") config --format json | jq -r '.services.proxy.depends_on.usher.condition // "nothing"'
}
for proxy in $proxies; do
  # The extended level is the same unit with more said of it, and a
  # demo the same unit told that it is one.
  [ "$proxy" != extended ] || continue
  [ "$proxy" != demo ] || continue
  [ "$proxy" != move ] || continue
  [ "$proxy" != update ] || continue
  [ "$proxy" != satellite ] || continue
  [ "$(unit_of "$proxy")" = "$(unit_of alone)" ] || { echo "prove-proxy: the unit in $(files_of "$proxy") is not the unit of packaging/docker/compose.yaml" >&2; exit 1; }
  # A proxy is started only once the unit says it is up.
  [ "$proxy" = alone ] || [ "$(waits_for "$proxy")" = service_healthy ] || { echo "prove-proxy: the proxy in $(files_of "$proxy") does not wait for the unit to say it is up" >&2; exit 1; }
done

work=$(mktemp -d)
started=""
tidy() {
  local project
  for project in $started; do
    docker compose -p "$project" down --volumes --remove-orphans > /dev/null 2>&1 || true
  done
  # The network of the satellite's case, which no project owns.
  docker network rm usher-proof-lan > /dev/null 2>&1 || true
  rm -rf "$work"
}
trap tidy EXIT

fail() {
  echo "prove-proxy: FAILED: $*" >&2
  if [ -n "${project:-}" ]; then
    docker compose -p "$project" logs --no-color --tail 40 2>&1 | sed 's/^/  /' >&2 || true
  fi
  exit 1
}

# A port of the loopback address that nothing listens on.
free_port() {
  python3 -c 'import socket; s = socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1])'
}

# The site's certificate, of an authority made for this run.
openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:prime256v1 -nodes -days 2 \
  -subj "/CN=the site's authority, for a proof" \
  -addext "basicConstraints=critical,CA:TRUE" -addext "keyUsage=critical,keyCertSign" \
  -keyout "$work/site-ca.key" -out "$work/site-ca.crt" 2> /dev/null
openssl req -newkey ec -pkeyopt ec_paramgen_curve:prime256v1 -nodes \
  -subj "/CN=$NAME" -keyout "$work/site.key" -out "$work/site.csr" 2> /dev/null
printf 'subjectAltName=DNS:%s\nbasicConstraints=CA:FALSE\nkeyUsage=digitalSignature\nextendedKeyUsage=serverAuth\nauthorityKeyIdentifier=keyid\nsubjectKeyIdentifier=hash\n' "$NAME" > "$work/site.ext"
openssl x509 -req -in "$work/site.csr" -CA "$work/site-ca.crt" -CAkey "$work/site-ca.key" -CAcreateserial \
  -days 2 -extfile "$work/site.ext" -out "$work/site.crt" 2> /dev/null
# A proxy in a container reads them as an account of its own.
chmod 644 "$work/site.key" "$work/site.crt"

# One frame of the steward's line of happenings: the line is joined with
# the bearer as a browser joins it, under the name and the authority
# given, happenings are asked for, and the first of the type named is
# printed.
#
#   happening <name> <port> <authority> <bearer> <type>
happening() {
  python3 - "$@" << 'PY'
import base64, json, os, socket, ssl, struct, sys, time

name, port, authority, bearer, wanted = sys.argv[1], int(sys.argv[2]), sys.argv[3], sys.argv[4], sys.argv[5]
context = ssl.create_default_context(cafile=authority)
# The chain and the name are held. A unit's own certificate names no
# authority key, which a strict reading refuses.
context.verify_flags &= ~getattr(ssl, "VERIFY_X509_STRICT", 0)
line = context.wrap_socket(socket.create_connection(("127.0.0.1", port), timeout=10), server_hostname=name)
key = base64.b64encode(os.urandom(16)).decode()
line.sendall((
    f"GET /api/v1/ws HTTP/1.1\r\nHost: {name}:{port}\r\nUpgrade: websocket\r\n"
    f"Connection: Upgrade\r\nSec-WebSocket-Key: {key}\r\nSec-WebSocket-Version: 13\r\n"
    f"Sec-WebSocket-Protocol: evo.bearer.{bearer}\r\n\r\n").encode())
head = b""
while b"\r\n\r\n" not in head:
    more = line.recv(4096)
    if not more:
        sys.exit("the line closed before it was joined")
    head += more
status = head.split(b"\r\n", 1)[0].decode(errors="replace")
if " 101 " not in status + " ":
    sys.exit("the line was not joined: " + status)
# The line is answered on the subprotocol that was offered, and on no
# other: a browser drops a line that is answered on another.
answered = head.decode(errors="replace")
named = [line.split(":", 1)[1].strip() for line in answered.split("\r\n")
         if line.lower().startswith("sec-websocket-protocol:")]
if named != ["evo.bearer." + bearer]:
    sys.exit("the line is not answered on the subprotocol that was offered")
rest = head.split(b"\r\n\r\n", 1)[1]

def send(text):
    data, mask = text.encode(), os.urandom(4)
    size = bytes([0x80 | len(data)]) if len(data) < 126 else bytes([0x80 | 126]) + struct.pack(">H", len(data))
    line.sendall(bytes([0x81]) + size + mask + bytes(b ^ mask[i % 4] for i, b in enumerate(data)))

def take(count):
    global rest
    while len(rest) < count:
        more = line.recv(65536)
        if not more:
            sys.exit("the line closed")
        rest += more
    taken, rest = rest[:count], rest[count:]
    return taken

send(json.dumps({"frame_type": "subscribe", "subscription_id": "happenings", "op": "subscribe_happenings", "payload": {}}))
# Said when happenings are asked for, so that whoever waits knows when
# to make one happen.
print("asked", flush=True)
before = []
until = time.time() + 20
line.settimeout(1)
while time.time() < until:
    try:
        first = take(2)
    except (socket.timeout, TimeoutError):
        continue
    line.settimeout(10)
    opcode, length = first[0] & 0x0F, first[1] & 0x7F
    if length == 126:
        length = struct.unpack(">H", take(2))[0]
    elif length == 127:
        length = struct.unpack(">Q", take(8))[0]
    body = take(length)
    line.settimeout(1)
    if opcode != 1:
        continue
    frame = json.loads(body)
    if frame.get("frame_type") != "subscription_event":
        continue
    event = frame.get("event") or {}
    happened = event.get("happening", event)
    if happened.get("type") == wanted:
        # Every happening heard before it is told with it: a line hears
        # what its session may read, and nothing else.
        happened["heard_before"] = before
        print(json.dumps(happened), flush=True)
        sys.exit(0)
    before.append(happened.get("type"))
sys.exit("no happening of the type came in time")
PY
}

# As many connections as are asked for, held at once as screens hold
# them: each is joined under the name and the authority given, asked on
# at once, as a screen asks as soon as it is joined, and kept open. A
# connection that asks nothing is closed by the unit within ten seconds,
# and only so many of those are let wait at once. With all of them held,
# the unit is asked on the first, on the last and on one more, and each
# has to be answered.
#
#   holds <name> <port> <authority> <count>
holds() {
  python3 - "$@" << 'PY'
import resource, socket, ssl, sys

name, port, authority, count = sys.argv[1], int(sys.argv[2]), sys.argv[3], int(sys.argv[4])
# Whoever holds the connections has as many files open as the unit.
goes_by, at_most = resource.getrlimit(resource.RLIMIT_NOFILE)
needed = count + 64
if goes_by < needed:
    if at_most != resource.RLIM_INFINITY and at_most < needed:
        sys.exit(f"this machine lets the proof have {at_most} files open, and it needs {needed}")
    resource.setrlimit(resource.RLIMIT_NOFILE, (needed, at_most))
context = ssl.create_default_context(cafile=authority)
# The chain and the name are held. A unit's own certificate names no
# authority key, which a strict reading refuses.
context.verify_flags &= ~getattr(ssl, "VERIFY_X509_STRICT", 0)


def joined():
    return context.wrap_socket(socket.create_connection(("127.0.0.1", port), timeout=30), server_hostname=name)


def answered(line):
    # The answer is read whole, so that the connection is fit for the
    # next thing asked on it.
    line.sendall(f"GET /usher/v1/state HTTP/1.1\r\nHost: {name}:{port}\r\n\r\n".encode())
    got = b""
    while b"\r\n\r\n" not in got:
        more = line.recv(4096)
        if not more:
            return "the connection was closed"
        got += more
    head, _, body = got.partition(b"\r\n\r\n")
    lines = head.decode("latin-1").split("\r\n")
    length = 0
    for said in lines[1:]:
        key, _, value = said.partition(":")
        if key.strip().lower() == "content-length":
            length = int(value)
    while len(body) < length:
        more = line.recv(65536)
        if not more:
            return "the connection was closed"
        body += more
    return lines[0]


held = []
for at in range(count):
    try:
        line = joined()
        said = answered(line)
    except OSError as why:
        sys.exit(f"connection {at + 1} of {count} was not taken: {why}")
    if said.split()[1:2] != ["200"]:
        sys.exit(f"connection {at + 1} of {count} is answered: {said}")
    held.append(line)
for which, line in (("first", held[0]), ("last", held[-1]), ("one more", joined())):
    said = answered(line)
    if said.split()[1:2] != ["200"]:
        sys.exit(f"with {count} held, the {which} connection is answered: {said}")
print(len(held))
PY
}

prove() {
  proxy=$1
  project="usher-proof-$proxy"
  started="$started $project"
  local port dir code bearer second keeper status types said
  port=$(free_port)
  dir="$work/$proxy"
  mkdir -p "$dir"
  export USHER_AUTHORITY="$dir/unit-ca.crt"
  export SITE_CERT="$work/site.crt" SITE_KEY="$work/site.key" SITE_ADDRESS=127.0.0.1 SITE_PORT="$port"
  export USHER_ADDRESS=127.0.0.1 USHER_PORT="$port"
  # A name of the proof's own, so that it stands beside a unit that is
  # run on this machine under the examples' name.
  export USHER_NAME="$project"
  code=""
  if [ "$proxy" = extended ]; then
    # The code a unit of the extended level is given, as a file the
    # account 10001 reads.
    (umask 077 && od -An -N6 -tx1 /dev/urandom | tr -d ' \n' > "$dir/unit-code")
    chmod 644 "$dir/unit-code"
    export USHER_CODE="$dir/unit-code"
    code=$(cat "$dir/unit-code")
  fi
  # Whom the caller believes, and under which name: the site's authority
  # through a proxy; the unit's own, which names it localhost, alone.
  local name="$NAME" authority="$work/site-ca.crt" behind=yes
  if [ "$proxy" = alone ] || [ "$proxy" = extended ]; then
    name=localhost
    authority="$dir/unit-ca.crt"
    behind=no
  fi
  # shellcheck disable=SC2046 # one word a file, as they are named
  compose() { docker compose -p "$project" $(named_of "$proxy") "$@"; }
  # A request as a browser makes it, believing that authority alone: the
  # status is printed, and the answer kept.
  ask() {
    local path="$1"
    shift
    curl -sS --max-time 20 -o "$dir/answer" -w '%{http_code}' \
      --cacert "$authority" --resolve "$name:$port:127.0.0.1" \
      "$@" "https://$name:$port$path" 2> "$dir/curl.err" || true
  }
  post() {
    local path="$1" body="$2"
    shift 2
    ask "$path" -X POST -H 'Content-Type: application/json' -d "$body" "$@"
  }
  request() {
    jq -cn --arg shelf "$1" --arg type "$2" --arg payload "$(printf '%s' "$3" | base64 -w0)" \
      '{shelf: $shelf, request_type: $type, payload_b64: $payload}'
  }

  echo "prove-proxy: $proxy"
  # Started as the guide starts it: Compose comes back once Docker, which
  # asks the usher's own program, says of the container that it is
  # healthy.
  compose up -d --wait usher > "$dir/up.log" 2>&1 || {
    cat "$dir/up.log" >&2
    fail "the unit was not started, or does not come to say it is up: $(compose ps -aq usher | head -n 1 | xargs -r docker inspect --format '{{json .State.Health}}' 2> /dev/null | head -c 600)"
  }
  local unit health tries
  unit=$(compose ps -q usher)
  health=$(docker inspect "$unit" --format '{{if .State.Health}}{{.State.Health.Status}}{{else}}not asked{{end}}')
  [ "$health" = healthy ] || fail "Compose came back, and the container is not said to be healthy: $health"
  echo "  ok    the container says it is healthy, on the word of the usher's own program"
  # A unit that is up has made its authority: it makes it when it first
  # starts.
  compose cp usher:/var/lib/evo/state/https/ca.crt "$dir/unit-ca.crt" > /dev/null 2>&1 || fail "the unit that is up made no authority"
  chmod 644 "$dir/unit-ca.crt"
  if [ "$proxy" = alone ]; then
    # The image carries the licences of what is in it: the usher's own,
    # evo's, and the notices of the libraries the program contains.
    for file in copyright LICENSE-evo.txt THIRD-PARTY-NOTICES.txt; do
      compose cp "usher:/usr/share/doc/evo-usher-steward/$file" "$dir/$file" > /dev/null 2>&1 || fail "the image carries no $file"
      [ -s "$dir/$file" ] || fail "the image's $file is empty"
    done
    grep -q '^Apache License 2.0 (Apache-2.0)$' "$dir/THIRD-PARTY-NOTICES.txt" || fail "the image's notices of the libraries name no licence"
    echo "  ok    the image carries the licences: the usher's, evo's, and the notices of the libraries the program contains"
    # The program's word is worth something: asked of a listener that is
    # not there, it says the unit is not up, and which part of it.
    compose exec -T usher /opt/evo/bin/evo-usher-steward --healthy > "$dir/healthy" 2>&1 || fail "asked in the container, the program says the unit is not up: $(cat "$dir/healthy")"
    if compose exec -T -e EVO_HTTPS_LISTEN_ADDR=127.0.0.1:1 usher /opt/evo/bin/evo-usher-steward --healthy > "$dir/healthy" 2>&1; then
      fail "the program says a unit is up whose listener is not where it is said to be"
    fi
    grep -q 'the HTTPS listener takes no connection on 127.0.0.1:1' "$dir/healthy" || fail "the program does not say what of the unit is not up: $(cat "$dir/healthy")"
    echo "  ok    asked of a listener that is not there, the program says the unit is not up, and why"
    # Every screen joined to a unit holds a connection, and a connection
    # is an open file. Two thousand are held at once, twice what a
    # process is commonly started with, and the unit goes on answering.
    holds localhost "$port" "$dir/unit-ca.crt" 2000 > "$dir/held" 2> "$dir/held.err" || fail "the unit does not hold two thousand connections: $(tail -n 1 "$dir/held.err")"
    echo "  ok    $(cat "$dir/held") connections held at once, each asked on as it is joined, and the unit answers on the first, on the last and on one more"
  fi
  if [ "$behind" = no ]; then
    said=$(docker inspect "$unit" --format '{{.Config.User}} {{.HostConfig.ReadonlyRootfs}} {{.HostConfig.CapDrop}} {{.HostConfig.SecurityOpt}}')
    [ "$said" = "10001:10001 true [ALL] [no-new-privileges:true]" ] || fail "the unit does not run with the least privilege: $said"
  else
    compose up -d proxy >> "$dir/up.log" 2>&1 || { cat "$dir/up.log" >&2; fail "the proxy was not started"; }
  fi
  tries=0
  until [ "$(ask /)" = 200 ]; do
    tries=$((tries + 1))
    [ "$tries" -lt 60 ] || fail "the face does not come: $(cat "$dir/curl.err") $(head -c 200 "$dir/answer" 2> /dev/null)"
    sleep 0.5
  done
  grep -q 'door.js' "$dir/answer" || fail "what came is not the door"
  [ "$(ask /org.evoframework.usher.shell/pages.json)" = 200 ] || fail "the shell does not come"
  # Every module a browser loads for the shell comes: the shell's own,
  # and whatever each imports from beside it.
  local seen=" " queue=shell.js module named
  while [ -n "$queue" ]; do
    module=${queue%% *}
    queue=${queue#"$module"}
    queue=${queue# }
    case "$seen" in *" $module "*) continue ;; esac
    seen="$seen$module "
    [ "$(ask "/org.evoframework.usher.shell/$module")" = 200 ] || fail "$module, which the shell loads, does not come"
    for named in $(grep -oE 'from "\./[A-Za-z0-9_-]+\.js"' "$dir/answer" | sed -e 's/^from "\.\///' -e 's/"$//'); do
      queue="${queue:+$queue }$named"
    done
  done
  # The words the face says come with it: the list of the languages it
  # speaks, and the texts of each.
  [ "$(ask /org.evoframework.usher.shell/lang/languages.json)" = 200 ] || fail "the list of the face's languages does not come"
  local spoken
  spoken=$(jq -r 'map(.tag) | join(" ")' "$dir/answer")
  [ "${spoken%% *}" = en ] || fail "the face's languages are $spoken, and English is not the first"
  for language in $spoken; do
    [ "$(ask "/org.evoframework.usher.shell/lang/$language.json")" = 200 ] || fail "the texts of $language do not come"
    jq -e '."common.save" | type == "string"' "$dir/answer" > /dev/null || fail "what came for $language is no file of texts"
  done
  if [ "$behind" = no ]; then
    echo "  ok    the unit runs as its own account, with no capability, no privilege to gain and a root it cannot write to"
    echo "  ok    the face comes, under the certificate of the unit's own authority"
  else
    echo "  ok    the face comes through, under the site's certificate"
  fi

  # Who has not signed in is not let in, and no bearer is had by evo's
  # own pairing.
  types=$(request issue.counter types '{}')
  status=$(post /api/v1/request "$types")
  [ "$status" = 401 ] || fail "a caller who has not signed in was answered $status: $(cat "$dir/answer")"
  status=$(post /api/v1/pair_complete '{"pair_id": "bootstrap", "code": "000000000000"}')
  [ "$status" = 401 ] || fail "evo's pairing is open beside signing in: $status $(cat "$dir/answer")"

  # A new unit says that nobody has set it up, and at which level it is.
  local level=standard least=12
  if [ "$proxy" = extended ]; then
    level=extended
    least=16
  fi
  status=$(ask /usher/v1/state)
  [ "$status" = 200 ] || fail "the unit does not say what state it is in: $status $(cat "$dir/answer")"
  jq -e --arg level "$level" --argjson least "$least" '.set_up == false and .level == $level and .least_password == $least' "$dir/answer" > /dev/null \
    || fail "a new unit says of itself: $(cat "$dir/answer")"
  # And what its site is called and what language it speaks, as it was
  # started with: the form it is set up by wears the name, in that
  # language. Nothing else of the site is said.
  jq -e --arg name "$USHER_SITE_NAME" --arg language "$USHER_LANGUAGE" '.site == {name: $name, logo: null, language: $language, follows_browser: false}' "$dir/answer" > /dev/null \
    || fail "a new unit that was started with a name says of its site: $(cat "$dir/answer")"
  # Whoever opens it first sets it up, with a name and a password; at
  # the extended level with the unit's code as well.
  person() {
    jq -cn --arg name "$1" --arg password "$2" --arg code "${3:-}" \
      '{name: $name, password: $password} + (if $code == "" then {} else {code: $code} end)'
  }
  if [ "$proxy" = extended ]; then
    status=$(post /usher/v1/set_up "$(person "$PERSON" "$PASSWORD")")
    [ "$status" = 403 ] || fail "a unit of the extended level is set up without its code: $status $(cat "$dir/answer")"
    status=$(post /usher/v1/set_up "$(person "$PERSON" "$PASSWORD" 000000000000)")
    [ "$status" = 403 ] || fail "a unit of the extended level is set up with a wrong code: $status $(cat "$dir/answer")"
    status=$(post /usher/v1/set_up "$(person "$PERSON" "fifteen letters" "$code")")
    [ "$status" = 400 ] || fail "the extended level takes a password of fifteen characters: $status $(cat "$dir/answer")"
  fi
  status=$(post /usher/v1/set_up "$(person "$PERSON" "$PASSWORD" "$code")")
  [ "$status" = 200 ] || fail "whoever opens a new unit first does not set it up: $status $(cat "$dir/answer")"
  bearer=$(jq -r '.token' "$dir/answer")
  status=$(post /usher/v1/set_up "$(person Mallory "$PASSWORD" "$code")")
  [ "$status" = 409 ] || fail "a unit is set up a second time: $status $(cat "$dir/answer")"
  status=$(post /api/v1/request "$types" -H "Authorization: Bearer $bearer")
  [ "$status" = 200 ] || fail "who set the unit up is refused: $status $(cat "$dir/answer")"
  said=$(jq -r '.payload_b64' "$dir/answer" | base64 -d)
  [ "$(printf '%s' "$said" | jq -c '.types')" = '[]' ] || fail "the counter answered $said"
  if [ "$proxy" = extended ]; then
    echo "  ok    who has not signed in is refused; at the extended level the unit is set up with its code and a longer password, and answers who set it up"
  else
    echo "  ok    who has not signed in is refused; whoever opens the unit first sets it up, and is answered"
  fi

  # The site: the unit goes by the name, the time zone and the language
  # it was started with until its administrator sets others, and anyone
  # is told the name and the language.
  status=$(post /api/v1/request "$(request site.settings get '{}')" -H "Authorization: Bearer $bearer")
  [ "$status" = 200 ] || fail "the site is not told to who has signed in: $status $(cat "$dir/answer")"
  jq -r '.payload_b64' "$dir/answer" | base64 -d \
    | jq -e --arg name "$USHER_SITE_NAME" --arg zone "$USHER_TIME_ZONE" --arg language "$USHER_LANGUAGE" '.name == $name and .time_zone == $zone and .set_here == false and .logo == null and .language == $language and .follows_browser == false and .language_set_here == false' > /dev/null \
    || fail "the unit does not go by the name, the time zone and the language it was started with: $(jq -r '.payload_b64' "$dir/answer" | base64 -d)"
  # A language that is none is refused, under a code a screen finds its
  # own words by, and changes nothing.
  status=$(post /api/v1/request "$(request site.settings set '{"name": "Elsewhere", "language": "German"}')" -H "Authorization: Bearer $bearer")
  [ "$status" != 200 ] || fail "a language that is none is taken: $(cat "$dir/answer")"
  jq -e '.error.details.subclass == "language_not_one"' "$dir/answer" > /dev/null || fail "a language that is none is refused as: $(cat "$dir/answer")"
  said=$(jq -cn --arg name "$SET_NAME" --arg zone "$SET_ZONE" --arg language "$SET_LANGUAGE" '{name: $name, time_zone: $zone, language: $language, follows_browser: true}')
  status=$(post /api/v1/request "$(request site.settings set "$said")")
  [ "$status" = 401 ] || fail "a caller who has not signed in sets the site: $status $(cat "$dir/answer")"
  status=$(post /api/v1/request "$(request site.settings set "$said")" -H "Authorization: Bearer $bearer")
  [ "$status" = 200 ] || fail "the site's name, time zone and language are not set: $status $(cat "$dir/answer")"
  status=$(ask /usher/v1/state)
  jq -e --arg name "$SET_NAME" --arg language "$SET_LANGUAGE" '.site == {name: $name, logo: null, language: $language, follows_browser: true}' "$dir/answer" > /dev/null \
    || fail "before anyone has signed in, the unit says of its site: $status $(cat "$dir/answer")"
  echo "  ok    the unit goes by the name, the time zone and the language it was started with; its administrator sets others, and anyone is told the name and the language"

  # A unit's file is long. One of two MiB, sent to be set up from,
  # reaches the unit through whatever stands before it, and is answered
  # by the unit: it is set up already, and says so.
  head -c 1500000 /dev/zero | base64 -w0 > "$dir/long"
  jq -cn --rawfile long "$dir/long" '{file: {usher_file: 1, settings: {padding: $long}}, passphrase: "of no file"}' > "$dir/long.json"
  status=$(ask /usher/v1/set_up_from_file -X POST -H 'Content-Type: application/json' --data-binary "@$dir/long.json")
  [ "$status" = 409 ] || fail "a file of two MiB did not reach the unit: $status $(head -c 300 "$dir/answer")"
  jq -e '.error.details.subclass == "set_up_already"' "$dir/answer" > /dev/null || fail "a file sent to a unit that is set up is answered: $(head -c 300 "$dir/answer")"
  rm -f "$dir/long" "$dir/long.json"
  echo "  ok    a file of two MiB that is sent to be set up from reaches the unit, which is set up already and says so"

  # The name and the password sign in, a wrong one does not, and a
  # session is refused once it is signed out of.
  status=$(post /usher/v1/sign_in "$(person "$PERSON" "three small owls sang")")
  [ "$status" = 401 ] || fail "a wrong password signs in: $status $(cat "$dir/answer")"
  jq -e '.error.details.subclass == "wrong_name_or_password"' "$dir/answer" > /dev/null || fail "a wrong password is refused under no code: $(cat "$dir/answer")"
  status=$(post /usher/v1/sign_in "$(person "$PERSON" "$PASSWORD")")
  [ "$status" = 200 ] || fail "the name and the password do not sign in: $status $(cat "$dir/answer")"
  second=$(jq -r '.token' "$dir/answer")
  status=$(ask /usher/v1/people -H "Authorization: Bearer $second")
  [ "$status" = 200 ] || fail "who has signed in is not told of the people: $status $(cat "$dir/answer")"
  status=$(post /usher/v1/sign_out '{}' -H "Authorization: Bearer $second")
  [ "$status" = 200 ] || fail "signing out is refused: $status $(cat "$dir/answer")"
  status=$(post /api/v1/request "$types" -H "Authorization: Bearer $second")
  [ "$status" = 401 ] || fail "a session is answered after it was signed out of: $status $(cat "$dir/answer")"
  echo "  ok    the name and the password sign in, a wrong one does not, and a session is refused once it is signed out of"

  # A happening, made while the line is open. First something of evo's
  # own, which no person's line reads: evo's household level, which on
  # an usher protects nothing, set by the unit's keeper, who has the
  # unit's own bearer out of the unit's files. Then the counter's offer
  # is changed, which the counter says, and which an administrator's
  # line hears: that, and nothing of what happened before it.
  compose cp usher:/var/lib/evo/state/https/bootstrap.token "$dir/units-bearer" > /dev/null 2>&1 || fail "the unit's own bearer is not among its files"
  keeper=$(tr -d ' \n' < "$dir/units-bearer")
  # Hear the counter say that its offer changed, on the line of the
  # session given: what was heard is left in $said.
  hears_the_counter() {
    local session="$1" listener
    rm -f "$dir/line"
    mkfifo "$dir/line"
    happening "$name" "$port" "$authority" "$session" plugin_event > "$dir/line" 2> "$dir/line.err" &
    listener=$!
    {
      read -r said || said=""
      [ "$said" = asked ] || { wait "$listener" || true; fail "the WebSocket does not come through: $(cat "$dir/line.err")"; }
      status=$(post /api/v1/household_protection_set '{"level": "open", "lend": false}' -H "Authorization: Bearer $keeper")
      [ "$status" = 200 ] || fail "nothing of evo's own was made to happen: $status $(cat "$dir/answer")"
      status=$(post /api/v1/request "$(request issue.counter put_type '{"name": "Heard on the line", "minutes": 60}')" -H "Authorization: Bearer $bearer")
      [ "$status" = 200 ] || fail "the counter's offer was not changed: $status $(cat "$dir/answer")"
      read -r said || said=""
    } < "$dir/line"
    wait "$listener" || fail "the counter was not heard over the WebSocket: $(cat "$dir/line.err")"
    status=$(post /api/v1/request "$(request issue.counter drop_type '{"name": "Heard on the line"}')" -H "Authorization: Bearer $bearer")
    [ "$status" = 200 ] || fail "the voucher type made for the line was not deleted: $status $(cat "$dir/answer")"
    printf '%s' "$said" | jq -e '.type == "plugin_event" and .plugin == "org.evoframework.usher.counter" and .event_type == "offer_changed"' > /dev/null || fail "what came over the WebSocket is not the counter saying that its offer changed: $said"
    printf '%s' "$said" | jq -e '.heard_before == []' > /dev/null || fail "the line heard what its session does not read: $said"
  }
  hears_the_counter "$bearer"
  echo "  ok    the counter is heard over the WebSocket when its offer changes, joined with the session, and nothing of evo's own is heard with it"

  # A person at the front desk: added by the administrator, with a PIN
  # the usher makes, of as many digits as the unit's level asks. They
  # sign in with it, are answered what the counter's page asks, are
  # refused by name what is an administrator's, and hear on their line
  # what the counter says and nothing else of the unit.
  local pin desk digits=6
  [ "$proxy" != extended ] || digits=8
  status=$(post /usher/v1/add_person '{"name": "Ben", "role": "front_desk"}' -H "Authorization: Bearer $bearer")
  [ "$status" = 200 ] || fail "a person is not added for the front desk: $status $(cat "$dir/answer")"
  pin=$(jq -r '.pin' "$dir/answer")
  { [[ "$pin" =~ ^[0-9]+$ ]] && [ "${#pin}" = "$digits" ]; } || fail "the PIN the usher made is not $digits digits: $(cat "$dir/answer")"
  status=$(post /usher/v1/sign_in "$(person Ben "$pin")")
  [ "$status" = 200 ] || fail "the name and the PIN do not sign in: $status $(cat "$dir/answer")"
  jq -e '.name == "Ben" and .role == "front_desk"' "$dir/answer" > /dev/null || fail "the unit says of who signed in with a PIN: $(cat "$dir/answer")"
  desk=$(jq -r '.token' "$dir/answer")
  status=$(post /api/v1/request "$(request issue.counter offered '{}')" -H "Authorization: Bearer $desk")
  [ "$status" = 200 ] || fail "a person at the front desk is not told the types on offer: $status $(cat "$dir/answer")"
  status=$(post /api/v1/request "$types" -H "Authorization: Bearer $desk")
  [ "$status" = 403 ] || fail "a person at the front desk is answered the voucher types: $status $(cat "$dir/answer")"
  jq -e '.error.details.subclass == "not_for_front_desk"' "$dir/answer" > /dev/null || fail "the voucher types are refused a person at the front desk as: $(cat "$dir/answer")"
  status=$(ask /usher/v1/people -H "Authorization: Bearer $desk")
  [ "$status" = 403 ] || fail "a person at the front desk is told of the people: $status $(cat "$dir/answer")"
  status=$(ask /api/v1/list_bearer_tokens -H "Authorization: Bearer $desk")
  [ "$status" = 403 ] || fail "a person at the front desk is told of the unit's bearers: $status $(cat "$dir/answer")"
  # What is stocked for their page is told them by the usher, and of
  # it the counter alone; evo's own word on what is stocked is refused
  # them by the gate, and which shell is active, which the gate lets
  # anyone ask, evo itself refuses a session that is not good for the
  # plugins: it is asked with nothing presented.
  status=$(ask /usher/v1/stocked -H "Authorization: Bearer $desk")
  [ "$status" = 200 ] || fail "a person at the front desk is not told what is stocked for their page: $status $(cat "$dir/answer")"
  jq -e '(.entries | map(.widget)) == ["usher.counter"] and (.shelves | map(.id)) == ["issue.counter"]' "$dir/answer" > /dev/null || fail "a person at the front desk is told to be stocked: $(cat "$dir/answer")"
  status=$(ask /api/v1/describe_ui_stockings -H "Authorization: Bearer $desk")
  [ "$status" = 403 ] || fail "a person at the front desk is told everything that is stocked: $status $(cat "$dir/answer")"
  jq -e '.error.details.subclass == "not_for_front_desk"' "$dir/answer" > /dev/null || fail "what is stocked is refused a person at the front desk as: $(cat "$dir/answer")"
  status=$(ask /api/v1/describe_active_ui_selection -H "Authorization: Bearer $desk")
  [ "$status" = 403 ] || fail "evo answers a session that is not good for the plugins which shell is active: $status $(cat "$dir/answer")"
  ! grep -q 'not_for_front_desk' "$dir/answer" || fail "which shell is active was refused by the gate, and not by evo: $(cat "$dir/answer")"
  [ "$(ask /api/v1/describe_active_ui_selection)" = 200 ] || fail "which shell is active is not told a caller who presents nothing"
  # Their line hears the counter, and nothing of evo's own.
  hears_the_counter "$desk"
  echo "  ok    a person at the front desk signs in with a PIN the usher made, is told the types on offer and what is stocked for their page, is refused what is an administrator's by the gate and by evo, and hears the counter on their line and nothing else"

  # A satellite: a screen that nobody is signed in at opens a page of
  # its own, is given a code, and is adopted by it. It is let ask for
  # the default voucher and whether it is still a satellite, which
  # tells it what the site is called; everything else is refused it by
  # name, and no line of what happens is opened to it. Disabled, it is
  # signed out and refused.
  local shown secret presenting satellite screen
  [ "$(ask /satellite/)" = 200 ] || fail "the page of a satellite does not come"
  grep -q 'satellite.js' "$dir/answer" || fail "what came for a satellite is not its page"
  [ "$(ask /satellite/satellite.js)" = 200 ] || fail "the script of a satellite's page does not come"
  status=$(post /usher/v1/satellite_code '{}')
  [ "$status" = 200 ] || fail "a screen is not given a code: $status $(cat "$dir/answer")"
  shown=$(jq -r '.code' "$dir/answer")
  secret=$(jq -r '.secret' "$dir/answer")
  { [[ "$shown" =~ ^[0-9]{6}$ ]] && [[ "$secret" =~ ^[0-9a-f]{64}$ ]]; } || fail "a screen is given no code of six digits and secret of 256 bits"
  presenting=$(jq -cn --arg secret "$secret" '{secret: $secret}')
  status=$(post /usher/v1/satellite_sign_in "$presenting")
  [ "$status" = 200 ] || fail "a screen that waits is not told so: $status $(cat "$dir/answer")"
  jq -e '.adopted == false and (has("token") | not)' "$dir/answer" > /dev/null || fail "a screen nobody adopted is answered: $(cat "$dir/answer")"
  said=$(jq -cn --arg code "$shown" '{code: $code, name: "Lobby"}')
  status=$(post /usher/v1/adopt_satellite "$said")
  [ "$status" = 401 ] || fail "a caller who has not signed in adopts a screen: $status $(cat "$dir/answer")"
  status=$(post /usher/v1/adopt_satellite "$said" -H "Authorization: Bearer $desk")
  [ "$status" = 403 ] || fail "a person at the front desk adopts a screen: $status $(cat "$dir/answer")"
  status=$(post /usher/v1/adopt_satellite "$said" -H "Authorization: Bearer $bearer")
  [ "$status" = 200 ] || fail "a screen is not adopted by its code: $status $(cat "$dir/answer")"
  satellite=$(jq -r '.adopted.id' "$dir/answer")
  status=$(post /usher/v1/satellite_sign_in "$presenting")
  [ "$status" = 200 ] || fail "a satellite's secret is not exchanged for a session: $status $(cat "$dir/answer")"
  jq -e '.adopted == true and .name == "Lobby"' "$dir/answer" > /dev/null || fail "a satellite that presents its secret is answered: $(cat "$dir/answer")"
  screen=$(jq -r '.token' "$dir/answer")
  status=$(ask /usher/v1/satellite -H "Authorization: Bearer $screen")
  [ "$status" = 200 ] || fail "a satellite is not told that it is one: $status $(cat "$dir/answer")"
  jq -e --arg name "$SET_NAME" '.name == "Lobby" and .limit == 20 and .site.name == $name' "$dir/answer" > /dev/null || fail "a satellite that says it is there is answered: $(cat "$dir/answer")"
  # The default voucher is its to ask for: the counter is asked, and
  # says that no voucher type is the default yet, which is the
  # administrator's to set.
  status=$(post /api/v1/request "$(request issue.counter hand_default '{}')" -H "Authorization: Bearer $screen")
  [ "$status" != 403 ] || fail "a satellite is kept from the default voucher: $(cat "$dir/answer")"
  jq -e '.error.details.subclass == "no_default_type"' "$dir/answer" > /dev/null || fail "the counter answered a satellite: $status $(cat "$dir/answer")"
  for said in "issue.counter hand" "issue.counter offered" "issue.counter types" "stock.reserve count" "site.settings get" "site.settings set" "access.controller info"; do
    status=$(post /api/v1/request "$(request "${said% *}" "${said#* }" '{}')" -H "Authorization: Bearer $screen")
    [ "$status" = 403 ] || fail "a satellite is answered $said: $status $(cat "$dir/answer")"
    jq -e '.error.details.subclass == "not_for_satellite"' "$dir/answer" > /dev/null || fail "$said is refused a satellite as: $(cat "$dir/answer")"
  done
  for said in /usher/v1/me /usher/v1/stocked /usher/v1/people /usher/v1/satellites /api/v1/list_bearer_tokens /api/v1/describe_ui_stockings; do
    status=$(ask "$said" -H "Authorization: Bearer $screen")
    [ "$status" = 403 ] || fail "a satellite is answered $said: $status $(cat "$dir/answer")"
  done
  status=$(ask /api/v1/ws --http1.1 -H "Connection: Upgrade" -H "Upgrade: websocket" -H "Sec-WebSocket-Version: 13" \
    -H "Sec-WebSocket-Key: dGhlIHVzaGVyJ3MgcHJvb2Y=" -H "Sec-WebSocket-Protocol: evo.bearer.$screen")
  [ "$status" = 403 ] || fail "the line of what happens is opened to a satellite: $status $(cat "$dir/answer")"
  status=$(ask /usher/v1/satellites -H "Authorization: Bearer $bearer")
  [ "$status" = 200 ] || fail "the satellites are not listed: $status $(cat "$dir/answer")"
  jq -e --arg id "$satellite" '(.satellites | length) == 1 and .satellites[0].id == $id and (.satellites[0].last_seen_ms | type == "number")' "$dir/answer" > /dev/null || fail "of its satellites the unit says: $(cat "$dir/answer")"
  # Protected, it gives a voucher for its PIN and not without: the PIN
  # is judged by the unit, behind the proxy as before it, and the
  # screen learns that it is protected on the session it has.
  # A PIN of eight digits is one at either level; one of six is too
  # short for a unit at the extended level, and is refused there.
  status=$(post /usher/v1/set_satellite "$(jq -cn --arg id "$satellite" '{id: $id, mode: "protected", pin: "480273"}')" -H "Authorization: Bearer $bearer")
  if [ "$proxy" = extended ]; then
    [ "$status" = 400 ] || fail "at the extended level a satellite is protected by a PIN of six digits: $status $(cat "$dir/answer")"
    jq -e '.error.details.subclass == "pin_too_short"' "$dir/answer" > /dev/null || fail "a PIN of six digits is refused a satellite at the extended level as: $(cat "$dir/answer")"
  else
    [ "$status" = 200 ] || fail "a satellite is not protected by a PIN of six digits: $status $(cat "$dir/answer")"
  fi
  status=$(post /usher/v1/set_satellite "$(jq -cn --arg id "$satellite" '{id: $id, mode: "protected", pin: "48027316"}')" -H "Authorization: Bearer $bearer")
  [ "$status" = 200 ] || fail "a satellite is not protected: $status $(cat "$dir/answer")"
  ! grep -q -F 48027316 "$dir/answer" || fail "the PIN of a satellite is answered back"
  status=$(ask /usher/v1/satellite -H "Authorization: Bearer $screen")
  [ "$status" = 200 ] || fail "a satellite that was protected is signed out: $status $(cat "$dir/answer")"
  jq -e '.protected == true and .types == [] and .logo == null' "$dir/answer" > /dev/null || fail "a satellite that was protected is told: $(cat "$dir/answer")"
  status=$(post /api/v1/request "$(request issue.counter hand_default '{}')" -H "Authorization: Bearer $screen")
  [ "$status" = 403 ] || fail "a protected satellite is let ask for a voucher with no PIN: $status $(cat "$dir/answer")"
  jq -e '.error.details.subclass == "satellite_pin_required"' "$dir/answer" > /dev/null || fail "a protected satellite asked with no PIN is refused as: $(cat "$dir/answer")"
  status=$(post /api/v1/request "$(request issue.counter hand_default '{"pin": "11111111"}')" -H "Authorization: Bearer $screen")
  [ "$status" = 403 ] || fail "a protected satellite is let ask for a voucher with a wrong PIN: $status $(cat "$dir/answer")"
  jq -e '.error.details.subclass == "satellite_pin_wrong"' "$dir/answer" > /dev/null || fail "a wrong PIN is refused a satellite as: $(cat "$dir/answer")"
  status=$(post /api/v1/request "$(request issue.counter hand_default '{"pin": "48027316"}')" -H "Authorization: Bearer $screen")
  [ "$status" != 403 ] || fail "a protected satellite is kept from the default voucher with its PIN: $(cat "$dir/answer")"
  jq -e '.error.details.subclass == "no_default_type"' "$dir/answer" > /dev/null || fail "the counter answered a protected satellite: $status $(cat "$dir/answer")"
  status=$(post /api/v1/request "$(request issue.counter hand_type '{"type": "Day", "pin": "48027316"}')" -H "Authorization: Bearer $screen")
  [ "$status" = 403 ] || fail "a satellite that was given no type is let ask for one by name: $status $(cat "$dir/answer")"
  jq -e '.error.details.subclass == "satellite_type_not_given"' "$dir/answer" > /dev/null || fail "a type by name is refused a satellite as: $(cat "$dir/answer")"
  status=$(ask "/usher/v1/satellite_logo" -H "Authorization: Bearer $screen")
  [ "$status" = 404 ] || fail "a satellite that wears no logo is given one: $status"
  echo "  ok    protected, a satellite is refused a voucher with no PIN and with a wrong one, and is let ask with its own"

  status=$(post /usher/v1/set_satellite "$(jq -cn --arg id "$satellite" '{id: $id, enabled: false}')" -H "Authorization: Bearer $bearer")
  [ "$status" = 200 ] || fail "a satellite is not disabled: $status $(cat "$dir/answer")"
  status=$(ask /usher/v1/satellite -H "Authorization: Bearer $screen")
  [ "$status" = 401 ] || fail "a satellite that was disabled is answered: $status $(cat "$dir/answer")"
  status=$(post /api/v1/request "$(request issue.counter hand_default '{}')" -H "Authorization: Bearer $screen")
  [ "$status" = 401 ] || fail "a satellite that was disabled is let ask for a voucher: $status $(cat "$dir/answer")"
  status=$(post /usher/v1/satellite_sign_in "$presenting")
  [ "$status" = 403 ] || fail "the secret of a satellite that was disabled is exchanged: $status $(cat "$dir/answer")"
  jq -e '.error.details.subclass == "satellite_disabled"' "$dir/answer" > /dev/null || fail "a satellite that was disabled is refused as: $(cat "$dir/answer")"
  echo "  ok    a screen is adopted as a satellite by the code it is given, is let ask for the default voucher and refused the rest and the line of what happens; disabled, it is signed out and refused"

  if [ "$behind" = no ]; then
    # What the unit kept is there when it is started again, who is
    # signed in among it.
    compose restart usher >> "$dir/up.log" 2>&1 || fail "the unit was not started again"
    tries=0
    until [ "$(post /api/v1/request "$types" -H "Authorization: Bearer $bearer")" = 200 ]; do
      tries=$((tries + 1))
      [ "$tries" -lt 60 ] || fail "who was signed in is refused after a restart: $(cat "$dir/answer")"
      sleep 0.5
    done
    status=$(ask /usher/v1/state)
    jq -e '.set_up == true' "$dir/answer" > /dev/null || fail "started again, the unit says of itself: $status $(cat "$dir/answer")"
    # What its administrator set of the site stands, though the unit is
    # started with the first name, time zone and language as before.
    jq -e --arg name "$SET_NAME" --arg language "$SET_LANGUAGE" '.site.name == $name and .site.language == $language and .site.follows_browser == true' "$dir/answer" > /dev/null || fail "started again, the unit says of its site: $(cat "$dir/answer")"
    status=$(post /api/v1/request "$(request site.settings get '{}')" -H "Authorization: Bearer $bearer")
    [ "$status" = 200 ] || fail "the site is not told after a restart: $status $(cat "$dir/answer")"
    jq -r '.payload_b64' "$dir/answer" | base64 -d \
      | jq -e --arg name "$SET_NAME" --arg zone "$SET_ZONE" --arg language "$SET_LANGUAGE" '.name == $name and .time_zone == $zone and .set_here == true and .language == $language and .follows_browser == true and .language_set_here == true' > /dev/null \
      || fail "what was set of the site was not kept through a restart: $(jq -r '.payload_b64' "$dir/answer" | base64 -d)"
    echo "  ok    started again, the unit has its people, still answers who was signed in, and goes by the name, the time zone and the language that were set"
    if [ "$proxy" = alone ]; then
      # A password that is lost: whoever has the unit gives its
      # administrator a new one, in the container, with the steward's
      # own program. The password is one line of what the program
      # reads, as a script gives it. The unit is not started again.
      local renewed="seven grey herons wade" ended=0
      printf '%s\n' "$renewed" | compose exec -T usher /opt/evo/bin/evo-usher-steward --reset-password "$PERSON" > "$dir/reset" 2>&1 || ended=$?
      [ "$ended" = 0 ] || fail "asked in the container, the program does not give an administrator a new password: $ended $(cat "$dir/reset")"
      grep -q -F "$PERSON has a new password" "$dir/reset" || fail "having given a new password, the program says: $(cat "$dir/reset")"
      ! grep -q -F "$renewed" "$dir/reset" || fail "the program says the password it was given"
      status=$(post /api/v1/request "$types" -H "Authorization: Bearer $bearer")
      [ "$status" = 401 ] || fail "who was signed in under the old password is answered: $status $(cat "$dir/answer")"
      status=$(post /usher/v1/sign_in "$(person "$PERSON" "$renewed")")
      [ "$status" = 200 ] || fail "the new password does not sign in at once: $status $(cat "$dir/answer")"
      bearer=$(jq -r '.token' "$dir/answer")
      status=$(post /api/v1/request "$types" -H "Authorization: Bearer $bearer")
      [ "$status" = 200 ] || fail "who signed in with the new password is refused: $status $(cat "$dir/answer")"
      status=$(post /usher/v1/sign_in "$(person "$PERSON" "$PASSWORD")")
      [ "$status" = 401 ] || fail "the old password still signs in: $status $(cat "$dir/answer")"
      # A name that is nobody's is refused, with the administrators
      # named, and nothing is changed.
      ended=0
      printf '%s\n' "$renewed" | compose exec -T usher /opt/evo/bin/evo-usher-steward --reset-password Mallory > "$dir/reset" 2>&1 || ended=$?
      [ "$ended" = 1 ] || fail "asked for a name that is nobody's, the program ends with $ended: $(cat "$dir/reset")"
      grep -q -F "The administrators of this usher are: $PERSON." "$dir/reset" || fail "a name that is nobody's is refused as: $(cat "$dir/reset")"
      status=$(post /api/v1/request "$types" -H "Authorization: Bearer $bearer")
      [ "$status" = 200 ] || fail "refused, the program signed the administrator out: $status $(cat "$dir/answer")"
      echo "  ok    whoever has the unit gives its administrator a new password in the container: it signs in at once, the old one does not, and who was signed in under the old one is signed out"
      # Where nothing else helps: whoever has the unit puts a file of
      # nobody in the place of its people, and the unit is set up anew.
      echo '[]' > "$dir/people.json"
      compose cp "$dir/people.json" usher:/var/lib/evo/state/usher/people.json > /dev/null 2>&1 || fail "the file of the people is not put into the unit"
      status=$(post /api/v1/request "$types" -H "Authorization: Bearer $bearer")
      [ "$status" = 401 ] || fail "who was signed in is answered after the people were taken away: $status $(cat "$dir/answer")"
      status=$(ask /usher/v1/state)
      jq -e '.set_up == false' "$dir/answer" > /dev/null || fail "its people taken away, the unit says of itself: $status $(cat "$dir/answer")"
      status=$(post /usher/v1/set_up "$(person Ben "a lantern in the yard")")
      [ "$status" = 200 ] || fail "the unit is not set up anew: $status $(cat "$dir/answer")"
      bearer=$(jq -r '.token' "$dir/answer")
      status=$(post /api/v1/request "$types" -H "Authorization: Bearer $bearer")
      [ "$status" = 200 ] || fail "who set the unit up anew is refused: $status $(cat "$dir/answer")"
      status=$(post /usher/v1/sign_in "$(person "$PERSON" "$renewed")")
      [ "$status" = 401 ] || fail "who was taken away still signs in: $status $(cat "$dir/answer")"
      echo "  ok    its people taken away by whoever has the unit, it is set up anew, and nobody who was signed in still is"
    fi
    compose down --volumes --remove-orphans > /dev/null 2>&1 || true
    project=""
    return
  fi

  # With another authority than the unit's, the proxy passes nothing on.
  export USHER_AUTHORITY="$work/site-ca.crt"
  compose up -d --force-recreate proxy >> "$dir/up.log" 2>&1 || { cat "$dir/up.log" >&2; fail "the proxy was not started again"; }
  tries=0
  status=000
  while [ "$tries" -lt 40 ]; do
    status=$(ask /)
    case "$status" in 5??) break ;; esac
    [ "$status" != 200 ] || fail "the proxy passes the unit on without believing it by the unit's authority"
    tries=$((tries + 1))
    sleep 0.5
  done
  case "$status" in
    5??) echo "  ok    given another authority than the unit's, the proxy passes nothing on ($status)" ;;
    *) fail "the proxy answered $status where it cannot believe the unit" ;;
  esac

  compose down --volumes --remove-orphans > /dev/null 2>&1 || true
  project=""
}

# A unit started as a demo, and what a demo's volume and a unit's are
# started as afterwards.
prove_demo() {
  proxy=demo
  project="usher-proof-demo"
  started="$started $project $project-unit"
  local port dir bearer status unit shown secret screen tries
  port=$(free_port)
  dir="$work/demo"
  mkdir -p "$dir"
  export USHER_ADDRESS=127.0.0.1 USHER_PORT="$port" USHER_NAME="$project"
  local name=localhost authority="$dir/unit-ca.crt" logged
  compose() { docker compose -p "$project" -f packaging/docker/compose.yaml "$@"; }
  ask() {
    local path="$1"
    shift
    curl -sS --max-time 20 -o "$dir/answer" -w '%{http_code}' \
      --cacert "$authority" --resolve "$name:$port:127.0.0.1" \
      "$@" "https://$name:$port$path" 2> "$dir/curl.err" || true
  }
  post() {
    local path="$1" body="$2"
    shift 2
    ask "$path" -X POST -H 'Content-Type: application/json' -d "$body" "$@"
  }
  request() {
    jq -cn --arg shelf "$1" --arg type "$2" --arg payload "$(printf '%s' "$3" | base64 -w0)" \
      '{shelf: $shelf, request_type: $type, payload_b64: $payload}'
  }
  # The answer to a request of a shelf, as the shelf gave it.
  said_of() { jq -r '.payload_b64' "$dir/answer" | base64 -d; }
  # Start the unit, as a demo or as a unit, and wait for it to say it is
  # up; where it does not come up, say so by the status.
  up() {
    export USHER_DEMO="$1"
    compose up -d --wait --wait-timeout 60 usher > "$dir/up.log" 2>&1
  }
  face() {
    compose cp usher:/var/lib/evo/state/https/ca.crt "$dir/unit-ca.crt" > /dev/null 2>&1 || fail "the unit that is up made no authority"
    chmod 644 "$dir/unit-ca.crt"
    tries=0
    until [ "$(ask /usher/v1/state)" = 200 ]; do
      tries=$((tries + 1))
      [ "$tries" -lt 60 ] || fail "the unit does not say what it is: $(cat "$dir/curl.err") $(head -c 200 "$dir/answer" 2> /dev/null)"
      sleep 0.5
    done
  }

  echo "prove-proxy: demo"
  up 1 || { cat "$dir/up.log" >&2; fail "a unit started as a demo does not come up"; }
  face
  jq -e '.demo == true and .set_up == false' "$dir/answer" > /dev/null || fail "a new demo says of itself: $(cat "$dir/answer")"
  logged=$(compose logs usher 2> /dev/null || true)
  grep -q 'this unit is a demo' <<< "$logged" || fail "the log of a demo does not say that it is one"
  # The page carries the line that says so, for a browser to show.
  [ "$(ask /org.evoframework.usher.shell/)" = 200 ] || fail "the shell does not come"
  grep -q 'data-demo' "$dir/answer" || fail "the shell's page has no line to say that the unit is a demo"
  [ "$(ask /satellite/)" = 200 ] || fail "the page of a satellite does not come"
  grep -q 'data-demo' "$dir/answer" || fail "a satellite's page has no line to say that the unit is a demo"
  echo "  ok    started with USHER_DEMO=1 the unit is up, and says that it is a demo: to anyone who asks, in its log, and on its pages"

  # No name and no password come with it: it is set up as any unit.
  status=$(post /usher/v1/sign_in '{"name": "admin", "password": "admin"}')
  [ "$status" != 200 ] || fail "a demo lets a name in that nobody gave it"
  status=$(post /usher/v1/set_up "$(jq -cn --arg name "$PERSON" --arg password "$PASSWORD" '{name: $name, password: $password}')")
  [ "$status" = 200 ] || fail "a demo is not set up: $status $(cat "$dir/answer")"
  bearer=$(jq -r '.token' "$dir/answer")
  # With nothing else set up, the controller is set, the network is
  # chosen and the types are on offer; and a voucher is handed out.
  status=$(post /api/v1/request "$(request access.controller info '{}')" -H "Authorization: Bearer $bearer")
  [ "$status" = 200 ] || fail "a demo does not tell of its controller: $status $(cat "$dir/answer")"
  said_of | jq -e '.set == true and .held == true and (.base | startswith("https://127.0.0.1:")) and (.version | type == "string")' > /dev/null || fail "of its controller a demo says: $(said_of)"
  status=$(post /api/v1/request "$(request issue.counter offered '{}')" -H "Authorization: Bearer $bearer")
  [ "$status" = 200 ] || fail "a demo does not tell the types on offer: $status $(cat "$dir/answer")"
  said_of | jq -e '(.types | map(.name)) == ["Day guest", "Week", "Stay"] and .default == "Day guest"' > /dev/null || fail "on offer in a demo: $(said_of)"
  status=$(post /api/v1/request "$(request issue.counter hand '{"type": "Day guest"}')" -H "Authorization: Bearer $bearer")
  [ "$status" = 200 ] || fail "a demo hands out no voucher at the counter's request: $status $(cat "$dir/answer")"
  said_of | jq -e '(.voucher.code | test("^[0-9]{10}$")) and .network == "Guests" and (.join.rows | length > 0) and .type.name == "Day guest"' > /dev/null || fail "the voucher a demo handed out: $(said_of)"
  echo "  ok    set up with a name and a password and nothing else, it hands out a voucher at the counter's request: a code, the network, and the symbol it is joined by"

  # A screen is adopted, and gives the default voucher at its button.
  status=$(post /usher/v1/satellite_code '{}')
  [ "$status" = 200 ] || fail "a screen is not given a code: $status $(cat "$dir/answer")"
  shown=$(jq -r '.code' "$dir/answer")
  secret=$(jq -r '.secret' "$dir/answer")
  status=$(post /usher/v1/adopt_satellite "$(jq -cn --arg code "$shown" '{code: $code, name: "Lobby"}')" -H "Authorization: Bearer $bearer")
  [ "$status" = 200 ] || fail "a screen is not adopted: $status $(cat "$dir/answer")"
  status=$(post /usher/v1/satellite_sign_in "$(jq -cn --arg secret "$secret" '{secret: $secret}')")
  [ "$status" = 200 ] || fail "a satellite's secret is not exchanged for a session: $status $(cat "$dir/answer")"
  screen=$(jq -r '.token' "$dir/answer")
  status=$(post /api/v1/request "$(request issue.counter hand_default '{}')" -H "Authorization: Bearer $screen")
  [ "$status" = 200 ] || fail "a satellite of a demo hands out no voucher: $status $(cat "$dir/answer")"
  said_of | jq -e '(.voucher.code | test("^[0-9]{10}$")) and .network == "Guests"' > /dev/null || fail "the voucher a satellite of a demo handed out: $(said_of)"
  echo "  ok    a screen is adopted and hands out the default voucher at its button"

  # It is given no other controller.
  status=$(post /api/v1/request "$(request access.controller set '{"base": "https://203.0.113.9/proxy/network/integration", "key": "a key"}')" -H "Authorization: Bearer $bearer")
  [ "$status" = 403 ] || fail "a demo is pointed at another controller: $status $(cat "$dir/answer")"
  jq -e '.error.details.subclass == "demo_controller"' "$dir/answer" > /dev/null || fail "another controller is refused a demo as: $(cat "$dir/answer")"
  echo "  ok    pointed at another controller, it refuses"

  # Its volume is a demo's for good: started as a unit, the program says
  # why not and ends; started as a demo again, it is the demo it was.
  compose down > /dev/null 2>&1 || fail "the demo is not stopped"
  if up ""; then
    fail "a demo's volume is started as a unit"
  fi
  logged=$(compose logs usher 2> /dev/null || true)
  grep -q 'a demo does not become a unit' <<< "$logged" || fail "started as a unit, a demo's volume does not say why not: $(compose logs --tail 5 usher 2>&1)"
  compose down > /dev/null 2>&1 || true
  up 1 || { cat "$dir/up.log" >&2; fail "a demo's volume is not started as a demo again"; }
  face
  jq -e '.demo == true and .set_up == true' "$dir/answer" > /dev/null || fail "started again, the demo says of itself: $(cat "$dir/answer")"
  status=$(post /usher/v1/sign_in "$(jq -cn --arg name "$PERSON" --arg password "$PASSWORD" '{name: $name, password: $password}')")
  [ "$status" = 200 ] || fail "started again, the demo's administrator does not sign in: $status $(cat "$dir/answer")"
  bearer=$(jq -r '.token' "$dir/answer")
  status=$(post /api/v1/request "$(request issue.counter hand '{"type": "Week"}')" -H "Authorization: Bearer $bearer")
  [ "$status" = 200 ] || fail "started again, the demo hands out no voucher: $status $(cat "$dir/answer")"
  echo "  ok    its volume is not started as a unit, and says why; started as a demo again, it is the demo it was and hands out a voucher"
  compose down --volumes --remove-orphans > /dev/null 2>&1 || true

  # And a unit's volume is a unit's for good.
  project="usher-proof-demo-unit"
  export USHER_NAME="$project"
  up "" || { cat "$dir/up.log" >&2; fail "a unit does not come up"; }
  face
  jq -e '.demo == false' "$dir/answer" > /dev/null || fail "a unit says of itself: $(cat "$dir/answer")"
  compose down > /dev/null 2>&1 || fail "the unit is not stopped"
  if up 1; then
    fail "a unit's volume is started as a demo"
  fi
  logged=$(compose logs usher 2> /dev/null || true)
  grep -q 'a unit does not become a demo' <<< "$logged" || fail "started as a demo, a unit's volume does not say why not: $(compose logs --tail 5 usher 2>&1)"
  compose down > /dev/null 2>&1 || true
  up "" || { cat "$dir/up.log" >&2; fail "a unit's volume is not started as a unit again"; }
  face
  jq -e '.demo == false' "$dir/answer" > /dev/null || fail "started again, the unit says of itself: $(cat "$dir/answer")"
  echo "  ok    a unit's volume is not started as a demo, and says why; it is the unit it was"
  compose down --volumes --remove-orphans > /dev/null 2>&1 || true
  unset USHER_DEMO
  project=""
}

# A unit moved from one volume to another by its file.
prove_move() {
  proxy=move
  local old="usher-proof-move-old" new="usher-proof-move-new"
  started="$started $old $new"
  local port dir bearer status shown secret screen tries logged began
  local name=localhost authority passphrase="five small owls sing"
  dir="$work/move"
  mkdir -p "$dir"
  authority="$dir/unit-ca.crt"
  # Both are demos, so that a voucher can be handed out with no
  # controller at hand: a demo's file is taken in by a demo.
  export USHER_ADDRESS=127.0.0.1 USHER_DEMO=1
  compose() { docker compose -p "$project" -f packaging/docker/compose.yaml "$@"; }
  ask() {
    local path="$1"
    shift
    curl -sS --max-time 30 -o "$dir/answer" -w '%{http_code}' \
      --cacert "$authority" --resolve "$name:$port:127.0.0.1" \
      "$@" "https://$name:$port$path" 2> "$dir/curl.err" || true
  }
  post() {
    local path="$1" body="$2"
    shift 2
    ask "$path" -X POST -H 'Content-Type: application/json' -d "$body" "$@"
  }
  request() {
    jq -cn --arg shelf "$1" --arg type "$2" --arg payload "$(printf '%s' "$3" | base64 -w0)" \
      '{shelf: $shelf, request_type: $type, payload_b64: $payload}'
  }
  said_of() { jq -r '.payload_b64' "$dir/answer" | base64 -d; }
  # Start the unit of a project, on a port of its own.
  up() {
    project="$1"
    port=$(free_port)
    export USHER_PORT="$port" USHER_NAME="$project"
    compose up -d --wait --wait-timeout 60 usher > "$dir/up.log" 2>&1
  }
  # The unit's authority, as it is now, and the unit's word on itself.
  face() {
    tries=0
    until compose cp usher:/var/lib/evo/state/https/ca.crt "$authority" > /dev/null 2>&1 \
      && chmod 644 "$authority" && [ "$(ask /usher/v1/state)" = 200 ] && jq -e "$1" "$dir/answer" > /dev/null; do
      tries=$((tries + 1))
      [ "$tries" -lt 120 ] || fail "the unit does not say of itself that $1: $(cat "$dir/curl.err") $(head -c 300 "$dir/answer" 2> /dev/null)"
      sleep 0.5
    done
  }

  echo "prove-proxy: move"
  up "$old" || { cat "$dir/up.log" >&2; fail "the unit that is moved does not come up"; }
  face '.set_up == false'
  status=$(post /usher/v1/set_up "$(jq -cn --arg name "$PERSON" --arg password "$PASSWORD" '{name: $name, password: $password}')")
  [ "$status" = 200 ] || fail "the unit that is moved is not set up: $status $(cat "$dir/answer")"
  bearer=$(jq -r '.token' "$dir/answer")
  status=$(post /usher/v1/satellite_code '{}')
  [ "$status" = 200 ] || fail "a screen is not given a code: $status $(cat "$dir/answer")"
  shown=$(jq -r '.code' "$dir/answer")
  secret=$(jq -r '.secret' "$dir/answer")
  status=$(post /usher/v1/adopt_satellite "$(jq -cn --arg code "$shown" '{code: $code, name: "Lobby"}')" -H "Authorization: Bearer $bearer")
  [ "$status" = 200 ] || fail "a screen is not adopted: $status $(cat "$dir/answer")"
  # Its settings alone, and everything, sealed.
  status=$(post /usher/v1/export '{}' -H "Authorization: Bearer $bearer")
  [ "$status" = 200 ] || fail "the unit's settings are not given out: $status $(cat "$dir/answer")"
  jq -e '.usher_file == 1 and (has("unit") | not) and (.settings.voucher_types.types | length) == 3' "$dir/answer" > /dev/null || fail "the unit's settings, as a file: $(head -c 300 "$dir/answer")"
  cp "$dir/answer" "$dir/profile.json"
  status=$(post /usher/v1/export "$(jq -cn --arg passphrase "$passphrase" '{everything: true, passphrase: $passphrase, password: "three small owls sang"}')" -H "Authorization: Bearer $bearer")
  [ "$status" = 403 ] || fail "everything is given out for a wrong password: $status"
  status=$(post /usher/v1/export "$(jq -cn --arg passphrase "$passphrase" --arg password "$PASSWORD" '{everything: true, passphrase: $passphrase, password: $password}')" -H "Authorization: Bearer $bearer")
  [ "$status" = 200 ] || fail "everything the unit keeps is not given out: $status $(cat "$dir/answer")"
  jq -e '.usher_file == 1 and (.unit.sealed_b64 | length) > 1000 and .unit.memory_kib == 65536 and .unit.passes == 3' "$dir/answer" > /dev/null || fail "the unit, as a file: $(head -c 300 "$dir/answer")"
  cp "$dir/answer" "$dir/unit.json"
  # Nothing of the unit's own stands in the file as it was: what is in
  # the clear names nobody.
  logged=$(jq -c '.settings' "$dir/unit.json")
  if grep -q -F "$PERSON" <<< "$logged"; then
    fail "the file names $PERSON in the clear"
  fi
  cp "$authority" "$dir/old-ca.crt"
  compose down > /dev/null 2>&1 || fail "the unit that is moved is not stopped"
  echo "  ok    a unit gives its settings as a file with no unit in it, and everything it keeps, sealed, for its administrator's password and a passphrase"

  # The machine it is moved to: a unit nobody has set up.
  up "$new" || { cat "$dir/up.log" >&2; fail "the unit that is moved to does not come up"; }
  face '.set_up == false and .demo == true'
  began=$(docker inspect -f '{{.State.StartedAt}}' "$new")
  jq -cn --slurpfile file "$dir/unit.json" '{file: $file[0], passphrase: "six small owls sing"}' > "$dir/taking.json"
  status=$(ask /usher/v1/set_up_from_file -X POST -H 'Content-Type: application/json' --data-binary "@$dir/taking.json")
  [ "$status" = 403 ] || fail "a file is taken in under a wrong passphrase: $status $(cat "$dir/answer")"
  jq -e '.error.details.subclass == "wrong_passphrase"' "$dir/answer" > /dev/null || fail "a wrong passphrase is refused as: $(cat "$dir/answer")"
  status=$(ask /usher/v1/state)
  jq -e '.set_up == false' "$dir/answer" > /dev/null || fail "a file that was refused changed the unit: $status $(cat "$dir/answer")"
  jq -cn --slurpfile file "$dir/unit.json" --arg passphrase "$passphrase" '{file: $file[0], passphrase: $passphrase}' > "$dir/taking.json"
  status=$(ask /usher/v1/set_up_from_file -X POST -H 'Content-Type: application/json' --data-binary "@$dir/taking.json")
  [ "$status" = 200 ] || fail "the unit's file is not taken in: $status $(cat "$dir/answer")"
  jq -e '.taken_in == true' "$dir/answer" > /dev/null || fail "the unit's file is answered: $(cat "$dir/answer")"
  # It starts again by itself as the unit of the file, and is believed
  # from then on by that unit's authority.
  face '.set_up == true and .demo == true'
  [ "$(docker inspect -f '{{.State.StartedAt}} {{.RestartCount}}' "$new")" = "$began 0" ] || fail "the container was started again, where the program starts again in it by itself: $(docker inspect -f '{{.State.StartedAt}} {{.RestartCount}}' "$new"), begun $began"
  [ "$(sha256sum < "$authority")" = "$(sha256sum < "$dir/old-ca.crt")" ] || fail "the unit that was moved has another authority than it had"
  logged=$(compose logs usher 2> /dev/null || true)
  grep -q 'the unit that was moved here is put in place' <<< "$logged" || fail "the log does not say that the unit was put in place: $(compose logs --tail 5 usher 2>&1)"
  echo "  ok    a wrong passphrase is refused and changes nothing; with its passphrase the file is taken in, and the unit starts again by itself, in its container, under the authority of the unit that was moved"

  # It is the unit that was moved: its administrator signs in, and the
  # screen that was adopted hands out a voucher with the secret it has.
  status=$(post /usher/v1/sign_in "$(jq -cn --arg name "$PERSON" --arg password "$PASSWORD" '{name: $name, password: $password}')")
  [ "$status" = 200 ] || fail "the administrator of the unit that was moved does not sign in: $status $(cat "$dir/answer")"
  status=$(ask /usher/v1/me -H "Authorization: Bearer $bearer")
  [ "$status" = 401 ] || fail "a session of the machine it was moved from is answered: $status"
  bearer=$(jq -r '.token' "$dir/answer")
  status=$(post /usher/v1/satellite_sign_in "$(jq -cn --arg secret "$secret" '{secret: $secret}')")
  [ "$status" = 200 ] || fail "the secret of a satellite of the unit that was moved is not exchanged: $status $(cat "$dir/answer")"
  jq -e '.adopted == true and .name == "Lobby"' "$dir/answer" > /dev/null || fail "a satellite of the unit that was moved is answered: $(cat "$dir/answer")"
  screen=$(jq -r '.token' "$dir/answer")
  status=$(post /api/v1/request "$(request issue.counter hand_default '{}')" -H "Authorization: Bearer $screen")
  [ "$status" = 200 ] || fail "a satellite of the unit that was moved hands out no voucher: $status $(cat "$dir/answer")"
  said_of | jq -e '(.voucher.code | test("^[0-9]{10}$")) and .network == "Guests"' > /dev/null || fail "the voucher a satellite handed out after the move: $(said_of)"
  echo "  ok    its administrator signs in with the password they had, and a screen that was adopted before the move hands out a voucher after it, with the secret it holds"

  # And the settings, as a profile, are applied to it as to any unit.
  status=$(post /usher/v1/sign_in "$(jq -cn --arg name "$PERSON" --arg password "$PASSWORD" '{name: $name, password: $password}')")
  bearer=$(jq -r '.token' "$dir/answer")
  jq -cn --slurpfile file "$dir/profile.json" '{file: $file[0]}' > "$dir/applying.json"
  status=$(ask /usher/v1/apply_profile -X POST -H 'Content-Type: application/json' -H "Authorization: Bearer $bearer" --data-binary "@$dir/applying.json")
  [ "$status" = 200 ] || fail "a profile is not applied: $status $(cat "$dir/answer")"
  jq -e '(.applied.types | length) == 3 and (.applied.refused | length) == 0' "$dir/answer" > /dev/null || fail "of a profile that was applied the unit says: $(cat "$dir/answer")"
  echo "  ok    the settings that were given out as a profile are applied, and nothing of them is refused"
  compose down --volumes --remove-orphans > /dev/null 2>&1 || true
  project="$old"
  compose down --volumes --remove-orphans > /dev/null 2>&1 || true
  unset USHER_DEMO
  project=""
}

# A unit that ran the newest release, updated to this version and put
# back to that release, on the volume it has.
prove_update() {
  proxy=update
  project="usher-proof-update"
  started="$started $project"
  local port dir bearer status shown secret screen tries pin first second third
  local name=localhost authority
  local from=${USHER_FROM:-ghcr.io/foonerd/unifi-evo-usher:latest}
  local here="$USHER_IMAGE:$USHER_VERSION" released
  local satellites_pin="48027316"
  dir="$work/update"
  mkdir -p "$dir"
  authority="$dir/unit-ca.crt"
  port=$(free_port)
  # A demo, so that a voucher can be handed out with no controller at
  # hand: what a unit keeps is kept by a demo in the same files.
  export USHER_ADDRESS=127.0.0.1 USHER_PORT="$port" USHER_NAME="$project" USHER_DEMO=1
  compose() { docker compose -p "$project" -f packaging/docker/compose.yaml "$@"; }
  ask() {
    local path="$1"
    shift
    curl -sS --max-time 30 -o "$dir/answer" -w '%{http_code}' \
      --cacert "$authority" --resolve "$name:$port:127.0.0.1" \
      "$@" "https://$name:$port$path" 2> "$dir/curl.err" || true
  }
  post() {
    local path="$1" body="$2"
    shift 2
    ask "$path" -X POST -H 'Content-Type: application/json' -d "$body" "$@"
  }
  request() {
    jq -cn --arg shelf "$1" --arg type "$2" --arg payload "$(printf '%s' "$3" | base64 -w0)" \
      '{shelf: $shelf, request_type: $type, payload_b64: $payload}'
  }
  said_of() { jq -r '.payload_b64' "$dir/answer" | base64 -d; }
  person() { jq -cn --arg name "$1" --arg password "$2" '{name: $name, password: $password}'; }
  # Start the unit from an image, named as <image>:<tag>, on the volume
  # it has, and see that it is that image the unit then runs.
  up() {
    USHER_IMAGE="${1%:*}" USHER_VERSION="${1##*:}" \
      docker compose -p "$project" -f packaging/docker/compose.yaml up -d --wait --wait-timeout 60 usher > "$dir/up.log" 2>&1 || return 1
    [ "$(docker inspect -f '{{.Config.Image}}' "$project")" = "$1" ]
  }
  # The unit's authority, as it is now, and the unit's word on itself.
  face() {
    tries=0
    until compose cp usher:/var/lib/evo/state/https/ca.crt "$authority" > /dev/null 2>&1 \
      && chmod 644 "$authority" && [ "$(ask /usher/v1/state)" = 200 ] && jq -e "$1" "$dir/answer" > /dev/null; do
      tries=$((tries + 1))
      [ "$tries" -lt 120 ] || fail "the unit does not say of itself that $1: $(cat "$dir/curl.err") $(head -c 300 "$dir/answer" 2> /dev/null)"
      sleep 0.5
    done
  }
  # The unit as it was set up, whichever version runs it: asked as its
  # people and its screen ask it, and one voucher more handed out by
  # the screen, whose id is printed.
  #
  #   as_it_was <which start this is, for what is said where it fails>
  as_it_was() {
    local when="$1" desk handed
    [ "$(sha256sum < "$authority")" = "$(sha256sum < "$dir/first-ca.crt")" ] || fail "$when, the unit has another authority than it had"
    status=$(post /usher/v1/sign_in "$(person "$PERSON" "$PASSWORD")")
    [ "$status" = 200 ] || fail "$when, the administrator does not sign in with the password they had: $status $(cat "$dir/answer")"
    jq -e '.role == "administrator"' "$dir/answer" > /dev/null || fail "$when, the administrator signs in as: $(cat "$dir/answer")"
    bearer=$(jq -r '.token' "$dir/answer")
    status=$(post /usher/v1/sign_in "$(person Ben "$pin")")
    [ "$status" = 200 ] || fail "$when, the front desk does not sign in with the PIN it had: $status $(cat "$dir/answer")"
    jq -e '.role == "front_desk"' "$dir/answer" > /dev/null || fail "$when, the front desk signs in as: $(cat "$dir/answer")"
    desk=$(jq -r '.token' "$dir/answer")
    status=$(post /api/v1/request "$(request issue.counter offered '{}')" -H "Authorization: Bearer $desk")
    [ "$status" = 200 ] || fail "$when, the front desk is not told the types on offer: $status $(cat "$dir/answer")"
    said_of | jq -e '(.types | map(.name)) == ["Day guest", "Week", "Stay", "Contractor"] and .default == "Day guest"' > /dev/null || fail "$when, on offer: $(said_of)"
    status=$(post /api/v1/request "$(request issue.counter types '{}')" -H "Authorization: Bearer $bearer")
    [ "$status" = 200 ] || fail "$when, the administrator is not told the types: $status $(cat "$dir/answer")"
    said_of | jq -e '.types | map(select(.name == "Contractor")) | length == 1 and .[0].minutes == 10080 and .[0].guests == 2' > /dev/null || fail "$when, the type of the site's own: $(said_of)"
    [ "$(ask /usher/v1/state)" = 200 ] || fail "$when, the unit does not say what it is"
    jq -e --arg name "$SET_NAME" '.site.name == $name and .set_up == true and .demo == true' "$dir/answer" > /dev/null || fail "$when, the unit says of itself: $(cat "$dir/answer")"
    status=$(ask /usher/v1/satellites -H "Authorization: Bearer $bearer")
    [ "$status" = 200 ] || fail "$when, the satellites are not listed: $status $(cat "$dir/answer")"
    jq -e '.satellites | length == 1 and .[0].name == "Lobby" and .[0].mode == "protected"' "$dir/answer" > /dev/null || fail "$when, the satellites: $(cat "$dir/answer")"
    status=$(post /usher/v1/satellite_sign_in "$(jq -cn --arg secret "$secret" '{secret: $secret}')")
    [ "$status" = 200 ] || fail "$when, the screen's secret is not exchanged for a session: $status $(cat "$dir/answer")"
    screen=$(jq -r '.token' "$dir/answer")
    status=$(post /api/v1/request "$(request issue.counter hand_default '{}')" -H "Authorization: Bearer $screen")
    [ "$status" = 403 ] || fail "$when, the protected screen is given a voucher with no PIN: $status $(cat "$dir/answer")"
    status=$(post /api/v1/request "$(request issue.counter hand_default "$(jq -cn --arg pin "$satellites_pin" '{pin: $pin}')")" -H "Authorization: Bearer $screen")
    [ "$status" = 200 ] || fail "$when, the screen hands out no voucher with the PIN it had: $status $(cat "$dir/answer")"
    said_of | jq -e '(.voucher.code | test("^[0-9]{10}$")) and .network == "Guests"' > /dev/null || fail "$when, the voucher the screen handed out: $(said_of)"
    handed=$(said_of | jq -r '.voucher.id')
    echo "$handed"
  }
  # The record of hand-outs names each of these vouchers.
  recorded() {
    local when="$1" ids
    shift
    ids=$(jq -cn '$ARGS.positional' --args "$@")
    status=$(post /api/v1/request "$(request issue.counter handed "$(jq -cn --argjson ids "$ids" '{ids: $ids}')")" -H "Authorization: Bearer $bearer")
    [ "$status" = 200 ] || fail "$when, the record of hand-outs is not told: $status $(cat "$dir/answer")"
    said_of | jq -e --argjson ids "$ids" '(.handed | map(.id) | sort) == ($ids | sort)' > /dev/null || fail "$when, the record of hand-outs does not name $ids: $(said_of)"
  }

  echo "prove-proxy: update"
  if ! docker image inspect "$from" > /dev/null 2>&1; then
    docker pull "$from" > "$dir/pull.log" 2>&1 || { cat "$dir/pull.log" >&2; fail "the release to update from, $from, is not on this machine and is not fetched"; }
  fi
  released=$(docker image inspect -f '{{index .Config.Labels "org.opencontainers.image.version"}}' "$from")
  [ -n "$released" ] || fail "$from does not say which version it is"

  # The unit on the release, set up as a site sets it up.
  up "$from" || { cat "$dir/up.log" >&2; fail "a unit of the release $released does not come up"; }
  face '.set_up == false and .demo == true'
  cp "$authority" "$dir/first-ca.crt"
  status=$(post /usher/v1/set_up "$(person "$PERSON" "$PASSWORD")")
  [ "$status" = 200 ] || fail "the unit of the release is not set up: $status $(cat "$dir/answer")"
  bearer=$(jq -r '.token' "$dir/answer")
  status=$(post /usher/v1/add_person '{"name": "Ben", "role": "front_desk"}' -H "Authorization: Bearer $bearer")
  [ "$status" = 200 ] || fail "a person is not added for the front desk: $status $(cat "$dir/answer")"
  pin=$(jq -r '.pin' "$dir/answer")
  status=$(post /api/v1/request "$(request issue.counter put_type '{"name": "Contractor", "minutes": 10080, "guests": 2}')" -H "Authorization: Bearer $bearer")
  [ "$status" = 200 ] || fail "a voucher type is not added: $status $(cat "$dir/answer")"
  status=$(post /api/v1/request "$(request site.settings get '{}')" -H "Authorization: Bearer $bearer")
  [ "$status" = 200 ] || fail "the site is not told: $status $(cat "$dir/answer")"
  status=$(post /api/v1/request "$(request site.settings set "$(said_of | jq -c --arg name "$SET_NAME" '{name: $name, time_zone: .time_zone, language: .language}')")" -H "Authorization: Bearer $bearer")
  [ "$status" = 200 ] || fail "the site is not given a name: $status $(cat "$dir/answer")"
  status=$(post /usher/v1/satellite_code '{}')
  [ "$status" = 200 ] || fail "a screen is not given a code: $status $(cat "$dir/answer")"
  shown=$(jq -r '.code' "$dir/answer")
  secret=$(jq -r '.secret' "$dir/answer")
  status=$(post /usher/v1/adopt_satellite "$(jq -cn --arg code "$shown" --arg pin "$satellites_pin" '{code: $code, name: "Lobby", mode: "protected", pin: $pin}')" -H "Authorization: Bearer $bearer")
  [ "$status" = 200 ] || fail "a screen is not adopted, protected: $status $(cat "$dir/answer")"
  first=$(as_it_was "on the release $released")
  status=$(post /usher/v1/sign_in "$(person "$PERSON" "$PASSWORD")")
  bearer=$(jq -r '.token' "$dir/answer")
  recorded "on the release $released" "$first"
  echo "  ok    a unit of the release $released is set up: a person at the front desk, a voucher type and a name of the site's own, and a screen protected by a PIN, which hands out a voucher"

  # The update: this version, on the volume the unit has.
  up "$here" || { cat "$dir/up.log" >&2; fail "updated to $USHER_VERSION, the unit does not come up"; }
  face '.set_up == true and .demo == true'
  second=$(as_it_was "updated to $USHER_VERSION")
  status=$(post /usher/v1/sign_in "$(person "$PERSON" "$PASSWORD")")
  bearer=$(jq -r '.token' "$dir/answer")
  status=$(ask /usher/v1/me -H "Authorization: Bearer $bearer")
  [ "$status" = 200 ] || fail "updated, the administrator is not told who they are: $status $(cat "$dir/answer")"
  jq -e --arg version "$USHER_VERSION" '.runs.version == $version' "$dir/answer" > /dev/null || fail "updated, the unit does not tell its administrator that it runs $USHER_VERSION: $(cat "$dir/answer")"
  recorded "updated to $USHER_VERSION" "$first" "$second"
  echo "  ok    updated to $USHER_VERSION on the volume it has, the unit is as it was: its people sign in with what they had, its types and its site stand, the screen hands out a voucher with its secret and its PIN, the authority is the same, and the record names the hand-outs of both versions"

  # The way back: the release again, on the same volume.
  up "$from" || { cat "$dir/up.log" >&2; fail "put back to the release $released, the unit does not come up"; }
  face '.set_up == true and .demo == true'
  third=$(as_it_was "put back to the release $released")
  status=$(post /usher/v1/sign_in "$(person "$PERSON" "$PASSWORD")")
  bearer=$(jq -r '.token' "$dir/answer")
  recorded "put back to the release $released" "$first" "$second" "$third"
  echo "  ok    put back to the release $released on the same volume, it is as it was once more, and keeps what was handed out while it ran $USHER_VERSION"
  compose down --volumes --remove-orphans > /dev/null 2>&1 || true
  unset USHER_DEMO
  project=""
}

# A satellite as a service, as packaging/docker/satellite.yaml has it,
# beside a unit: both on a network of their own, as on a macvlan of a
# site, each at its address. The satellite is adopted by the code it
# shows, is that satellite for whoever opens its address on HTTP and on
# HTTPS, hands out a voucher, and passes nothing else of the unit on.
prove_satellite() {
  proxy=satellite
  project="usher-proof-satellite"
  local beside="usher-proof-satellite-beside" lan="usher-proof-lan"
  started="$started $project $beside"
  local port dir bearer status code tries name=localhost authority
  dir="$work/satellite"
  mkdir -p "$dir"
  authority="$dir/unit-ca.crt"
  port=$(free_port)
  export USHER_ADDRESS=127.0.0.1 USHER_PORT="$port" USHER_NAME="$project" USHER_DEMO=1
  compose() { docker compose -p "$project" -f packaging/docker/compose.yaml "$@"; }
  ask() {
    local path="$1"
    shift
    curl -sS --max-time 30 -o "$dir/answer" -w '%{http_code}' \
      --cacert "$authority" --resolve "$name:$port:127.0.0.1" \
      "$@" "https://$name:$port$path" 2> "$dir/curl.err" || true
  }
  post() {
    local path="$1" body="$2"
    shift 2
    ask "$path" -X POST -H 'Content-Type: application/json' -d "$body" "$@"
  }
  # The satellite, asked as a browser asks it: over HTTP, or over HTTPS
  # with its own certificate, which no browser was told of.
  at() {
    local how="$1" path="$2"
    shift 2
    curl -sS --max-time 60 -k -o "$dir/seen" -w '%{http_code}' "$@" "$how://172.31.250.11$path" 2> "$dir/curl.err" || true
  }
  request() {
    jq -cn --arg shelf "$1" --arg type "$2" --arg payload "$(printf '%s' "$3" | base64 -w0)" \
      '{shelf: $shelf, request_type: $type, payload_b64: $payload}'
  }
  person() { jq -cn --arg name "$1" --arg password "$2" '{name: $name, password: $password}'; }

  compose up -d --wait --wait-timeout 60 usher > "$dir/up.log" 2>&1 || { cat "$dir/up.log" >&2; fail "the unit does not come up"; }
  tries=0
  until compose cp usher:/var/lib/evo/state/https/ca.crt "$authority" > /dev/null 2>&1 \
    && chmod 644 "$authority" && [ "$(ask /usher/v1/state)" = 200 ]; do
    tries=$((tries + 1))
    [ "$tries" -lt 120 ] || fail "the unit does not answer: $(cat "$dir/curl.err")"
    sleep 0.5
  done
  status=$(post /usher/v1/set_up "$(person "$PERSON" "$PASSWORD")")
  [ "$status" = 200 ] || fail "the unit is not set up: $status $(cat "$dir/answer")"
  bearer=$(jq -r '.token' "$dir/answer")

  # The network of the site, and the unit on it at its address.
  docker network rm "$lan" > /dev/null 2>&1 || true
  docker network create --subnet 172.31.250.0/24 "$lan" > /dev/null || fail "the network is not made"
  docker network connect --ip 172.31.250.10 "$lan" "$project" || fail "the unit does not join the network"
  SATELLITE_NAME="$beside" SATELLITE_ADDRESS=172.31.250.11 SATELLITE_NETWORK="$lan" \
    USHER_UNIT=https://172.31.250.10:8443 STATION_NAMES=satellite.example \
    docker compose -p "$beside" --project-directory "$dir" -f packaging/docker/satellite.yaml \
    up -d --wait --wait-timeout 60 > "$dir/satellite-up.log" 2>&1 \
    || { cat "$dir/satellite-up.log" >&2; docker logs "$beside" >&2 2>&1 || true; fail "the satellite does not come up, or does not say that it is healthy"; }
  [ "$(docker inspect -f '{{.Config.User}} {{.HostConfig.ReadonlyRootfs}}' "$beside")" = "10001:10001 true" ] \
    || fail "the satellite runs as $(docker inspect -f '{{.Config.User}} {{.HostConfig.ReadonlyRootfs}}' "$beside")"
  tries=0
  until docker logs "$beside" 2>&1 | grep -E -o 'with the code [0-9]{6}' > "$dir/code"; do
    tries=$((tries + 1))
    [ "$tries" -lt 60 ] || { docker logs "$beside" >&2 2>&1; fail "the satellite does not show a code to be adopted by"; }
    sleep 0.5
  done
  code=$(tail -n 1 "$dir/code" | grep -E -o '[0-9]{6}')

  for how in http https; do
    status=$(at "$how" /satellite/)
    [ "$status" = 200 ] && grep -q satellite.js "$dir/seen" || fail "the satellite's page over $how: $status"
  done
  status=$(at http /usher/v1/satellite_sign_in -X POST -H 'Content-Type: application/json' -d '{"secret":"station"}')
  [ "$status" = 200 ] && jq -e --arg code "$code" '.adopted == false and .code == $code' "$dir/seen" > /dev/null \
    || fail "the satellite does not show its code: $status $(cat "$dir/seen")"
  status=$(post /usher/v1/adopt_satellite "$(jq -cn --arg code "$code" '{code: $code, name: "Hall"}')" -H "Authorization: Bearer $bearer")
  [ "$status" = 200 ] || fail "the satellite is not adopted by its code: $status $(cat "$dir/answer")"
  for how in http https; do
    status=$(at "$how" /usher/v1/satellite_sign_in -X POST -H 'Content-Type: application/json' -d '{"secret":"station"}')
    [ "$status" = 200 ] && jq -e '.adopted == true and .token == "station" and .name == "Hall"' "$dir/seen" > /dev/null \
      || fail "the satellite over $how is not the one adopted: $status $(cat "$dir/seen")"
    status=$(at "$how" /api/v1/request -X POST -H 'Content-Type: application/json' -H 'Authorization: Bearer station' -d "$(request issue.counter hand_default '{}')")
    [ "$status" = 200 ] && jq -r '.payload_b64' "$dir/seen" | base64 -d | jq -e '.voucher.code | test("^[0-9]{10}$")' > /dev/null \
      || fail "the satellite over $how hands out no voucher: $status $(cat "$dir/seen")"
  done
  for path in /usher/v1/me /usher/v1/satellites /index.html; do
    status=$(at http "$path")
    [ "$status" = 404 ] || fail "the satellite passes $path on: $status"
  done
  echo "  ok    a satellite as a service, at an address of its own beside the unit, is adopted by the code in its log and hands out a voucher to whoever opens it, on HTTP and on HTTPS, and passes nothing else of the unit on"

  docker compose -p "$beside" --project-directory "$dir" -f packaging/docker/satellite.yaml down --volumes > /dev/null 2>&1 || true
  docker network disconnect "$lan" "$project" > /dev/null 2>&1 || true
  compose down --volumes --remove-orphans > /dev/null 2>&1 || true
  tries=0
  until docker network rm "$lan" > /dev/null 2>&1 || [ "$tries" -ge 20 ]; do
    tries=$((tries + 1))
    sleep 0.5
  done
  unset USHER_DEMO
  project=""
}

for proxy in $proxies; do
  case "$proxy" in
    demo) prove_demo ;;
    move) prove_move ;;
    update) prove_update ;;
    satellite) prove_satellite ;;
    *) prove "$proxy" ;;
  esac
done
echo "[OK] prove-proxy: $proxies"
