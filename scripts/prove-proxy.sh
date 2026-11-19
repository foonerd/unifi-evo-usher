#!/bin/bash
# Proof that the unit runs in a container as packaging/docker/compose.yaml
# has it, and that the examples under packaging/proxy/ put it behind a
# proxy, as docs/deploying.md says: each is started as it stands in the
# repository and asked as a browser asks.
#
#   scripts/prove-proxy.sh [<case>...]
#
# <case> is alone, nginx, apache, caddy or traefik; with none named, all
# five. The usher's image is unifi-evo-usher:<version> on this machine, as
# scripts/image.sh --here keeps it, or $USHER_IMAGE:$USHER_VERSION.
#
# Alone: the unit is started by itself, as the image's account, with no
# capability, no way to gain a privilege and a root file system it cannot
# write to, and is asked on its own port, believed by its own authority.
#
# For each proxy: the unit is started and its authority taken from it;
# the proxy is started with a certificate of an authority made for the
# run, which stands in for the site's; then, through the proxy and
# believing nothing but that authority, the face is loaded, a caller who
# presents nothing is refused, a browser is paired with the unit's code,
# and with the bearer the pairing gave it is answered and a happening
# arrives over the WebSocket. Last, the
# proxy is given another authority than the unit's and must then pass
# nothing on: it believes the unit by the unit's authority and by nothing
# else.
#
# It needs docker with compose, openssl, curl, jq and python3, publishes
# on the loopback address alone, and takes away what it started.
set -euo pipefail
ROOT=$(CDPATH='' cd -- "$(dirname "$0")/.." && pwd)
cd "$ROOT"
. scripts/targets.sh

# The name the examples give the usher.
NAME=usher.example.org

proxies=${*:-alone nginx apache caddy traefik}
# The Compose file of a case.
file_of() {
  if [ "$1" = alone ]; then echo packaging/docker/compose.yaml; else echo "packaging/proxy/$1/compose.yaml"; fi
}
for proxy in $proxies; do
  [ -f "$(file_of "$proxy")" ] || { echo "prove-proxy: there is no example for '$proxy' under packaging/proxy/" >&2; exit 1; }
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
  SITE_CERT=/site.crt SITE_KEY=/site.key USHER_AUTHORITY=/unit-ca.crt USHER_PAIRING_CODE=/pairing-code \
    docker compose -f "$(file_of "$1")" config --format json | jq -S '.services.usher | del(.ports)'
}
for proxy in $proxies; do
  [ "$(unit_of "$proxy")" = "$(unit_of alone)" ] || { echo "prove-proxy: the unit in $(file_of "$proxy") is not the unit of packaging/docker/compose.yaml" >&2; exit 1; }
done

work=$(mktemp -d)
started=""
tidy() {
  local project
  for project in $started; do
    docker compose -p "$project" down --volumes --remove-orphans > /dev/null 2>&1 || true
  done
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
answered = head.decode(errors="replace").lower()
if "sec-websocket-protocol: evo.bearer." not in answered:
    sys.exit("the proxy did not pass the subprotocol on")
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
        print(json.dumps(happened), flush=True)
        sys.exit(0)
sys.exit("no happening of the type came in time")
PY
}

prove() {
  proxy=$1
  project="usher-proof-$proxy"
  started="$started $project"
  local port dir code bearer status types said
  port=$(free_port)
  dir="$work/$proxy"
  mkdir -p "$dir"
  (umask 077 && od -An -N6 -tx1 /dev/urandom | tr -d ' \n' > "$dir/pairing-code")
  # The unit reads it as the account 10001.
  chmod 644 "$dir/pairing-code"
  export USHER_PAIRING_CODE="$dir/pairing-code" USHER_AUTHORITY="$dir/unit-ca.crt"
  export SITE_CERT="$work/site.crt" SITE_KEY="$work/site.key" SITE_ADDRESS=127.0.0.1 SITE_PORT="$port"
  export USHER_ADDRESS=127.0.0.1 USHER_PORT="$port"
  # Whom the caller believes, and under which name: the site's authority
  # through a proxy; the unit's own, which names it localhost, alone.
  local name="$NAME" authority="$work/site-ca.crt"
  if [ "$proxy" = alone ]; then
    name=localhost
    authority="$dir/unit-ca.crt"
  fi
  compose() { docker compose -p "$project" -f "$(file_of "$proxy")" "$@"; }
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
  compose up -d usher > "$dir/up.log" 2>&1 || { cat "$dir/up.log" >&2; fail "the unit was not started"; }
  # The unit makes its authority when it first starts.
  local tries=0
  until compose cp usher:/var/lib/evo/state/https/ca.crt "$dir/unit-ca.crt" > /dev/null 2>&1; do
    tries=$((tries + 1))
    [ "$tries" -lt 60 ] || fail "the unit made no authority"
    sleep 0.5
  done
  chmod 644 "$dir/unit-ca.crt"
  if [ "$proxy" = alone ]; then
    said=$(docker inspect "$(compose ps -q usher)" --format '{{.Config.User}} {{.HostConfig.ReadonlyRootfs}} {{.HostConfig.CapDrop}} {{.HostConfig.SecurityOpt}}')
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
  if [ "$proxy" = alone ]; then
    echo "  ok    the unit runs as its own account, with no capability, no privilege to gain and a root it cannot write to"
    echo "  ok    the face comes, under the certificate of the unit's own authority"
  else
    echo "  ok    the face comes through, under the site's certificate"
  fi

  # Without a bearer a caller is not let in. Everything after this is
  # asked with the bearer the pairing gives.
  types=$(request issue.counter types '{}')
  status=$(post /api/v1/request "$types")
  [ "$status" = 401 ] || fail "a caller who presents nothing was answered $status: $(cat "$dir/answer")"
  code=$(cat "$dir/pairing-code")
  status=$(post /api/v1/pair_complete "$(jq -cn --arg code "$code" '{pair_id: "bootstrap", code: $code}')")
  [ "$status" = 200 ] || fail "the unit's code pairs no browser through the proxy: $status $(cat "$dir/answer")"
  bearer=$(jq -r '.token' "$dir/answer")
  status=$(post /api/v1/request "$types" -H "Authorization: Bearer $bearer")
  [ "$status" = 200 ] || fail "a paired browser is refused through the proxy: $status $(cat "$dir/answer")"
  said=$(jq -r '.payload_b64' "$dir/answer" | base64 -d)
  [ "$(printf '%s' "$said" | jq -c '.types')" = '[]' ] || fail "the counter answered $said"
  echo "  ok    a caller who presents nothing is refused; a browser pairs and is answered"

  # A happening, made while the line is open, by the paired browser:
  # evo's household level, which on an usher protects nothing.
  rm -f "$dir/line"
  mkfifo "$dir/line"
  happening "$name" "$port" "$authority" "$bearer" household_protection_changed > "$dir/line" 2> "$dir/line.err" &
  local listener=$!
  {
    read -r said || said=""
    [ "$said" = asked ] || { wait "$listener" || true; fail "the WebSocket does not come through the proxy: $(cat "$dir/line.err")"; }
    status=$(post /api/v1/household_protection_set '{"level": "open", "lend": false}' -H "Authorization: Bearer $bearer")
    [ "$status" = 200 ] || fail "nothing was made to happen: $status"
    read -r said || said=""
  } < "$dir/line"
  wait "$listener" || fail "no happening came through the proxy: $(cat "$dir/line.err")"
  [ "$(printf '%s' "$said" | jq -r '.type')" = household_protection_changed ] || fail "what came over the WebSocket is no happening: $said"
  echo "  ok    a happening arrives over the WebSocket, joined with the bearer"

  if [ "$proxy" = alone ]; then
    # What the unit kept is there when it is started again.
    compose restart usher >> "$dir/up.log" 2>&1 || fail "the unit was not started again"
    tries=0
    until [ "$(post /api/v1/request "$types" -H "Authorization: Bearer $bearer")" = 200 ]; do
      tries=$((tries + 1))
      [ "$tries" -lt 60 ] || fail "the paired browser is refused after a restart: $(cat "$dir/answer")"
      sleep 0.5
    done
    echo "  ok    started again, the unit still answers the paired browser"
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

for proxy in $proxies; do
  prove "$proxy"
done
echo "[OK] prove-proxy: $proxies"
