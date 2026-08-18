# VHS tape for the `edge_routing` example — the demo for the article
# "Rounded turns, sharp crossings: drawing flow-graph edges in a terminal".
#
# Regenerate:  vhs assets/edge-routing.tape   (run from repo root after
#              `cargo build --example edge_routing --release`)
#
# The example is "all 16 handle position combos, step edges on top, straight
# below", so the choreography is: show the whole field, then get close enough
# that individual box-drawing glyphs and the corner joins are legible. The
# article is about what those glyphs ARE, so the zoomed pass is the point —
# a fit-to-view shot alone reads as "some lines" at GIF resolution.

# Two Outputs, one render. The MP4 is for the website and the GIF for the
# README, and emitting both here means they are the same recording by
# construction — there is no transcode step to skip, and no way for one to end
# up older than the other. VHS renders true-colour frames, so the MP4 never
# passes through the GIF's 256-colour palette on its way out.
Output assets/edge-routing.gif
Output assets/edge-routing.mp4

Set Shell "bash"
Set FontSize 13
Set Width 1280
Set Height 760
Set Padding 12
Set Framerate 24
Set TypingSpeed 120ms

# Launch the prebuilt binary without capturing the shell prompt.
Hide
Type "RATAFLOW_DEMO=1 target/release/examples/edge_routing"
Enter
Sleep 1500ms
Show

# --- The pointer grabs one step-edge target node and drags it around a full
#     loop, so the route re-elbows continuously as the source/target relationship
#     sweeps through every quadrant. It zooms in BEFORE grabbing the node, so the
#     corner glyphs are legible while they are being recomputed rather than after.
#     Script runtime ~5.3s.
#
#     The grid of 16 combos shows the OUTCOMES of the routing rule. One node
#     dragged in a circle shows the rule itself, which is the thing a static
#     screenshot can never carry. ---
Type "g"
Sleep 5.5s

# Fit back out so the last frame is the whole field of combos.
Type "f"
Sleep 1200ms

# Quit cleanly (not captured).
Hide
Type "q"
Sleep 500ms
