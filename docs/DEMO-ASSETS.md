# Demo assets — how the GIFs are made

Every moving image in the README and in the articles is **reproducible from the
repo**. Nothing is a one-off screen capture, and that is deliberate: a recording
nobody can regenerate is one that silently rots the first time the UI changes.

## The tool: VHS

[charmbracelet/vhs](https://github.com/charmbracelet/vhs) — installed at
`/opt/homebrew/bin/vhs` (`brew install vhs`).

VHS records a terminal session from a declarative script (a "tape"), so a
recording is a build artifact rather than something a human performed once. It
takes keystrokes as data, which is what lets a tape *demonstrate* zooming into
glyph detail or panning a node past the buffer's edge instead of just showing a
static graph.

## What makes rataflow's setup unusual

The thing being recorded and the thing running on the website are **the same
source**. `examples/shared` (the `rataflow-examples` crate) is consumed twice:

```
examples/edge_routing.rs  →  use rataflow_examples::{ExampleMeta, render_shell}   (native, what VHS records)
web/wasm/src/main.rs      →  use rataflow_examples::{render_shell, ExampleMeta}   (wasm, what the site serves)
```

So a GIF of `edge_routing` and the live browser demo of `edge_routing` cannot
drift — they are one example compiled for two targets. An article can show the
recording, link the live version, and print the `cargo run` line, and all three
are the same artifact. That is a stronger guarantee than a shared fixture file,
because there is no fixture to keep in sync in the first place.

It also means a broken example is a broken demo *and* a broken website section,
which is the right blast radius: they fail together, loudly.

The *prose* used to be the hole in this. Titles, descriptions and key lists were
written twice — once in `examples/<name>.rs`, once in `web/wasm/src/demos.rs`
— and eight of twenty-one had drifted, with the wasm copies describing older
versions of their examples. They now come from `examples/shared/src/meta.rs`,
so a description is as shared as the flow it describes. The one genuine
per-platform difference is `q`, which native binaries add via
`ExampleMeta::with_quit()` because a browser tab has nothing to quit.

## Regenerating

Everything goes through one script, so there is one list of demos rather than
one per tool. Adding a demo is a line in `TAPES` inside it plus a matching tape.

```bash
./assets/build.sh all       # tapes + mp4 + social card + copies
./assets/build.sh check     # verify without changing anything
./assets/build.sh hero      # re-record the Ghostty hero (needs you present)
```

`check` is the one to run before publishing. It catches a missing MP4, an MP4
older than its GIF, a copy that drifted from its source, any file in `assets/`
that no tape produces, and a tape that sleeps for less time than the pointer
script runs. Each of those has actually happened here.

That last one is worth explaining, because it is where a number used to be
written three times. The script's length lives in one place — the `Step` list in
`demo_pilot` — and the binary reports it:

```bash
RATAFLOW_DEMO=duration target/release/examples/overview   # -> 12.62
```

`demo_steps` is separated from the waypoints it aims at precisely so this can be
answered without a terminal, a layout, or a running app: the marks decide where
the pointer goes, never how long it takes. The recorder asks at run time and
`check` compares the tape against it. Before that, the number was copied by hand
into the recorder and the tape, and it drifted the first time a beat changed
length — the typed value went from one character to two, the script became
12.62s, and three places still said 12.44.

Underneath, each tape needs its example built in release first — VHS launches
the prebuilt binary rather than `cargo run`, so a recording never captures a
compile — and `build.sh tapes` does that for you. A single tape by hand still
works:

```bash
cargo build --example overview --release
vhs assets/overview.tape          # run from the repo root
```

Every tape now drives the scripted pointer rather than the keyboard. The
per-example wiring is `autopilot::DemoPilot` — read the env var, start a script
on `g`, pump events into the flow, draw the cursor last — so an example needs
four lines rather than its own copy of the loop. The copies were how the first
version drifted into animating node positions instead of emitting events.

| Tape | Example | Output | Used by |
| --- | --- | --- | --- |
| `assets/overview.tape` | `overview` | `overview.gif` | README hero, "Building a node editor on a grid of terminal cells" — but see [Recording a real terminal](#recording-a-real-terminal-ghostty): the shipped hero is captured from Ghostty so the image node renders as an image, and this tape is the fallback |
| `assets/custom-edges.tape` | `custom_edges` | `custom-edges.gif` | "Rounded turns, sharp crossings" |
| `assets/edge-routing.tape` | `edge_routing` | `edge-routing.gif` | second figure in the routing section of "Rounded turns, sharp crossings" |
| `assets/negative-pixels.tape` | `basic` | `negative-pixels.gif` | "Negative pixels don't exist" |
| `assets/drag-and-connect.tape` | `validation` | `drag-and-connect.gif` | "Three mouse bytes and a state machine" |
| `assets/floating-edges.tape` | `floating_edges` | `floating-edges.gif` | the floating-edge figure in the routing section of "Rounded turns, sharp crossings" |
| `assets/og.tape` | `overview` | `og.png` | the social card (`web/public/og.png`) |

Current outputs, all 24 fps. The tapes render 1280×760; `overview.gif` is the
exception because it is captured from Ghostty rather than VHS, so its size is
whatever the recording window's grid works out to.

| File | Size | Frames | Bytes |
| --- | --- | --- | --- |
| `overview.gif` (Ghostty) | 1220×712 | 342 | ~915 KB |
| `custom-edges.gif` | 1280×760 | 181 | ~244 KB |
| `edge-routing.gif` | 1280×760 | 169 | ~236 KB |
| `negative-pixels.gif` | 1280×760 | 119 | ~480 KB |
| `drag-and-connect.gif` | 1280×760 | 228 | ~112 KB |
| `floating-edges.gif` | 1280×760 | 330 | ~529 KB |

`overview.gif` is the heaviest, but only about 4x — and not for the reason that
seems obvious. It is longer than the others and carries a true-colour image, but
what actually dominated was the recorder: while it went through `screencapture`
it weighed **3 MB**, and recording losslessly took it to under a megabyte
without changing a frame of content. The explanation that lived here before —
"a real terminal renders anti-aliased text, which a 256-colour palette handles
badly" — sounded right, was written confidently, and was wrong. See the
measurement under [Recording a real terminal](#recording-a-real-terminal-ghostty).

`custom_edges` carries the box-drawing article rather than `edge_routing`
because the two vary different things: `edge_routing` varies where an edge
ATTACHES (16 handle-position combos), while `custom_edges` varies how one is
DRAWN (five renderings of a single edge). The article is about the alphabet, so
it takes the second. `edge-routing.tape` supplies the second figure in that
article's routing section.

### Choosing the example for an article

Pick the example whose *subject* is the article's subject, not the one that
looks busiest. `negative-pixels` records `basic`, not `overview`, because that
shot needs the eye on one node crossing one boundary and overview's density
buries it. Conversely the hub article records `overview` precisely because it is
about everything at once.

The trap to avoid: an early cut of `edge-routing.tape` panned four rows down
during its zoomed section and spent the whole close-up on the *straight* edges,
which are a braille renderer — while the article is about box-drawing step edges
and their corner joins. The tape ran fine and the GIF looked plausible. Check
frames, not exit codes.

## Recording mouse gestures

VHS has **no mouse at all** — the full command set is `Type`, `Ctrl`, arrows,
`Enter`, `Tab`, `PageUp`/`PageDown`, `ScrollUp`/`ScrollDown`, `Hide`, `Show`,
`Wait`, `Escape`, `Space`, `Source`, `Screenshot`, `Copy`, `Paste`. No motion,
no clicks. And it films a headless terminal, where there is no OS pointer to
film even if it could.

So the app draws one. `rataflow_examples::autopilot` runs a script of waypoints,
emits real `MouseEvent`s into the flow, and paints a cursor glyph at the pointer.
The caller gates it behind `RATAFLOW_DEMO=1` and binds it to an otherwise-unused
key, so a tape can fire it with `Type "g"`. Nothing ships to real users.

Two things make it read as a person rather than an animation:

- **Real events.** An earlier overview demo animated `set_node_position`
  directly. Nothing was actually being dragged, so there was no hit test, no
  click-vs-drag threshold, and the child never collided with its parent — it
  just slid. Synthesizing `Down`/`Drag`/`Up` runs the genuine state machine.
- **Easing and dwells.** `ease_in_out` rests at both endpoints, and the script
  pauses before pressing and after dropping. Linear motion, and the sine sweep
  the old demo used, both read as machinery because they never stop.

Waypoints come from `Flow::node_terminal_rect`, not hardcoded cells, so a script
survives the graph being laid out differently.

The pointer is a rat (U+1F400) — rataflow is built on ratatui and the pun is
free. Verify any glyph before relying on it: plenty do not render under VHS. A
throwaway tape that `echo`s the candidates and a `-coalesce`'d frame settles it
in a minute. The rat is double-width, so `draw` blanks the cell it spills into,
or whatever is underneath shows through its right half.

`Step::Scroll` emits wheel events at the cursor. The library zooms around the
pointer, so this reads as someone magnifying the thing they are looking at —
better than the keyboard `+` it replaced, which zooms about the viewport centre
and reads as a keypress.

The pointer is drawn plain, with no pressed state. Lighting the cell while the
button was down just read as an unexplained blue box — real cursors do not
change colour when you click, and what the button is doing is already legible
from the thing being dragged.

The overview script is five named actions in 6.9 seconds:

1. drag a nested child until it pins against its parent's edge
2. drag another node around a full circle, edges re-routing throughout
3. press empty canvas and drag to pan the viewport
4. wheel to zoom around the pointer
5. park the pointer clear of the graph

Two pacing lessons, both learned by watching the result rather than reading the
script:

- **Match the tape's `Sleep` to the script's runtime.** It said `9s` for a 6.9s
  script, and those 2.1 spare seconds — a still frame before the closing fit —
  were the single worst thing about the recording. Sum the durations and use
  that number.
- **Cut motion that adds no fact.** An early cut dragged the nested child back
  out of its clamp again, then dragged a second node in a straight line. Both
  demonstrated something the first push had already shown. One clamp plus one
  orbit says more in less time.
- **Find the empty cell, do not offset to it.** The pan beat pressed at a fixed
  offset from the parent node's corner, which happened to land inside the
  Sparkline — so it dragged that node instead of panning, and looked like a
  duplicate of the beat before it. A press only pans when it hits
  `MouseHit::Nothing`, so the script now walks every `node_terminal_rect` and
  searches for a cell that is clear at both ends of the throw.
- **Watch out for auto-pan.** `Step::Orbit` on the image node looked fine in the
  script and destroyed the recording: that node sits near the bottom-left, the
  loop carried the pointer into the five-cell auto-pan edge zone, and the canvas
  scrolled away mid-gesture. Orbit something with clearance on all sides — the
  Sparkline, not a corner node.
- **Zoom before the gesture, not after.** The edge-routing tape first orbited a
  node at fit scale and zoomed afterwards: the drag was a few pixels of movement
  in a dense grid, and the zoom then revealed a static picture. Whatever the
  recording is about has to be legible *while* it changes.
- **Show the rule, not just its outcomes.** That example renders 16 handle-position
  combos — every outcome of the routing rule, laid out at once. Dragging one
  target node around a loop shows the rule producing them, which is the thing a
  static grid can never carry.
- **A scripted gesture is only as real as what the app does with its events.**
  `DemoPilot::tick_into` first swallowed the `EventResponse` from
  `handle_mouse_event`, so `ConnectionCompleted` never reached the example that
  builds edges from it. Connections previewed perfectly and then vanished on
  release — the drag looked right and the graph came out unchanged. It returns
  the emitted events now, and callers that create things handle them exactly as
  they do for a real mouse.
- **Aim the accepted case at something that does not already exist.** The first
  version connected `source` to `target`, which edge `e1` already joins: success
  duplicated an edge sitting right there and looked like nothing happening. It
  targets `no_outgoing` now — whose *target* handle is plain, only its source
  handle being blocked — so accepting draws something new.
- **Do not let the pointer imply an interaction the app does not have.** The
  overview script used to glide the cursor to an "empty" cell before the tape
  pressed `f` — and the empty-cell search scans from the bottom-left, which is
  exactly where the Controls widget sits. The cursor came to rest on `[f]` as
  the view recentred, so it read as a click on a control that takes no clicks:
  Controls is keyboard-only. A recording that teaches a gesture the app does not
  support is worse than one that shows less. The fit is `Step::Fit` now, inside
  the script — nothing has to move for a viewport command, so nothing does, and
  the pointer simply stays on the node it just zoomed.
- **That rule is about the cursor, not about the keyboard.** It is easy to
  over-read into "every change must be caused by a visible pointer", and the
  overview script briefly was: the Input widget's two fields are node D's width
  and height, and an early cut demonstrated them by dragging D's resize grip
  instead of typing, on the theory that numbers changing with the cursor
  elsewhere would look like the app acting on its own. It does not. A widget
  that prints `Tab: switch, Enter: apply` across its own footer is legible
  without a cursor to point at it, and mouse resize is `node_flags`'s subject
  anyway. `Step::Key` types into it and the pointer stays where the last drag
  left it.
- **Count Tab presses from a known selection.** `SelectNext` walks *insertion*
  order (`A B C D E F G H`), not spatial order — the arrow keys are the spatial
  ones. The keyboard beat sits immediately after the drag because that drag's
  press selected D, putting F exactly two Tabs away. Almost anywhere else the
  selection is whatever the last press left behind, and the pan beat presses
  empty canvas, which clears it entirely.
- **Type a number the parent will refuse.** The beat asks for width 24 and the
  field settles on 15, because the apply clamps D inside C and the previous beat
  pinned D against C's right edge. That is the point: the correction is visible,
  and it is the containment rule stating itself. The first version typed a
  smaller width instead, which applied literally and left the node narrower with
  its label truncated further — accurate, and it read as damage.
- **Keep the caption and the recording in step.** The hub article's caption
  claimed "Tab-order navigation" — a beat that had been cut from the tape, and
  which was never Tab in the first place (the example binds `↑↓`). A caption is
  a claim about a GIF; re-read it whenever the tape changes.
- **`Orbit` uses `smoothstep`, not the cubic ease straight moves use.** Cubic is
  so front/back-loaded that a quarter of the way around the loop the node has
  travelled six degrees, which reads as a stall rather than a swing.

**`drag-and-connect`** is the article that could not be recorded at all before
the autopilot existed — its entire subject is a mouse gesture. It records
`validation`, and draws one connection that lands and one that is refused. The
refusal is the half worth the trouble: it leaves nothing behind, so a screenshot
of the aftermath shows an ordinary graph. The only way to show a rejection is to
show the attempt, previewed in red against a handle that will not take it.

**`widget-api-three-contracts`** has no motion to record; its media slot is
marked optional and wants a diagram, not a GIF.

## Recording a real terminal: Ghostty

One asset does not come from VHS. `overview` contains a `ratatui-image` node,
and VHS films **headless Chromium running xterm.js** — not your terminal.
xterm.js implements no image protocol, so that node degrades to half-blocks and
Ferris arrives as a blocky orange smear. Ghostty implements the Kitty graphics
protocol, so the same node draws as an actual image. Nothing else about the VHS
recording was wrong; this is one node's worth of fidelity.

```bash
cargo build --release --example overview
./assets/record-real-terminal.sh          # from inside the Ghostty window, hands off
```

`assets/overview.tape` stays as the fallback. It produces a correct recording
with a half-blocks crab, and it needs no Mac.

**What the trade costs.** VHS is hermetic: the tape pins the font, the size, the
theme and the frame rate, and anyone gets the same GIF. A screen capture is a
picture of *your machine*, so the window size, font and theme all leak into the
output. Size the window to roughly 1280x760 points first, so the frame needs no
downscaling and the text stays the size it was authored at.

**Match the grid, not the resolution.** A terminal app is laid out in cells, so
the character grid decides how much of the UI exists at all. `Set FontSize 13`
at 1280x760 gives the tape **153x48**, and rataflow truncates node labels to fit
whatever it is given. The first Ghostty capture ran at roughly 119x34 — an
everyday font size in the recording window — and produced a hero where the
Sparkline node read `ratatui Sparkli`, the buffer node was a bare `(`, and the
Input widget was an empty box. None of that looks broken on its own, which is
why the script prints the grid and warns below 150x44 rather than leaving it to
the eye. Font size moves the grid far more than window size does.

**Nothing types into it.** `RATAFLOW_DEMO=auto` starts the pointer script one
frame after layout, and `Step::Fit` ends it, so the whole demo runs without a
keystroke. That is not a convenience — synthesising a keypress means asking for
**Accessibility** permission on top of Screen Recording, and Accessibility is
the one that lets a process drive the entire machine. The auto path exists so
that permission is never needed. `RATAFLOW_DEMO_LEAD` is the settled-graph pause
before the pointer moves, which is also the window the recorder has to start in.

**The window rect is not the crop.** It includes the macOS title bar, Ghostty's
tab bar and the window border, and the first version of the script shipped all
three — a hero with the recording command sitting in the tab bar for its whole
ten seconds. Subtracting a fixed chrome height would be wrong on the next
machine: the tab bar only exists when a window has tabs, and everything doubles
on a retina panel. So the content rect is *derived* — an app frame and a blank
frame share identical chrome and differ across the whole terminal, so the
bounding box of the difference **is** the content rect, with no knowledge of
what the chrome looks like or whether there is any.

The same blank/app contrast times the clip. The script blanks the screen before
recording, so the video is blank / app / blank by construction, and
`blackdetect` on the cropped region returns the two boundaries directly. That
replaced a scene-detection guess that opened the GIF on a shell prompt.

**Record losslessly — `screencapture -v` is not good enough for a terminal.**
It writes H.264 **yuv420p**: chroma stored at half resolution in both axes. On a
photo that is invisible. On a terminal it is destructive, because a node border
is a *one pixel* coloured line on a dark background — it shares a chroma sample
with three dark pixels, the average desaturates, and it arrives grey. And it
does so **selectively**: survival depends on the line's pixel parity against the
2x2 chroma grid, so one recording comes back with some borders coloured and some
grey, and a node that moves mid-demo changes which.

That cost real time. A capture showed the nested child with three grey borders
and one blue one, immediately after the beat that resizes it, which reads
exactly like a rendering bug in the library. It was chased as one. What ruled it
out: the live app was correct, a VHS render of the *same* sequence was correct,
and the fault appeared in the MP4 as well as the GIF — so it was not palette
quantisation either. Everything downstream inherited it because both outputs are
derived from that one `.mov`.

`ffmpeg -f avfoundation` films the same screen and lets the pixel format be
named. `libx264rgb -qp 0` keeps full RGB (`gbrp`) and sustains 24fps at
2560x1440 without dropping frames. The intermediate is large and temporary.

**It also made the GIF three and a half times smaller**, which was not the point
but is the clearest evidence. Same content, same GIF settings, the only
difference being a 4:2:0 round-trip: **916 KB against 3.3 MB**.

Two measurements say why. Capturing one screen twice, the same region has
**2,172 colours** recorded losslessly and **12,789** through `screencapture` —
subsampled chroma is interpolated on the way back to RGB, and since a terminal
is nothing but high-contrast edges, every glyph and every one-pixel border gains
a halo of colours nobody drew. (A terminal frame is not a handful of colours to
begin with: font anti-aliasing accounts for those 2,172.)

The palette damage is real but secondary. The cost that dominates is that GIF
pays per *changed* pixel from frame to frame, and a demo like this is mostly
still. Between two frames inside the opening lead, where nothing whatsoever
moves, **0.045%** of pixels differ in the lossless capture and **0.167%** through
4:2:0 — nearly four times as many, in a scene that is not moving. That ratio is
the file-size ratio, near enough.

Beware of measuring this with a colour count taken from the GIF itself: a GIF
holds at most 256 colours by definition, so that number describes the format,
not the source. An earlier version of this note compared a GIF frame against an
MP4 frame and drew a confident conclusion from it. Compare sources, or compare
outputs, but not one of each.

Its device index is **not** the display number: avfoundation enumerates cameras
first and numbers screens from 0, while `screencapture -D` numbers from 1. The
script looks the index up from `-list_devices` rather than assuming it, because
a machine with a different number of cameras would otherwise film the wrong
screen without saying so.

The web MP4 is still `yuv420p` — 4:2:0 is what browsers decode — but it is now
one subsampling pass from a clean source instead of a second pass over already
grey pixels.

**The silent failure.** Without Screen Recording permission, `screencapture`
does not error. It returns an image of the desktop picture and the menu bar with
**every window missing** — which looks exactly like a capture that worked until
you open it. The tell is `CGPreflightScreenCaptureAccess()`, which the script
checks before recording anything.

Two things about that permission are worth knowing in advance:

- It attaches to **Ghostty**, not to `screencapture` or to the binary. TCC
  follows the parent app bundle, which is why running the same command from a
  different terminal asks a different app for permission.
- Once an app has been denied, macOS **never prompts again**.
  `CGRequestScreenCaptureAccess()` returns `false` immediately without showing
  anything. It has to be switched on by hand in Settings > Privacy & Security >
  Screen & System Audio Recording, and Ghostty has to be **quit and reopened** —
  the permission is only read at launch.

**Trimming is done by content, not by a stopwatch.** `screencapture -v` takes an
unpredictable moment to start writing frames, so a fixed offset drifts into the
opening drag on a slow run. The script finds the first scene cut instead — the
alt-screen switch, when the app takes over — and trims relative to that. Later
cuts are the demo's own motion, so only the first one is read.

The crop rect goes through three conversions and each is a place it silently
comes out wrong: `CGWindow` bounds are in **points** in a screen-global space,
`screencapture -D` writes **one display's** framebuffer, and a retina panel
doubles every number. `the window-rect helper inside it` does all three and prints the
result, so a wrong crop is visible before the recording starts rather than
after.

## Inspecting output

GIF frames are **delta-encoded**, so extracting one frame directly gives you the
changed region on a blank field:

```bash
magick 'assets/overview.gif[60]' /tmp/frame.png     # mostly blank — not a bad recording
magick assets/overview.gif -coalesce /tmp/f.png     # composited frames: /tmp/f-0.png, f-1.png, …
```

Always `-coalesce` before judging a frame. The first time this comes up it reads
as a failed record.

## MP4 for the blog, GIF for the repo

The same recordings ship twice, because the destinations disagree about video.

```bash
./assets/build.sh mp4     # the tape GIFs -> assets/*.mp4
./assets/build.sh sync    # copies GIFs and MP4s to the blog
```

**The README cannot use video.** GitHub's markdown sanitizer strips `<video>`
and `<iframe>`, and `![](assets/demo.mp4)` does not render. The one thing that
does work is a bare `user-attachments` URL on its own line — GitHub auto-embeds
a player for it — but that file is uploaded through the web UI, lives outside
the repo, and cannot be regenerated by any script. It is also invisible on
**crates.io**, which is the other place this README is read and which renders no
video at all. So the repo keeps GIFs.

**A page has no such limit, and the saving is real.** Across the five demos the
blog drops from ~2.0 MB to ~670 KB.

| | GIF | MP4 |
| --- | --- | --- |
| `overview` | ~915 KB | ~340 KB |
| `custom-edges` | 244 KB | 105 KB |
| `edge-routing` | 236 KB | 106 KB |
| `negative-pixels` | 480 KB | 72 KB |
| `drag-and-connect` | 112 KB | 52 KB |

Note the hero's GIF is no longer the outlier it was. It weighed 3 MB until the
recorder was fixed — see the lossless-capture note above, and the measurement
below.

`build.sh mp4` skips the hero on purpose: **`overview.mp4` comes from
`record-real-terminal.sh`**, encoded from the lossless screen recording rather than
from the GIF. Re-encoding a GIF means spending bitrate on a 256-colour
quantisation — bigger and worse. The capture script already has the raw `.mov`
and the same crop and trim numbers, so it writes both in one pass.

There is no longer an `og.gif`. The social card's still is
`assets/og-shot.png`, written directly by `assets/og.tape` through VHS's
`Screenshot` directive. A note in that tape used to claim `Screenshot`
"silently no-ops in 0.11.0" and worked around it by recording an animated GIF
and taking its last frame. It does not no-op: it needs an `Output` directive
present (`Screenshot` alone is ignored) and a relative path, since VHS's parser
rejects absolute ones — the same trap that bites `Output`. The workaround cost
real quality, because the card was being assembled from **255** quantised
colours when the screenshot carries about **10,000**. For the one image that
represents the project in every feed, that was the wrong economy.

`yuv420p`, not the sharper `yuv444p`: 4:4:4 keeps coloured text crisp, but
Safari and most hardware decoders refuse it, and a demo that does not play beats
one that is slightly soft. 4:2:0 also cannot represent an odd width, and ffmpeg
errors rather than rounding — hence the `trunc(iw/2)*2` in the filter.

On the blog side `<Demo>` renders `<video autoplay muted loop playsinline>` with
the GIF as inner fallback content. All four attributes are load-bearing: mobile
Safari refuses to autoplay unmuted, and without `playsinline` it goes fullscreen
instead of playing in place. The MP4 path is derived from the GIF path rather
than passed separately, so the two cannot be named inconsistently.

## Serving GIFs on the website

The rataflow site is Astro, so anything animated needs a copy under
`web/public/` and a plain `<img>` — Astro's image pipeline (sharp)
**flattens an animated GIF to a single frame**, so it cannot go through
`<Image>`. Keep the copies in sync:

```bash
md5 -q assets/overview.gif web/public/overview.gif | uniq | wc -l   # want 1
```

## The social card

The card is RENDERED, not screenshotted. The reason is size: a card is viewed at
roughly 500px wide in a feed, and at that width a 1200x630 shot of a terminal is
a grey blur — every label unreadable, and the word "rataflow" nowhere on it,
because the example's sidebar says "Overview". So the name and the one-line
pitch are set large enough to survive the shrink, and the real UI appears as a
crop rather than the whole dense frame.

`web/scripts/og.mjs` does the whole job: it pulls the last frame out of
`assets/og.gif`, crops it, lays the card out with satori (flexbox to SVG) and
rasterises it with resvg.

```bash
cargo build --example overview --release
vhs assets/og.tape
cd web && pnpm og        # -> assets/og.png AND web/public/og.png
```

There is no crop any more. Cropping was the first design and it cost too much:
an 830x420 window on rataflow's graph dropped the image node, the tachyonfx
node, the input widget and the minimap, and zoetrope's 1160x330 window dropped
the prompt timeline and status bar outright. Whatever gets cut is the part
someone never learns exists.

So the whole frame is the background and the words sit on top of a scrim. The
screenshot no longer has to carry the message by itself, which is what made
legibility fight coverage.

Two things about the scrim, both learned by looking at the render:

- **Cover the boring half.** The gradient runs opaque from the right, because
  rataflow's graph sits on the left of the frame and the example's sidebar (a
  column of help text) sits on the right. The first cut had it the other way
  round and hid every node while revealing the help.
- **Leave the far side nearly clear.** At 0.45 alpha the graph read as a smudge;
  0.04 lets it through while the headline stays readable over the opaque end.
- **Lift the screenshot itself, not just the scrim.** A terminal capture is
  nearly black by nature, so even a light scrim left it sooty and the accent
  colours in it did nothing. `sharp.modulate({ brightness: 1.22, saturation:
  1.35 })` before embedding. Brightness on its own only greys it out; the
  saturation is what keeps the colours colours.

The accent carries the card at thumbnail size. Coloured text alone vanished, so
one headline line is INVERTED — dark text on a solid accent block, the way a
terminal draws a selection — and a 12px accent rule runs along one full edge.

Which edge is per-project, and it matters: zoetrope's status bar (carrying its
wordmark) runs along the very bottom of the frame, and a bottom rule sliced it
in half. Its rule is on the top edge; rataflow's frame is empty down there, so
its rule stays at the bottom.

sharp reads the GIF frame through libvips, which applies frame disposal, so the
script gets a composited frame rather than the delta tile a raw read would give.
That is the same trap as `-coalesce` when inspecting GIFs by hand.

Fonts are vendored at `assets/fonts/JetBrainsMono-{Regular,Bold}.ttf`: satori
needs font bytes, and neither Google Fonts nor Astro's font cache hands over a
usable TTF.

Layout.astro builds the absolute `og:image` URL from `site` in astro.config, so
the card can never be advertised from a preview deploy's hostname.

## VHS gotchas

1. **`Output` is mandatory, even for a still.** VHS will not run a tape without
   it, so asking only for a `Screenshot` still writes a GIF you did not want.
2. **The path parser rejects absolute paths outright** — not just gnarly ones.
   `Screenshot /tmp/ce.png` fails with `Invalid command: ce.png`, and that is a
   two-character path. Write to a relative path from the repo root and move the
   file afterwards.
3. **`Screenshot` silently does nothing** in the installed version. The tape
   runs, VHS reports success, and no PNG appears. To grab a still, record a
   short tape and pull a frame out of the GIF with `-coalesce` instead.
4. **Clean up throwaway tapes.** A scratch tape writes its `Output` GIF next to
   it, which is how a stray 794 KB `shot.gif` ended up committed in zoetrope.
   Delete both when done.
5. **Hide the launch and the quit.** Wrap the `Type "target/release/…"` + `Enter`
   in `Hide` / `Show` so the shell prompt never lands in frame, and hide the
   trailing `q`.
