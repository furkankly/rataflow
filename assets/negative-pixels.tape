# VHS tape for the `basic` example — the demo for the article
# "Negative pixels don't exist: three coordinate systems behind a terminal
# flow graph".
#
# Regenerate:  vhs assets/negative-pixels.tape   (run from repo root after
#              `cargo build --example basic --release`)
#
# The article is about what happens when a node's position stops being
# representable: the framebuffer is indexed by u16, so a node panned past the
# left edge sits at a column the buffer has no name for. So the choreography is
# built around exactly that moment — pan west until a node is half off-screen
# and hold there, because a node CLEANLY CLIPPED at the boundary is the visual
# proof that the signed middle stage did its job. Panning back proves nothing
# was lost in the round trip.
#
# `basic` rather than `overview`: this shot needs the eye on one node crossing
# one boundary, and overview's density buries that.

# Two Outputs, one render. The MP4 is for the website and the GIF for the
# README, and emitting both here means they are the same recording by
# construction — there is no transcode step to skip, and no way for one to end
# up older than the other. VHS renders true-colour frames, so the MP4 never
# passes through the GIF's 256-colour palette on its way out.
Output assets/negative-pixels.gif
Output assets/negative-pixels.mp4

Set Shell "bash"
Set FontSize 13
Set Width 1280
Set Height 760
Set Padding 12
Set Framerate 24
Set TypingSpeed 120ms

# Launch the prebuilt binary without capturing the shell prompt.
Hide
Type "RATAFLOW_DEMO=1 target/release/examples/basic"
Enter
Sleep 1500ms
Show

# --- The pointer presses empty canvas and drags west, so the viewport pans and
#     a node crosses the left boundary; it holds there while the node is
#     half-clipped, then drags back to show nothing was lost in the round trip.
#     Script runtime ~4.6s.
#
#     Panning by MOUSE rather than by hjkl matters here: the article is about a
#     coordinate that stops being representable, and a hand dragging the canvas
#     into that state reads as cause and effect. A keypress reads as a setting. ---
Type "g"
Sleep 4.8s

# Quit cleanly (not captured).
Hide
Type "q"
Sleep 500ms
