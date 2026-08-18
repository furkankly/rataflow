# VHS tape for the `validation` example — the demo for the article
# "Three mouse bytes and a state machine: drag and connect in a terminal".
#
# Regenerate:  vhs assets/drag-and-connect.tape   (run from repo root after
#              `cargo build --example validation --release`)
#
# RATAFLOW_DEMO=1 enables the scripted pointer (rataflow_examples::autopilot),
# fired by 'g'. This is the article that could not be recorded at all until the
# autopilot existed: its subject is a MOUSE GESTURE, and VHS sends only
# keystrokes.
#
# The script makes three attempts: one accepted, then two refused for DIFFERENT
# reasons — the validator (`conn.target != "rejected"`) and a handle flag
# (`connectable_end(false)`). That trio is the whole example.
#
# The refusals are the half a screenshot cannot carry: they leave nothing
# behind, so the only way to show one is to show the attempt while it is live.
# And the accepted drag targets `no_outgoing`, whose target handle is plain —
# not `target`, which edge e1 already reaches, where success would have looked
# like nothing happening.
#
# Script runtime ~8.5s.

# Two Outputs, one render. The MP4 is for the website and the GIF for the
# README, and emitting both here means they are the same recording by
# construction — there is no transcode step to skip, and no way for one to end
# up older than the other. VHS renders true-colour frames, so the MP4 never
# passes through the GIF's 256-colour palette on its way out.
Output assets/drag-and-connect.gif
Output assets/drag-and-connect.mp4

Set Shell "bash"
Set FontSize 13
Set Width 1280
Set Height 760
Set Padding 12
Set Framerate 24
Set TypingSpeed 120ms

# Launch the prebuilt binary without capturing the shell prompt.
Hide
Type "RATAFLOW_DEMO=1 target/release/examples/validation"
Enter
Sleep 1500ms
Show

Sleep 500ms
Type "g"
Sleep 8.7s

# Quit cleanly (not captured).
Hide
Type "q"
Sleep 500ms
