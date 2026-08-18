# VHS tape for the social card. 1200x630 is the OG/Twitter card ratio, so this
# is the overview example framed to that shape rather than a cropped screenshot
# of something else — the card shows the actual library, and it regenerates
# from the repo like every other visual here.
#
# Regenerate:  vhs assets/og.tape && node web/scripts/og.mjs
#
# The still is a SCREENSHOT, not a GIF frame.
#
# This tape used to write an animated GIF that a second command then quantised
# down to its last frame, and a note here claimed VHS's Screenshot directive
# "silently no-ops in 0.11.0". It does not. What it needs is an `Output`
# directive present — Screenshot alone is ignored — and a relative path, since
# VHS's parser rejects absolute ones. With both, 0.11.0 writes the PNG happily.
#
# That workaround cost real quality: the card was being built from a 256-colour
# GIF frame when a true-colour still was available. The screenshot has ~10,000
# colours; the GIF frame had 255. For the one image that represents the project
# in every feed, that is the wrong place to economise.
#
# tmp-og.gif is the throwaway VHS insists on. It is gitignored.

Output tmp-og.gif

Set Shell "bash"
Set FontSize 16
Set Width 1200
Set Height 630
Set Padding 20
Set Framerate 12

Hide
Type "target/release/examples/overview"
Enter
Sleep 2500ms
Show
# One held frame: the fitted graph, image node in colour.
Type "p"
Sleep 600ms
Type "f"
Sleep 1200ms
Screenshot assets/og-shot.png
Hide
Type "q"
Sleep 300ms
