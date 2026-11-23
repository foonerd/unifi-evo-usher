#!/bin/bash
# The check of a unit before a site goes live with it: the unit is asked,
# as an administrator's browser asks it, whether everything a guest's
# voucher takes is in place, and one voucher is made on the site's own
# UniFi controller, found there, and revoked again.
#
#   scripts/go-live.sh [--cacert <file> | --insecure] [--no-voucher] https://<the usher>[:<port>]
#
# It asks for an administrator's name and password on the terminal, and
# does not show the password. Where there is no terminal it takes them
# from USHER_NAME and USHER_PASSWORD in its environment.
#
# The unit is believed as a browser believes it: by the certificate the
# site's proxy shows, which the machine's own authorities vouch for.
# A unit with no proxy before it is asked on its own machine, as
# https://localhost:<port>, and believed by its own authority, named
# with --cacert (/var/lib/evo/state/https/ca.crt on the unit): its own
# certificate names it localhost and 127.0.0.1 and nothing else. With
# --insecure nobody is asked who the unit is, which the check says in
# what it prints: for a first look, on a network that is trusted.
#
# It changes nothing but this: one voucher of the default type is handed
# out in the administrator's name and revoked at once, so that the whole
# way to the controller and back is walked. The record of hand-outs
# keeps that it was. With --no-voucher the way is not walked, and the
# check says so.
#
# It needs curl, jq and base64, and ends with 0 where nothing failed.
set -euo pipefail

believed=()
voucher=yes
unit=""
while [ $# -gt 0 ]; do
  case "$1" in
    --cacert) believed=(--cacert "${2:?--cacert names a file}"); shift 2 ;;
    --insecure) believed=(--insecure); shift ;;
    --no-voucher) voucher=no; shift ;;
    https://*) unit=${1%/}; shift ;;
    *) echo "go-live: '$1' is neither an option nor the unit's address, as https://<the usher>" >&2; exit 2 ;;
  esac
done
[ -n "$unit" ] || { echo "go-live: name the unit, as https://<the usher>[:<port>]" >&2; exit 2; }
for tool in curl jq base64; do
  command -v "$tool" > /dev/null || { echo "go-live: $tool is not on this machine" >&2; exit 2; }
done

# What the check keeps while it runs, the session among it, is kept in a
# directory nobody else on this machine reads, and is gone when it ends.
# Neither the password nor the session is put on a command line, which
# anyone on the machine can read.
work=$(mktemp -d)
chmod 700 "$work"
ended() {
  # Whoever was signed in for the check is signed out, however it ends.
  if [ -s "$work/session" ]; then
    curl --silent --max-time 20 ${believed[@]+"${believed[@]}"} -o /dev/null -X POST -H @"$work/session" -H 'Content-Type: application/json' -d '{}' "$unit/usher/v1/sign_out" || true
  fi
  rm -rf "$work"
}
trap ended EXIT

failed=0
ok() { echo "  ok    $1"; }
note() { echo "  note  $1"; }
fail() { echo "  FAIL  $1"; failed=$((failed + 1)); }

# Ask the unit: the status is printed, the answer is kept.
ask() {
  local path=$1
  shift
  curl --silent --max-time 40 ${believed[@]+"${believed[@]}"} -o "$work/answer" -w '%{http_code}' "$@" "$unit$path" || true
}
signed() { ask "$1" -H @"$work/session" "${@:2}"; }
# Ask a shelf of the unit, and keep what it answers.
shelf() {
  local body status
  body=$(jq -cn --arg shelf "$1" --arg type "$2" --arg payload "$(printf '%s' "${3:-{\}}" | base64 | tr -d '\n')" \
    '{shelf: $shelf, request_type: $type, payload_b64: $payload}')
  status=$(signed /api/v1/request -X POST -H 'Content-Type: application/json' -d "$body")
  if [ "$status" = 200 ]; then
    jq -r '.payload_b64' "$work/answer" | base64 -d > "$work/said"
  else
    cp "$work/answer" "$work/said"
  fi
  echo "$status"
}
why() { jq -r '(.error.details.subclass // "") + " " + (.error.message // "")' "$work/said" 2> /dev/null || true; }

echo "go-live: $unit"
[ "${believed[0]:-}" != --insecure ] || note "nobody is asked who the unit is (--insecure): the answers are the unit's only if the network between is trusted"

# 1. The unit answers, and is set up.
status=$(ask /usher/v1/state)
if [ "$status" != 200 ]; then
  fail "the unit does not answer at $unit/usher/v1/state (status $status): the address, the port, the certificate it is believed by"
  echo "go-live: 1 failed; nothing else was asked"
  exit 1
fi
cp "$work/answer" "$work/state"
ok "the unit answers"
if [ "$(jq -r '.demo // false' "$work/state")" = true ]; then
  fail "this unit is a demo: its controller is simulated and its vouchers open no network; a site goes live on a unit that is none"
fi
if [ "$(jq -r '.people_unread // false' "$work/state")" = true ]; then
  fail "the unit cannot read the list of its people, and lets nobody in until it can"
  echo "go-live: $failed failed; nothing else was asked"
  exit 1
fi
if [ "$(jq -r '.set_up' "$work/state")" = true ]; then
  ok "it is set up"
else
  fail "nobody has set this unit up yet: whoever opens it in a browser first does, with a name and a password of their own"
  echo "go-live: $failed failed; nothing else was asked"
  exit 1
fi
note "it runs at the $(jq -r '.level' "$work/state") level, and its forms speak '$(jq -r '.site.language' "$work/state")'"
if [ "$(jq -r '.site.name' "$work/state")" = "" ]; then
  note "the site has no name yet: the page Site gives it one, which a guest reads on the voucher"
else
  note "the site is called '$(jq -r '.site.name' "$work/state")'"
fi

# 2. An administrator signs in.
if [ -t 0 ]; then
  read -r -p "An administrator's name: " name
  read -r -s -p "Their password: " password
  echo
else
  name=${USHER_NAME:?with no terminal, USHER_NAME names the administrator}
  password=${USHER_PASSWORD:?with no terminal, USHER_PASSWORD is their password}
fi
jq -Rcn --arg name "$name" '{name: $name, password: input}' <<< "$password" > "$work/asked"
unset password
status=$(ask /usher/v1/sign_in -X POST -H 'Content-Type: application/json' --data-binary @"$work/asked")
rm -f "$work/asked"
if [ "$status" != 200 ]; then
  fail "signing in is refused (status $status): $(jq -r '.error.message // empty' "$work/answer")"
  echo "go-live: $failed failed; nothing else was asked"
  exit 1
fi
jq -r '"Authorization: Bearer " + .token' "$work/answer" > "$work/session"
if [ "$(jq -r '.role' "$work/answer")" = administrator ]; then
  ok "$name signs in, and is an administrator"
else
  fail "$name is no administrator, and the check asks what an administrator asks"
  echo "go-live: $failed failed; nothing else was asked"
  exit 1
fi
# Which usher the unit runs, where it says so: one before 0.3.25 does not.
status=$(signed /usher/v1/me)
if [ "$status" = 200 ] && [ "$(jq -r '.runs.version // empty' "$work/answer")" != "" ]; then
  note "it runs unifi-evo-usher $(jq -r '.runs.version + ", built from " + .runs.commit[0:7]' "$work/answer")"
fi

# 3. The controller.
status=$(shelf access.controller info)
if [ "$status" != 200 ]; then
  fail "the controller is not told of (status $status): $(why)"
elif [ "$(jq -r '.set' "$work/said")" != true ]; then
  fail "no controller is set: the page Controller sets it"
elif [ "$(jq -r '.version // empty' "$work/said")" = "" ]; then
  fail "the controller at $(jq -r '.base' "$work/said") does not answer: $(jq -r '.trouble // "no reason given"' "$work/said")"
else
  ok "the controller at $(jq -r '.base' "$work/said") answers, UniFi Network $(jq -r '.version' "$work/said")"
  if [ "$(jq -r '.held' "$work/said")" = true ]; then
    note "it is believed by the one certificate it was set with"
  else
    note "it is believed by a certificate the public authorities vouch for"
  fi
fi

# 4. The wireless network vouchers are for.
site=""
status=$(shelf networks.directory chosen)
if [ "$status" != 200 ]; then
  fail "the network vouchers are for is not told (status $status): $(why)"
elif [ "$(jq -r '.chosen // empty' "$work/said")" = "" ]; then
  fail "no wireless network is chosen for vouchers: the page Network chooses it"
else
  site=$(jq -r '.chosen.site' "$work/said")
  ok "vouchers are for the wireless network '$(jq -r '.chosen.name' "$work/said")'"
fi

# 5. The voucher types.
standard=""
status=$(shelf issue.counter types)
if [ "$status" != 200 ]; then
  fail "the voucher types are not told (status $status): $(why)"
else
  count=$(jq -r '.types | length' "$work/said")
  standard=$(jq -r '.default // empty' "$work/said")
  if [ "$count" -gt 0 ]; then ok "$count voucher type(s) are defined: $(jq -r '[.types[].name] | join(", ")' "$work/said")"; else fail "no voucher type is defined: the page Voucher types defines them"; fi
  if [ -n "$standard" ]; then ok "the default type is '$standard'"; else fail "no voucher type is the default: a satellite, and a desk with one button, give out the default"; fi
fi

# 6. Who signs in, and the satellites.
status=$(signed /usher/v1/people)
if [ "$status" = 200 ]; then
  ok "$(jq -r '[.people[] | select(.role == "administrator")] | length' "$work/answer") administrator(s) and $(jq -r '[.people[] | select(.role != "administrator")] | length' "$work/answer") person(s) at the front desk"
  [ "$(jq -r '[.people[] | select(.role == "administrator")] | length' "$work/answer")" -gt 1 ] || note "one administrator alone: a second one is a way back in that needs no visit to the unit"
else
  fail "the people are not listed (status $status)"
fi
status=$(signed /usher/v1/satellites)
if [ "$status" = 200 ]; then
  total=$(jq -r '.satellites | length' "$work/answer")
  ok "$total satellite(s) are adopted"
  if [ "$total" -gt 0 ]; then
    jq -r --argjson now "$(date +%s)000" '.satellites[] | "  note  satellite \"" + .name + "\": " + (if .enabled == false then "switched off" elif .last_seen_ms == null then "not seen since the unit was started" elif ($now - .last_seen_ms) > 120000 then "last seen " + ((($now - .last_seen_ms) / 60000) | floor | tostring) + " minutes ago" else "seen just now" end)' "$work/answer"
  fi
else
  fail "the satellites are not listed (status $status)"
fi

# 7. The whole way: a voucher made on the controller, found there, revoked.
if [ "$voucher" = no ]; then
  note "no voucher was made (--no-voucher): the way to the controller and back was not walked"
elif [ -z "$standard" ] || [ -z "$site" ]; then
  fail "no voucher could be made: there is no default type, or no network is chosen"
else
  status=$(shelf issue.counter hand "$(jq -cn --arg type "$standard" '{type: $type}')")
  if [ "$status" != 200 ]; then
    fail "a voucher of the type '$standard' is not handed out (status $status): $(why)"
  else
    id=$(jq -r '.voucher.id' "$work/said")
    code=$(jq -r '.voucher.code' "$work/said")
    if [ "$(jq -r '.held' "$work/said")" = true ]; then from="one set aside on the unit"; else from="made on the controller for the asking"; fi
    ok "a voucher of the type '$standard' is handed out for '$(jq -r '.network' "$work/said")': $from"
    status=$(shelf access.controller list "$(jq -cn --arg site "$site" '{site: $site}')")
    if [ "$status" = 200 ] && [ "$(jq -r --arg id "$id" '[.vouchers[] | select(.id == $id)] | length' "$work/said")" = 1 ]; then
      ok "the controller lists it"
    else
      fail "the controller does not list the voucher that was handed out (status $status): $(why)"
    fi
    status=$(shelf access.controller revoke "$(jq -cn --arg site "$site" --arg id "$id" '{site: $site, id: $id}')")
    if [ "$status" = 200 ] && [ "$(jq -r '.revoked' "$work/said")" = 1 ]; then
      ok "it is revoked again: the code ${code:0:2}... opens nothing"
    else
      fail "the voucher $id is NOT revoked (status $status): $(why); revoke it on the page Vouchers"
    fi
    status=$(shelf issue.counter handed "$(jq -cn --arg id "$id" '{ids: [$id]}')")
    if [ "$status" = 200 ] && [ "$(jq -r --arg id "$id" --arg name "$name" '[.handed[] | select(.id == $id and .by == $name)] | length' "$work/said")" = 1 ]; then
      ok "the record of hand-outs names $name for it"
    else
      fail "the record of hand-outs does not name $name for the voucher (status $status)"
    fi
  fi
fi

# 8. Signing out ends the session.
status=$(signed /usher/v1/sign_out -X POST -H 'Content-Type: application/json' -d '{}')
after=$(signed /usher/v1/me)
if [ "$status" = 200 ] && [ "$after" = 401 ]; then ok "signing out ends the session"; else fail "signing out answered $status, and the session was then answered $after"; fi
rm -f "$work/session"

if [ "$failed" -eq 0 ]; then
  echo "[OK] go-live: nothing failed"
else
  echo "go-live: $failed failed"
  exit 1
fi
