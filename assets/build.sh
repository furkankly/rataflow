#!/usr/bin/env bash
# One entry point for every moving image and social card in this repo.
#
#   ./assets/build.sh tapes    regenerate the VHS demos (GIF + MP4 together)
#   ./assets/build.sh hero     re-record the Ghostty hero (needs you at the keyboard)
#   ./assets/build.sh og       rebuild the social card
#   ./assets/build.sh social   rebuild the GitHub repo social preview
#   ./assets/build.sh sync     copy everything to where it is served
#   ./assets/build.sh check    verify the lot without changing anything
#   ./assets/build.sh all      tapes + og + social + sync (not hero: it needs a human)
#
# ADDING A DEMO: add its name to TAPES below and drop a matching
# assets/<name>.tape beside it. Every command here — encoding, syncing, the
# orphan and staleness checks — picks it up from that one line. The previous
# arrangement was a handful of scripts that each knew their own list, which is
# how the earlier drift started.
#
# WHY EACH OUTPUT EXISTS, since the formats look redundant and are not:
#
#   GIF  the README and crates.io. GitHub strips <video> from markdown, and
#        crates.io renders no video at all, so this is the only animated format
#        that works in either.
#   MP4  the blog. Roughly a third of the size; a page has none of the README's
#        limits. Each tape emits it alongside its GIF, from the same render, so
#        the two cannot disagree and there is no transcode step to forget.
#   PNG  two of them, and they are not interchangeable. og.png (1200x630,
#        web/scripts/og.mjs) leads with a screenshot of the real UI, for
#        someone who already clicked a link about the library.
#        social-preview.png (1280x640, web/scripts/social-preview.mjs) leads
#        with the mascot, for a stranger seeing the repo name in a feed.
#
# See docs/DEMO-ASSETS.md for why the hero is captured rather than taped, and
# why it is captured losslessly.
set -euo pipefail

cd "$(dirname "$0")/.."

# VHS-produced demos. One line each; everything below is derived.
TAPES=(custom-edges edge-routing negative-pixels drag-and-connect floating-edges)
# The hero is the exception: it is screen-captured from Ghostty so its
# ratatui-image node renders as a real image rather than half-blocks.
# assets/overview.tape stays as the no-Mac fallback.
HERO=overview
# Where the site and the blog serve their copies from. Both are optional: a
# clone without the blog checked out should still be able to build its assets.
WEB_PUBLIC=web/public
BLOG=${BLOG:-../furkankly.dev/public/demos}

die() { echo "$*" >&2; exit 1; }
have() { command -v "$1" >/dev/null || die "missing $1"; }

all_gifs() { printf '%s\n' "${TAPES[@]}" "$HERO"; }

cmd_tapes() {
  have vhs
  cargo build --release --examples
  for name in "${TAPES[@]}"; do
    echo "vhs assets/$name.tape"
    vhs "assets/$name.tape"        # writes both the GIF and the MP4
  done
  echo
  echo "note: the hero is not taped. ./assets/build.sh hero re-records it,"
  echo "      or 'vhs assets/overview.tape' for the half-blocks fallback."
}

cmd_hero() { exec ./assets/record-real-terminal.sh; }

cmd_og() {
  have vhs
  cargo build --release --example overview
  vhs assets/og.tape          # writes assets/og-shot.png; see the tape
  rm -f tmp-og.gif            # VHS requires an Output even when only a Screenshot is wanted
  node web/scripts/og.mjs
}

cmd_social() {
  node web/scripts/social-preview.mjs
  echo "  upload it by hand: repo Settings -> Social preview (GitHub has no API for it)"
}

cmd_sync() {
  [[ -f assets/og.png ]] && cp assets/og.png "$WEB_PUBLIC/" && echo "  og.png -> $WEB_PUBLIC/"
  # The favicon is the mascot, so it is a copy of assets/icon.svg rather than
  # its own drawing. Kept in sync here for the same reason og.png is: the site
  # serves from web/public, and a second hand-maintained copy of a mark is a
  # copy that ends up a version behind.
  [[ -f assets/icon.svg ]] && cp assets/icon.svg "$WEB_PUBLIC/favicon.svg" \
    && echo "  icon.svg -> $WEB_PUBLIC/favicon.svg"
  if [[ -d $BLOG ]]; then
    cp assets/*.gif assets/*.mp4 "$BLOG/" 2>/dev/null || true
    echo "  gifs + mp4s -> $BLOG/"
  else
    echo "  no blog at $BLOG — skipping (set BLOG=... to point elsewhere)"
  fi
}

cmd_check() {
  local bad=0
  echo "expected demos: ${TAPES[*]} $HERO"

  # The hero's tape has to sleep at least as long as the pointer script runs, or
  # VHS cuts the recording mid-gesture. The script's length is asked of the
  # binary, which sums its own Step list — the number used to be hand-copied
  # into the tape and into the recorder, and it drifted the first time a beat
  # changed length.
  if [[ -x target/release/examples/overview ]]; then
    local secs sleep_s
    secs=$(RATAFLOW_DEMO=duration target/release/examples/overview 2>/dev/null || echo 0)
    sleep_s=$(grep -oE '^Sleep [0-9.]+s' assets/overview.tape | tail -1 | grep -oE '[0-9.]+')
    if [[ -n $secs && -n $sleep_s ]] && awk -v a="$sleep_s" -v b="$secs" 'BEGIN{exit !(a < b)}'; then
      echo "  SHORT   assets/overview.tape sleeps ${sleep_s}s for a ${secs}s script"; bad=1
    fi
  else
    echo "  note: build the overview example to check the tape's Sleep"
  fi

  for name in $(all_gifs); do
    [[ -f assets/$name.gif ]] || { echo "  MISSING assets/$name.gif"; bad=1; }
    [[ -f assets/$name.mp4 ]] || { echo "  MISSING assets/$name.mp4"; bad=1; }
    # A GIF newer than its MP4 means the MP4 is of an older recording — the kind
    # of skew nobody notices until the two are seen side by side.
    # Both come out of one render now, so a mismatch here means someone
    # rebuilt one by hand rather than through this script.
    if [[ -f assets/$name.gif && -f assets/$name.mp4 && assets/$name.gif -nt assets/$name.mp4 ]]; then
      echo "  STALE   assets/$name.mp4 is older than its GIF"; bad=1
    fi
    if [[ -d $BLOG && -f assets/$name.mp4 ]]; then
      if ! cmp -s "assets/$name.mp4" "$BLOG/$name.mp4"; then
        echo "  UNSYNCED $BLOG/$name.mp4"; bad=1
      fi
    fi
  done

  # Anything in assets/ that no command produces and nothing references.
  for f in assets/*.gif assets/*.mp4; do
    [[ -e $f ]] || continue
    local n; n=$(basename "$f"); n=${n%.*}
    if ! all_gifs | grep -qx "$n"; then
      echo "  ORPHAN  $f is not in TAPES and is not the hero"; bad=1
    fi
  done

  [[ -f assets/og-shot.png ]] || { echo "  MISSING assets/og-shot.png (run: build.sh og)"; bad=1; }
  [[ -f $WEB_PUBLIC/og.png ]] && cmp -s assets/og.png "$WEB_PUBLIC/og.png" || {
    echo "  UNSYNCED $WEB_PUBLIC/og.png"; bad=1; }
  [[ -f $WEB_PUBLIC/favicon.svg ]] && cmp -s assets/icon.svg "$WEB_PUBLIC/favicon.svg" || {
    echo "  UNSYNCED $WEB_PUBLIC/favicon.svg (run: build.sh sync)"; bad=1; }
  [[ -f assets/social-preview.png ]] || { echo "  MISSING assets/social-preview.png (run: build.sh social)"; bad=1; }
  # The card is drawn FROM the mascot, so a mascot edited afterwards means the
  # card on GitHub is of an older drawing. Same staleness rule as the MP4s.
  if [[ assets/mascot.svg -nt assets/social-preview.png ]]; then
    echo "  STALE   assets/social-preview.png is older than assets/mascot.svg"; bad=1
  fi

  ((bad == 0)) && echo "all good" || return 1
}

case "${1:-}" in
  tapes) cmd_tapes ;;
  hero)  cmd_hero ;;
  og)     cmd_og ;;
  social) cmd_social ;;
  sync)  cmd_sync ;;
  check) cmd_check ;;
  all)   cmd_tapes; cmd_og; cmd_social; cmd_sync ;;
  *)     sed -n '2,10p' "$0" | sed 's/^# \{0,1\}//'; exit 1 ;;
esac
