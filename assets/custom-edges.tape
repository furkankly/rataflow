# VHS tape for the `custom_edges` example — the demo for the article
# "Rounded turns, sharp crossings: drawing flow-graph edges in a terminal".
#
# Regenerate:  vhs assets/custom-edges.tape   (run from repo root after
#              `cargo build --example custom_edges --release`)
#
# Five edges, five ways to draw one: a dotted line with end markers, a braille
# diagonal, a labelled step route, a line with badges painted into the buffer,
# and a hand-built zigzag. That IS the article's thesis — the box-drawing
# alphabet is a choice, not a constraint — so the shot is the five side by side,
# then close enough to read the glyphs each one is spelled with.
#
# Chosen over `edge_routing` (16 handle-position combos) because that example
# varies where an edge ATTACHES, and this piece is about how an edge is DRAWN.

# Two Outputs, one render. The MP4 is for the website and the GIF for the
# README, and emitting both here means they are the same recording by
# construction — there is no transcode step to skip, and no way for one to end
# up older than the other. VHS renders true-colour frames, so the MP4 never
# passes through the GIF's 256-colour palette on its way out.
Output assets/custom-edges.gif
Output assets/custom-edges.mp4

Set Shell "bash"
Set FontSize 13
Set Width 1280
Set Height 760
Set Padding 12
Set Framerate 24
Set TypingSpeed 120ms

# Launch the prebuilt binary without capturing the shell prompt.
Hide
Type "RATAFLOW_DEMO=1 target/release/examples/custom_edges"
Enter
Sleep 1500ms
Show

# --- The pointer clicks each of the five edges in turn, so every rendering
#     shows both its normal and its selected styling, then zooms into the last
#     one so the individual cells are legible. Script runtime ~7.1s.
#
#     Keyboard zoom used to do the close-up; the wheel is better because the
#     library zooms around the pointer, so it magnifies the edge just clicked. ---
Type "g"
Sleep 7.3s

# Quit cleanly (not captured).
Hide
Type "q"
Sleep 500ms
