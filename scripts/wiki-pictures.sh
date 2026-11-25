#!/bin/bash
# The pictures of the face that the manual shows, made from the face
# itself: the tests that work the face in a browser write a picture of
# each page as they pass it, against a simulated controller, and this
# puts those pictures into a directory under the names the manual uses.
# Run again whenever a screen changes, so that what the manual shows is
# what the face looks like.
#
#   scripts/wiki-pictures.sh <directory>
#
# <directory> is where the pictures go, the images/ directory of a clone
# of the wiki for one. The pictures are of the dark face; with
# USHER_SCHEME=light they are of the light one. It needs the browser the
# tests need (bins/evo-usher-steward/tests/common/browser.rs says which),
# and says so where there is none.
set -euo pipefail
ROOT=$(CDPATH='' cd -- "$(dirname "$0")/.." && pwd)

to=${1:?the directory the pictures go into}
mkdir -p "$to"
to=$(CDPATH='' cd -- "$to" && pwd)
cd "$ROOT"

made=$(mktemp -d)
trap 'rm -rf "$made"' EXIT

USHER_SCHEME=${USHER_SCHEME:-dark} USHER_PICTURE="$made/face.png" \
  cargo test --locked -p evo-usher-steward --test browser

# What the tests call a picture, and what the manual calls it.
pictures="
face.png:counter-voucher.png
face-set-up.png:set-up.png
face-set-up-code.png:set-up-code.png
face-sign-in.png:sign-in.png
face-people.png:people.png
face-people-password.png:people-password.png
face-controller-key.png:controller-key.png
face-controller.png:controller.png
face-network.png:network.png
face-types.png:voucher-types.png
face-types-form.png:voucher-types-form.png
face-counter-days.png:counter-choice.png
face-guest.png:guest.png
face-vouchers.png:vouchers.png
face-vouchers-assigned.png:vouchers-assigned.png
face-vouchers-narrow.png:vouchers-narrow.png
face-narrow.png:narrow.png
face-site-set.png:site.png
face-site-worn.png:site-worn.png
face-site-guest.png:site-guest.png
face-site-sign-in.png:site-sign-in.png
face-language-german.png:language-german.png
face-language-site.png:language-site.png
face-language-form-ja.png:language-form-japanese.png
face-language-pl.png:language-polish.png
face-language-ru.png:language-russian.png
face-language-ja.png:language-japanese.png
face-people-front-desk.png:people-front-desk.png
face-people-pin.png:people-pin.png
face-people-set-pin.png:people-set-pin.png
face-front-desk.png:front-desk.png
face-vouchers-handed-by.png:vouchers-handed-by.png
face-satellite-code.png:satellite-code.png
face-satellites-adopt.png:satellites-adopt.png
face-satellites.png:satellites.png
face-satellite-button.png:satellite-button.png
face-satellite-voucher.png:satellite-voucher.png
face-vouchers-handed-at.png:vouchers-handed-at.png
face-site-cropper.png:site-cropper.png
face-satellites-adopt-protected.png:satellites-adopt-protected.png
face-satellite-types.png:satellite-types.png
face-satellite-keypad.png:satellite-keypad.png
face-demo-counter.png:demo-counter.png
face-site-travel.png:site-travel.png
face-site-apply-profile.png:site-apply-profile.png
face-set-up-from-file.png:set-up-from-file.png
face-printers.png:printers.png
face-printers-add.png:printers-add.png
face-printers-who.png:printers-who.png
face-counter-print.png:counter-print.png
face-satellite-print.png:satellite-print.png
face-satellite-printed.png:satellite-printed.png
"
for pair in $pictures; do
  from=${pair%%:*}
  name=${pair##*:}
  [ -s "$made/$from" ] || { echo "wiki-pictures: the tests wrote no $from; without a browser they pass and draw nothing" >&2; exit 1; }
  install -m 644 "$made/$from" "$to/$name"
  echo "[OK] $to/$name"
done
