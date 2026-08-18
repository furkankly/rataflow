// Renders the social card: assets/og.png, copied to web/public/og.png.
//
//   node web/scripts/og.mjs            (run from the repo root)
//
// Why a rendered card rather than the raw screenshot it replaced: a card is
// viewed at roughly 500px wide in a feed, and at that size a 1200x630 shot of a
// terminal is a grey blur. Every label in it was unreadable, and the word
// "rataflow" appeared nowhere on it — the example's sidebar says "Overview".
// So the name and the one-line pitch are set at a size that survives, and the
// real UI appears as a CROP rather than the whole dense frame.
//
// The picture is assets/og-shot.png, written directly by assets/og.tape via
// VHS's Screenshot directive, so the card shows the actual library and one
// command regenerates the whole thing. See docs/DEMO-ASSETS.md.
//
// satori lays out flexbox and emits SVG; resvg rasterises it. Satori rules that
// bite: every element with more than one child needs an explicit `display:flex`,
// there is no line-clamp, and images must be data URIs (no network, no paths).

import { readFileSync, writeFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import satori from "satori";
import { Resvg } from "@resvg/resvg-js";
import sharp from "sharp";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = join(HERE, "..", "..");

const W = 1200;
const H = 630;

// The site's own palette (web/src/styles/global.css) so the card and the page
// it links to look like one thing.
const INK = "#0a0c10";
const PANEL = "#161922";
const LINE = "#2a3040";
const BLUE = "#5fafff";
const DIM = "#545d6e";
const TEXT = "#c9d1d9";

const font = (f) => readFileSync(join(REPO, "assets", "fonts", f));

// The screenshot inset, cut straight out of the tape recording.
//
// The WHOLE frame. Cropping was the first attempt and it cost too much: a
// 830x420 window on the graph dropped the image node, the tachyonfx node, the
// input widget and the minimap — the card showed a sparse corner of a dense
// tool. Whatever gets cut is the part someone doesn't learn exists.
//
// So the frame is the background and the words sit on top of a scrim. The
// screenshot no longer has to carry the message on its own, which is what made
// legibility fight coverage in the first place.
const CROP = null;

// A true-colour PNG, not a GIF frame. The previous version pulled the last page
// out of an animated GIF, which meant the card was assembled from 255 quantised
// colours; the screenshot carries about 10,000.
async function shotDataUri() {
  const shot = join(REPO, "assets", "og-shot.png");
  let img = sharp(shot);
  if (CROP) img = img.extract(CROP);
  // Lift it. A terminal screenshot is nearly black by nature, and under a scrim
  // it went flat and sooty — the accent colours in the graph (the sparkline's
  // gold, the image node's orange) were doing nothing. Brightness alone greys
  // it out, so saturation comes up with it and the colours stay colours.
  img = img.modulate({ brightness: 1.22, saturation: 1.35 });
  const png = await img.png().toBuffer();
  return `data:image/png;base64,${png.toString("base64")}`;
}

const shot = await shotDataUri();

// The mascot, rasterised here rather than embedded as SVG. Satori will take an
// SVG data URI but rasterises it at its own scale, and the mark has 2px strokes
// that go to mush when that guess comes in low; rendering it at 3x and letting
// satori scale a bitmap down keeps them crisp.
function markDataUri(px) {
  const svg = readFileSync(join(REPO, "assets", "icon.svg"), "utf8");
  const png = new Resvg(svg, { fitTo: { mode: "width", value: px * 3 } })
    .render()
    .asPng();
  return `data:image/png;base64,${png.toString("base64")}`;
}

const MARK_PX = 46;
const mark = markDataUri(MARK_PX);

const card = {
  type: "div",
  props: {
    style: {
      width: "100%",
      height: "100%",
      display: "flex",
      position: "relative",
      backgroundColor: INK,
      fontFamily: "JetBrains Mono",
    },
    children: [
      // The real UI, full bleed.
      {
        type: "img",
        props: {
          src: shot,
          style: { position: "absolute", left: 0, top: 0, width: W, height: H },
        },
      },
      // Scrim: opaque on the RIGHT, where the words go, clearing toward the
      // left where the graph is. First cut had it the other way round and
      // covered the nodes while revealing the example's sidebar — a column of
      // help text, i.e. the least interesting part of the frame.
      {
        type: "div",
        props: {
          style: {
            position: "absolute",
            left: 0,
            top: 0,
            width: W,
            height: H,
            backgroundImage: `linear-gradient(260deg, ${INK} 0%, ${INK} 30%, rgba(10,12,16,0.86) 44%, rgba(10,12,16,0.04) 100%)`,
          },
        },
      },
      // Full-width accent rule on the bottom edge. Reads as a chrome detail up
      // close and as a colour signature at thumbnail size.
      {
        type: "div",
        props: {
          style: {
            position: "absolute",
            left: 0,
            bottom: 0,
            width: W,
            height: 12,
            backgroundColor: BLUE,
          },
        },
      },
      {
        type: "div",
        props: {
          style: {
            position: "relative",
            display: "flex",
            flexDirection: "column",
            justifyContent: "center",
            alignItems: "flex-end",
            marginLeft: "auto",
            width: 690,
            height: H,
            padding: "0 56px",
            textAlign: "right",
          },
          children: [
            {
              type: "div",
              props: {
                style: {
                  display: "flex",
                  alignItems: "center",
                  gap: 16,
                  marginBottom: 26,
                },
                children: [
                  {
                    type: "img",
                    props: {
                      src: mark,
                      width: MARK_PX,
                      height: MARK_PX,
                      style: { display: "flex" },
                    },
                  },
                  {
                    type: "div",
                    props: {
                      style: { fontSize: 30, color: BLUE, fontWeight: 700 },
                      children: "rataflow",
                    },
                  },
                  {
                    type: "div",
                    props: {
                      style: {
                        display: "flex",
                        flex: 1,
                        height: 2,
                        backgroundColor: LINE,
                      },
                    },
                  },
                  {
                    type: "div",
                    props: {
                      style: { fontSize: 17, color: DIM },
                      children: "a rust crate",
                    },
                  },
                ],
              },
            },
            {
              type: "div",
              props: {
                style: { fontSize: 50, color: TEXT, fontWeight: 700, lineHeight: 1.16 },
                children: "Node-based UIs",
              },
            },
            // The accent line is INVERTED rather than merely coloured: dark
            // text on a solid block, the way a terminal draws a selection. A
            // feed is mostly light cards, so a dark card needs one saturated
            // shape to hold a thumbnail's worth of attention. Coloured text
            // alone disappeared at that size.
            {
              type: "div",
              props: {
                style: {
                  display: "flex",
                  alignSelf: "flex-end",
                  backgroundColor: BLUE,
                  color: INK,
                  fontSize: 50,
                  fontWeight: 700,
                  lineHeight: 1.16,
                  padding: "4px 16px",
                  marginTop: 6,
                },
                children: "for the terminal",
              },
            },
            {
              type: "div",
              props: {
                style: { fontSize: 19, color: DIM, marginTop: 22, lineHeight: 1.5 },
                children: "built on ratatui · inspired by xyflow",
              },
            },
          ],
        },
      },
    ],
  },
};

const svg = await satori(card, {
  width: W,
  height: H,
  fonts: [
    { name: "JetBrains Mono", data: font("JetBrainsMono-Regular.ttf"), weight: 400, style: "normal" },
    { name: "JetBrains Mono", data: font("JetBrainsMono-Bold.ttf"), weight: 700, style: "normal" },
  ],
});

const png = new Resvg(svg, { fitTo: { mode: "width", value: W } })
  .render()
  .asPng();

writeFileSync(join(REPO, "assets", "og.png"), png);
writeFileSync(join(REPO, "web", "public", "og.png"), png);
console.log(`og.png ${W}x${H} (${png.length} bytes) -> assets/ and web/public/`);
