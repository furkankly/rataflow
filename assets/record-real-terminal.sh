#!/usr/bin/env bash
# Records the `overview` demo from a REAL terminal and writes assets/overview.gif
# and assets/overview.mp4.
#
# The name is the whole point. Every other asset here is TAPED — VHS drives a
# headless Chromium running xterm.js — and this one is FILMED off a terminal
# that is actually on your screen. The difference matters for exactly one node,
# and that node is why this file exists.
#
#   cargo build --release --example overview
#   ./assets/record-real-terminal.sh        (or: ./assets/build.sh hero)
#
# Run it from inside the Ghostty window you want filmed, with that window
# frontmost, and then keep your hands off the machine for ~12 seconds.
#
# WHY NOT VHS, which produces every other asset in here. VHS films a headless
# Chromium running xterm.js, not your terminal. xterm.js has no Kitty graphics
# protocol, so the ratatui-image node degrades to half-blocks: Ferris comes out
# a blocky orange smear. Ghostty implements the protocol, so the same node draws
# as an actual image. That single node is the whole reason this script exists —
# everything else about the VHS recording was fine, and assets/overview.tape is
# still the fallback for anyone without a Mac.
#
# What that trade costs: VHS is hermetic and this is not. The output depends on
# the window size, the font, and the theme of whoever runs it. Size the window
# to roughly 1280x760 points before recording, so the frame needs no downscaling
# and the text stays the size it was authored at.
#
# THE TWO PERMISSIONS. Screen capture needs Screen Recording, granted to Ghostty
# (permission follows the parent app, not the binary). Without it macOS does not
# error — it hands back an image of the desktop picture with every window
# missing, which looks like a capture that worked. The preflight below exists
# because that failure is silent.
#
# The demo needs NO keystrokes: RATAFLOW_DEMO=auto starts the pointer script
# itself and Step::Fit ends it, so nothing here has to synthesise input and
# Accessibility permission never enters into it.
set -euo pipefail

cd "$(dirname "$0")/.."

BIN=target/release/examples/overview
OUT=assets/overview.gif
MP4=assets/overview.mp4
WORK=$(mktemp -d)

# On failure, say what failed and KEEP the evidence.
#
# The first version was `trap 'rm -rf "$WORK"' EXIT`, which under `set -e` meant
# any failing step killed the script and deleted the recorder's log in the same
# breath — the run left no output, no partial file and no reason, and diagnosing
# it came down to guessing which stage broke. A capture is a long, unattended,
# hands-off operation; it is exactly the kind of thing that has to explain
# itself the first time rather than the third.
FAIL_LINE=
cleanup() {
  local code=$?
  if ((code != 0)); then
    {
      echo
      echo "capture FAILED (exit $code)${FAIL_LINE:+, at line $FAIL_LINE}"
      if [[ -s $WORK/ffmpeg.log ]]; then
        echo "--- recorder log (last 20 lines):"
        tail -n 20 "$WORK/ffmpeg.log"
      fi
      echo "--- working files kept: $WORK"
    } >&2
    exit "$code"
  fi
  rm -rf "$WORK"
}
trap 'FAIL_LINE=$LINENO' ERR
trap cleanup EXIT

# Seconds of settled graph before the pointer moves. Also the window in which
# the recorder has to get going: it takes an unpredictable moment to start
# writing frames, and the head of the clip is trimmed by content below rather
# than by guessing that number.
LEAD=${LEAD:-1.6}
# Asked of the binary rather than written down. It was a hand-copied constant
# and it drifted the first time a beat changed length: the script grew to 12.62s
# while three separate places still said 12.44. `RATAFLOW_DEMO=duration` sums the
# real Step list and exits before ratatui touches the terminal, so it answers
# over a pipe.
SCRIPT_SECS=$(RATAFLOW_DEMO=duration "$BIN" 2>/dev/null || echo 0)
# Script plus lead plus slack for process start. Over-recording is free; the
# trim decides the real length.
DURATION=${DURATION:-18}
FPS=${FPS:-24}
# 0 leaves the capture at native size. A retina panel will double every number,
# so check the result before shipping a 40MB GIF.
WIDTH=${WIDTH:-0}

[[ -x $BIN ]] || { echo "build it first: cargo build --release --example overview" >&2; exit 1; }

if ! swift -e 'import CoreGraphics; exit(CGPreflightScreenCaptureAccess() ? 0 : 1)' 2>/dev/null; then
  cat >&2 <<'MSG'
Screen Recording is not granted to this terminal.

  System Settings > Privacy & Security > Screen & System Audio Recording
  enable Ghostty, then QUIT AND REOPEN Ghostty (the permission is only
  picked up at launch), and run this again.

Once an app has been denied it is never re-prompted, so this cannot be
requested from here.
MSG
  exit 1
fi

# --- Where the window is -----------------------------------------------------
#
# Inlined rather than kept beside this script as a .swift file. It has exactly
# one caller and no meaning apart from it, and a two-file recorder invited the
# reasonable question of why there were two.
#
# Swift because these are macOS system APIs with no CLI equivalent, and because
# `swift` runs a file as a script — no build step, no artifact, and it ships
# with the Command Line Tools that Rust already needs on this platform to link.
# The alternatives were worse: `osascript` needs Accessibility permission, which
# is precisely what this whole design avoids, and Python's Quartz bindings are
# not installed by default.
window_rect() {
  cat > "$WORK/window-rect.swift" <<'SWIFT'
// Prints the frontmost on-screen window of an app as a display index plus a
// crop rect in that display's PIXELS:
//
//   DISPLAY=2 X=0 Y=50 W=5120 H=2690 SCALE=2
//
// Three conversions have to happen and each one is a place the crop silently
// comes out wrong:
//
//   1. CGWindow bounds are in POINTS, in a global space whose origin is the
//      top-left of the main display. A recorder captures ONE display's
//      framebuffer, so the rect has to be made relative to that display.
//   2. Retina. The framebuffer is 2x the point size, so every number doubles.
//      Skipping this crops the top-left quarter of the window, which looks
//      enough like a legitimate framing that it is easy to ship by accident.
//   3. Which display. Picked by containment of the window's centre, not by
//      guessing, because a window can overlap two.
//
// Window TITLES are omitted deliberately: reading them needs Screen Recording
// permission, and this runs before that is necessarily granted.

import CoreGraphics
import Foundation

let app = CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : "Ghostty"

// Front-to-back order, so the first match is the window in front.
let windows = (CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements],
                                          kCGNullWindowID) as? [[String: Any]]) ?? []

func rect(_ w: [String: Any]) -> CGRect? {
    guard let b = w[kCGWindowBounds as String] as? [String: Any],
          let x = b["X"] as? Double, let y = b["Y"] as? Double,
          let width = b["Width"] as? Double, let height = b["Height"] as? Double
    else { return nil }
    return CGRect(x: x, y: y, width: width, height: height)
}

// Layer 0 only: Ghostty also keeps 24pt-tall and 500x500 helper windows around,
// and the tallest-first search below would otherwise be fine but the *frontmost*
// search would not. A minimum size guards against picking a sliver.
let match = windows.first { w in
    guard (w[kCGWindowOwnerName as String] as? String) == app,
          (w[kCGWindowLayer as String] as? Int) == 0,
          let r = rect(w), r.width > 200, r.height > 200
    else { return false }
    return true
}

guard let w = match, let r = rect(w) else {
    FileHandle.standardError.write("no on-screen \(app) window found\n".data(using: .utf8)!)
    exit(1)
}

// Find the display containing the window's centre.
let centre = CGPoint(x: r.midX, y: r.midY)
var count: UInt32 = 0
CGGetActiveDisplayList(0, nil, &count)
var ids = [CGDirectDisplayID](repeating: 0, count: Int(count))
CGGetActiveDisplayList(count, &ids, &count)

var chosen: (index: Int, bounds: CGRect, scale: Double)?
for (i, id) in ids.enumerated() {
    let b = CGDisplayBounds(id)
    if b.contains(centre) {
        // Pixel width over point width. CGDisplayPixelsWide reports the
        // framebuffer; CGDisplayBounds reports points.
        let scale = Double(CGDisplayPixelsWide(id)) / Double(b.width)
        // `screencapture -D` is 1-based and follows the active display order.
        chosen = (i + 1, b, scale)
        break
    }
}

guard let d = chosen else {
    FileHandle.standardError.write("window centre is on no active display\n".data(using: .utf8)!)
    exit(1)
}

let s = d.scale
// Clamp to the display, or ffmpeg's crop filter fails outright on a rect that
// runs past the frame — a window can hang off the edge.
let x = max(0, (r.minX - d.bounds.minX) * s)
let y = max(0, (r.minY - d.bounds.minY) * s)
let maxW = d.bounds.width * s - x
let maxH = d.bounds.height * s - y
// Even dimensions: yuv420 chroma subsampling needs them, and ffmpeg refuses odd
// ones rather than rounding.
let w2 = (min(r.width * s, maxW) / 2).rounded(.down) * 2
let h2 = (min(r.height * s, maxH) / 2).rounded(.down) * 2

print("DISPLAY=\(d.index) X=\(Int(x)) Y=\(Int(y)) W=\(Int(w2)) H=\(Int(h2)) SCALE=\(Int(s))")
SWIFT
  swift "$WORK/window-rect.swift" "$1"
}

echo "resolving window..."
eval "$(window_rect Ghostty)"
echo "  display $DISPLAY, crop ${W}x${H}+${X}+${Y} (scale ${SCALE}x)"

# The tape renders at 1280x760 with a 13pt font, and the articles are laid out
# around a hero that size. A fullscreen window instead produces a GIF that has
# to be halved to fit, and halving it halves the text — legible on the machine
# that recorded it, mush in a feed. Warn rather than refuse: a wide window is a
# deliberate choice for some shots.
if [[ $W -gt 1800 ]]; then
  cat >&2 <<MSG

  note: the window is ${W}x${H}. The tape's hero is 1280x760, so this will
  either ship oversized or be scaled down — and scaling shrinks the TEXT, which
  is the part that has to survive. Resize the Ghostty window to about 1280x760
  points and run again, or set WIDTH=1280 to accept the smaller type.

MSG
fi

# --- The grid, which matters more than the pixels -------------------------
#
# A terminal app is laid out in CELLS, so the character grid decides how much of
# the UI exists at all — not the resolution. `Set FontSize 13` at 1280x760 gives
# the tape 153x48, and rataflow truncates node labels to fit whatever it gets.
# The first Ghostty capture ran at roughly 119x34 because the recording window
# had an everyday font size, and the result was a hero where the Sparkline node
# read "ratatui Sparkli", the buffer node was a bare "(", and the Input widget
# rendered as an empty box. Nothing about that looks broken in isolation, which
# is exactly why it needs checking rather than eyeballing: it only shows up next
# to a capture that had the room.
#
# Read through /dev/tty, not `tput`. `COLS=$(tput cols)` looks obvious and is
# wrong: command substitution makes stdout a pipe, ncurses cannot ioctl a pipe
# for the window size, so it falls back to terminfo's defaults. It reported a
# confident 80x24 for a window that was exactly right, and told the user to
# shrink a font that did not need shrinking. `stty size` takes the size from its
# STDIN, which can be pointed at the real terminal.
GRID=$(stty size </dev/tty 2>/dev/null || echo "0 0")
ROWS=${GRID%% *}
COLS=${GRID##* }
echo "  grid ${COLS}x${ROWS} (the tape's is 153x48)"
if [[ $COLS -lt 150 || $ROWS -lt 44 ]]; then
  cat >&2 <<MSG

  note: ${COLS}x${ROWS} is a coarser grid than the tape's 153x48, so node labels
  will be truncated relative to the recording this replaces. Drop the font size
  (Cmd+Minus) until the line above reads about 153x48, then run again. Font size
  moves the grid far more than window size does.

MSG
fi

# -capture_cursor 0 keeps the OS pointer out of the frame. The rat the app draws
# is the only cursor that should appear; a real arrow parked over the window can
# still leave a hover artefact, so keep it off the window anyway.
echo "hands off — nothing should move for the next ~$((DURATION + 2))s"

RAW=$WORK/raw.mov

# --- Recording losslessly, which is not what the obvious tool does ----------
#
# `screencapture -v` writes H.264 **yuv420p**: chroma at half resolution in both
# axes. On a photo that is invisible; on a terminal it is destructive, because a
# node border is a ONE PIXEL coloured line on a dark background. That line shares
# a chroma sample with three dark pixels, the average desaturates, and it comes
# out grey. Worse, it does so *selectively* — whether a given line survives
# depends on its pixel parity against the 2x2 chroma grid, so a recording ends up
# with some borders coloured and some grey. That looked exactly like a rendering
# bug in the library, and was chased as one: the app was fine, the VHS render of
# the same sequence was fine, and only the capture was wrong.
#
# ffmpeg's avfoundation input takes the same screen and lets us name the pixel
# format. libx264rgb at -qp 0 keeps full RGB (gbrp), so every colour that
# survives to the GIF is a colour the terminal actually drew. It sustains 24fps
# at 2560x1440 without dropping frames; the file is large but temporary.
#
# The device index is NOT the display number. avfoundation enumerates cameras
# first and its screens are 0-based, while `screencapture -D` is 1-based, so the
# index is looked up rather than assumed — a machine with a different number of
# cameras would otherwise silently film the wrong thing.
#
# `|| true` is load-bearing. Listing devices is not a normal ffmpeg run: there
# is no input, so it prints the list to STDERR and exits **251**. Under
# `set -o pipefail` that status is the pipeline's, so the assignment fails and
# `set -e` kills the script before it records anything — which is exactly what
# happened, silently, after the "hands off" line. The emptiness check below is
# what decides whether the lookup actually worked; the exit code never could.
AVF=$(ffmpeg -hide_banner -f avfoundation -list_devices true -i "" 2>&1 \
      | grep -oE "\[[0-9]+\] Capture screen $((DISPLAY - 1))$" \
      | grep -oE "[0-9]+\]" | tr -d ']' | head -1 || true)
if [[ -z $AVF ]]; then
  echo "could not find an avfoundation device for display $DISPLAY" >&2
  exit 1
fi

# EVERYTHING this script has to say is said BEFORE the screen is blanked. The
# recording then starts on an empty terminal and stays empty until the app takes
# over, which buys two things: no script output is ever filmed, and the video
# becomes an unambiguous blank / app / blank sandwich that the trim below reads
# directly. The first version printed "recording..." after starting the
# recorder, and that line sat in the opening frames of the GIF.
sleep 0.4
clear

# -nostdin so it does not eat the terminal's input, and stderr to a file rather
# than the screen — anything ffmpeg prints here would be filmed.
ffmpeg -hide_banner -loglevel error -nostdin \
  -f avfoundation -capture_cursor 0 -framerate "$FPS" -i "$AVF" \
  -t "$DURATION" -c:v libx264rgb -qp 0 -preset ultrafast \
  -y "$RAW" 2>"$WORK/ffmpeg.log" &
CAP=$!

# Let the recorder reach a steady state against the blank screen. Its start-up
# latency is not fixed, which is why nothing here depends on knowing it.
sleep 1.5

RATAFLOW_DEMO=auto RATAFLOW_DEMO_LEAD="$LEAD" "$BIN"
# The app exits on its own once the script finishes; see the `auto` break in
# examples/overview.rs. Nothing here kills it, because a kill would skip
# ratatui's restore and leave the terminal in raw mode on the alt screen.

# Tolerate a non-zero recorder exit if it still produced a usable file.
# avfoundation is chatty and exits oddly in some configurations, and throwing
# away a good 18-second take over an exit code would be the wrong trade — but
# say so, because a warning that is never explained becomes noise.
CAP_STATUS=0
wait "$CAP" || CAP_STATUS=$?
clear
if ((CAP_STATUS != 0)); then
  if [[ -s $RAW ]]; then
    echo "  note: recorder exited $CAP_STATUS but wrote a file; continuing" >&2
  else
    echo "recorder exited $CAP_STATUS and wrote nothing" >&2
    exit 1
  fi
fi
echo "processing..."

# --- Where the terminal actually is, found rather than assumed -------------
#
# The window rect is NOT the crop. It includes the macOS title bar, Ghostty's
# tab bar and the window border, and the first cut of this script shipped all
# three — a hero GIF with the recording command sitting in the tab bar for its
# whole ten seconds.
#
# Subtracting a hardcoded chrome height would be wrong on the next machine: the
# tab bar only exists when a window has tabs, and the whole lot doubles on a
# retina panel. So the content rect is derived instead. The app frame and the
# blank frame share identical chrome and differ across the entire terminal
# content, so the bounding box of the difference IS the content rect — with no
# knowledge of what the chrome looks like, or whether there is any.
#
# Both frames are cut to the WINDOW first, and that is not a tidiness measure.
# The difference is only meaningful inside the window being recorded; anywhere
# else on the display, another app animating during the take is also a
# difference. A capture run with a second terminal open on the same screen
# produced a bbox spanning both, so the GIF came out with the menu bar, the
# title bar and a neighbouring window in it. Clipping to the window makes that
# impossible rather than unlikely.
APP=$WORK/app.png
BLANK=$WORK/blank.png
WIN_CROP="${W}x${H}+${X}+${Y}"
ffmpeg -hide_banner -loglevel error -ss "$(awk "BEGIN{print $DURATION/2}")" -i "$RAW" \
  -frames:v 1 -y "$APP"
# The tail, after the app has exited and restored the screen that was cleared
# before it launched.
ffmpeg -hide_banner -loglevel error -sseof -0.4 -i "$RAW" -update 1 -frames:v 1 -y "$BLANK"

# The bbox comes back relative to the window, so the window origin is added
# below to put it back in display coordinates.
BBOX=$(magick \
        \( "$APP"   -crop "$WIN_CROP" +repage \) \
        \( "$BLANK" -crop "$WIN_CROP" +repage \) \
        -compose difference -composite \
        -colorspace Gray -threshold 12% -format "%@" info: 2>/dev/null || true)
OK=0
if [[ $BBOX =~ ^([0-9]+)x([0-9]+)\+([0-9]+)\+([0-9]+)$ ]]; then
  # Read the captures out immediately: any command in between resets
  # BASH_REMATCH, which fails silently as an empty crop.
  CW=$(( ${BASH_REMATCH[1]} / 2 * 2 )); CH=$(( ${BASH_REMATCH[2]} / 2 * 2 ))
  # Back into display coordinates: the bbox was measured inside the window.
  CX=$(( ${BASH_REMATCH[3]} + X )); CY=$(( ${BASH_REMATCH[4]} + Y ))
  # The content fills the window bar its borders and the chrome strip, so a
  # detection much smaller than the window means the diff found only part of the
  # UI — a partial crop that would otherwise look like a deliberate framing.
  [[ $CW -gt 400 && $CH -gt 300 ]] \
    && [[ $CW -gt $(( W * 8 / 10 )) && $CH -gt $(( H * 7 / 10 )) ]] && OK=1
fi
if [[ $OK == 1 ]]; then
  echo "  terminal content: ${CW}x${CH}+${CX}+${CY} (chrome removed)"
else
  # Falling back to the window rect keeps a run usable, but say so — the output
  # will have chrome in it and that is a thing to notice before publishing.
  CW=$W; CH=$H; CX=$X; CY=$Y
  echo "  WARNING: content detection failed, using the whole window (chrome included)" >&2
fi

# --- When the app was on screen -------------------------------------------
#
# starting the recorder and its first written frame varies run to run, so every
# wall-clock offset this script could compute is wrong by an unknown amount.
#
# Cropped to the content, a blank terminal is almost entirely background and the
# app is not, so `blackdetect` separates them cleanly. The video is blank / app
# / blank by construction (see the clear above), which makes this two numbers:
# the first blank ends when the app appears, the last begins when it exits.
DET=$(ffmpeg -hide_banner -nostats -i "$RAW" \
        -vf "crop=${CW}:${CH}:${CX}:${CY},blackdetect=d=0.25:pix_th=0.22:pic_th=0.97" \
        -an -f null - 2>&1 | grep -o 'black_start:[0-9.]* black_end:[0-9.]*' || true)
START=$(printf '%s\n' "$DET" | head -1 | sed -n 's/.*black_end:\([0-9.]*\).*/\1/p')
END=$(printf '%s\n' "$DET" | tail -1 | sed -n 's/.*black_start:\([0-9.]*\).*/\1/p')

# The script's own duration is the check on the detection, not the source of it.
EXPECT=$(awk -v l="$LEAD" -v s="$SCRIPT_SECS" 'BEGIN { print l + s }')
if [[ -n $START && -n $END ]] && awk -v a="$START" -v b="$END" 'BEGIN{exit !(b-a > 4)}'; then
  LEN=$(awk -v a="$START" -v b="$END" 'BEGIN { print b - a }')
else
  echo "  WARNING: could not time the app in the video, falling back to the script length" >&2
  START=1.5
  LEN=$EXPECT
fi
echo "  app on screen: ${START}s for ${LEN}s (script is ${EXPECT}s)"

SCALE_F=""
[[ $WIDTH -gt 0 ]] && SCALE_F=",scale=${WIDTH}:-2:flags=lanczos"

# Two passes with a shared palette. A GIF holds 256 colours and the default
# global palette wrecks a terminal's greys; stats_mode=diff weights the palette
# toward what actually changes between frames, which here is the graph rather
# than the static chrome around it.
PAL=$WORK/pal.png
FILTERS="fps=${FPS},crop=${CW}:${CH}:${CX}:${CY}${SCALE_F}"
ffmpeg -hide_banner -loglevel error -ss "$START" -t "$LEN" -i "$RAW" \
  -vf "${FILTERS},palettegen=stats_mode=diff" -y "$PAL"
ffmpeg -hide_banner -loglevel error -ss "$START" -t "$LEN" -i "$RAW" -i "$PAL" \
  -lavfi "${FILTERS}[x];[x][1:v]paletteuse=dither=bayer:bayer_scale=3:diff_mode=rectangle" \
  -y "$OUT"

# The same clip as MP4, for the website. Encoded from the SCREEN RECORDING, not
# from the GIF that was just written — a GIF is 256 colours and dithered, and
# re-encoding one means spending bitrate on the dithering. This is the reason
# assets/to-mp4.sh skips overview: it only has the GIF to work from.
#
# The README and crates.io keep the GIF regardless; GitHub strips <video> and
# crates.io renders no video at all.
ffmpeg -hide_banner -loglevel error -ss "$START" -t "$LEN" -i "$RAW" \
  -vf "${FILTERS}" -c:v libx264 -crf 20 -preset slow -pix_fmt yuv420p \
  -movflags +faststart -an -y "$MP4"

echo
echo "wrote $OUT"
printf "wrote %s (%s)\n" "$MP4" "$(du -h "$MP4" | cut -f1)"
ffprobe -v error -select_streams v:0 -show_entries stream=width,height,nb_frames \
        -of default=noprint_wrappers=1 "$OUT" 2>/dev/null || true
du -h "$OUT" | cut -f1 | sed 's/^/size: /'

# Check the ends rather than trusting the trim. A frame that still shows a shell
# is the failure this script has actually shipped, and it is invisible in a file
# listing — the GIF is the right size, the right length, and wrong. Comparing
# each end against the blank reference costs nothing and names the problem.
for f in 0 -1; do
  d=$(magick "${OUT}[$f]" -coalesce -resize 200x -colorspace Gray \
        \( "$BLANK" -crop "${CW}x${CH}+${CX}+${CY}" +repage -resize 200x -colorspace Gray \) \
        -compose difference -composite -format "%[fx:mean*100]" info: 2>/dev/null || echo 99)
  where=$([[ $f == 0 ]] && echo "first" || echo "last")
  awk -v d="$d" -v w="$where" 'BEGIN {
    if (d < 1.5) printf "  WARNING: the %s frame looks like a bare terminal — the trim missed\n", w
  }'
done

echo
echo "check the image node: it should be a real crab, not orange blocks."
