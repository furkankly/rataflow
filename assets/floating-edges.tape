# VHS tape for the `floating_edges` example — the figure for the floating-edge
# paragraph in "Rounded turns, sharp crossings: drawing flow-graph edges in a
# terminal".
#
# Regenerate:  vhs assets/floating-edges.tape   (run from repo root after
#              `cargo build --example floating_edges --release`)
#
# The subject is ATTACHMENT, not routing: where an edge meets a node, and who
# decides it. So the choreography drags one node around the same loop four
# times and changes only the attachment between passes. Holding the gesture
# constant is what makes the difference attributable — a different orbit each
# time would leave the viewer comparing two things at once.
#
# No zoom, unlike edge-routing.tape. What has to be legible here is the endpoint
# against the node's outline, which is a whole-node relationship; zooming past
# the rectangle would crop the very thing being compared.

# Two Outputs, one render. The MP4 is for the website and the GIF for the
# README, and emitting both here means they are the same recording by
# construction — there is no transcode step to skip, and no way for one to end
# up older than the other.
Output assets/floating-edges.gif
Output assets/floating-edges.mp4

Set Shell "bash"
Set FontSize 13
Set Width 1280
Set Height 760
Set Padding 12
Set Framerate 24
Set TypingSpeed 120ms

# Launch the prebuilt binary without capturing the shell prompt.
Hide
Type "RATAFLOW_DEMO=1 target/release/examples/floating_edges"
Enter
Sleep 1500ms
Show

# --- Grab the "drag me" satellite, bring it in beside the hub, then take it a
#     full lap around the hub — once per attachment, cycling with 'a' between
#     laps. A full lap is what makes the point: the side an edge attaches to has
#     to change four times to get back where it started. The status bar names
#     the mode, so the recording carries its own legend.
#
#     Passes two and three are the pair worth watching: same routing, same
#     nodes, and the endpoint jumps between side midpoints in one and slides
#     along the outline in the other.
#
#     Script runtime is 12.38s, reported by the binary itself:
#       RATAFLOW_DEMO=duration target/release/examples/floating_edges
#     Sleep must stay >= that number or VHS cuts the last lap mid-drag. ---
Type "g"
Sleep 12.6s

# The script ends on Step::Fit, so the closing frame is the whole graph back at
# fit scale. This dwell lets that animation settle before the last frame.
Sleep 800ms

# Quit cleanly (not captured).
Hide
Type "q"
Sleep 500ms
