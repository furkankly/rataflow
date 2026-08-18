# VHS tape for the rataflow `overview` example — README hero.
# Regenerate:  vhs assets/overview.tape   (run from repo root after
#              `cargo build --example overview --release`)
#
# RATAFLOW_DEMO=1 enables the scripted pointer (rataflow_examples::autopilot),
# fired by 'g'. VHS is keyboard-only and films a headless terminal with no OS
# cursor, so the app draws and drives the pointer itself; 'g' runs ~7.5s of
# real mouse events through the actual drag path.
#
# Pacing note: the mouse sequence leads, because a pointer grabbing a node is
# the only part that reads as a person using the thing. Keyboard zoom and pan
# are the flourish afterwards, not the opener — an earlier cut spent its first
# eight seconds on '+' and 'hjkl' and looked like a screensaver.

# Two Outputs, one render. The MP4 is for the website and the GIF for the
# README, and emitting both here means they are the same recording by
# construction — there is no transcode step to skip, and no way for one to end
# up older than the other. VHS renders true-colour frames, so the MP4 never
# passes through the GIF's 256-colour palette on its way out.
Output assets/overview.gif
Output assets/overview.mp4

Set Shell "bash"
Set FontSize 13
Set Width 1280
Set Height 760
Set Padding 12
Set Framerate 24
Set TypingSpeed 120ms

# Launch the prebuilt binary without capturing the shell prompt.
Hide
Type "RATAFLOW_DEMO=1 target/release/examples/overview"
Enter
Sleep 1500ms
Show

# Reveal the image node in colour (half-blocks Ferris) before anything moves.
Sleep 400ms
Type "p"
Sleep 600ms

# --- The pointer runs five actions, four of them real mouse events through the
#     real state machine:
#       1. drag a nested child until it pins against its parent's edge
#       2. drag another node around a full circle, edges re-routing throughout
#       3. press empty canvas and drag to pan the viewport
#       4. wheel to zoom around the pointer
#       5. fit, so the last frame is the whole graph
#     plus one keyboard beat: Tab to the Input widget, type D's new width and
#     height, Enter to apply. That widget is keyboard-driven by design — it says
#     so on its face — so the pointer stays put while it happens.
#
#     The fit used to be a `Type "f"` here. It is Step::Fit now, inside the
#     script, which is what lets the same demo be captured somewhere that cannot
#     type into it — see docs/DEMO-ASSETS.md on recording Ghostty.
#
#     Sleep must be at least the script's own length, which the binary reports:
#     RATAFLOW_DEMO=duration target/release/examples/overview   (build.sh check
#     verifies this tape against it). It once said 9s for a 6.9s script, and
#     those spare seconds — a still frame before the closing fit — were the
#     worst thing about the recording. Sum the durations and use that number. ---
Type "g"
Sleep 12.8s

# Quit cleanly (not captured).
Hide
Type "q"
Sleep 500ms
