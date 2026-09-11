// The example registry, read at build time from the Rust that defines it.
//
// Three things go into an example page and all three already exist in the repo;
// none of them is authored here:
//
//   title / description / keys  `rataflow_examples::meta`, via the
//                               `web-examples` bin. The same values the app
//                               paints into its sidebar.
//   module doc                  the `//!` block at the top of examples/<x>.rs.
//                               Thirteen to twenty-eight lines of real prose
//                               that, until these routes existed, was visible
//                               only to someone reading the source on GitHub.
//   source                      the rest of that file.
//
// The split matters for what the page may show. The description is ALREADY on
// screen — the wasm draws it in the sidebar — so it goes in <head> only, where
// it is a snippet for a crawler and a subtitle for a link unfurl and is never
// rendered twice. The module doc and the code are on screen nowhere, which is
// what makes them the honest thing to put in the body.

import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";

/**
 * The repository root, found by walking up from the working directory.
 *
 * Not `import.meta.dirname`: Astro bundles this module into `dist/.prerender/`
 * before running it, so a path relative to the source file points into the
 * build output. The working directory is stable (`web/` for `pnpm build`, the
 * root if someone runs it from there), and `examples/shared` is a thing only
 * this repository has.
 */
function repoRoot(): string {
  let dir = process.cwd();
  for (let i = 0; i < 6; i++) {
    if (existsSync(join(dir, "examples", "shared", "Cargo.toml"))) return dir;
    const up = dirname(dir);
    if (up === dir) break;
    dir = up;
  }
  throw new Error(`could not find the rataflow root above ${process.cwd()}`);
}

const REPO = repoRoot();

export interface Example {
  slug: string;
  source: string;
  /** The port the browser runs, when it is a different program. */
  webSource: string | null;
  title: string;
  /** `null` for `events`, which draws its own panel instead of the sidebar. */
  description: string | null;
  keys: [string, string][];
  /** The `//!` header, controls section removed, `//!` markers stripped. */
  doc: string;
  /**
   * What the browser does differently, from the port's own module doc.
   *
   * `null` unless `webSource` is set. No new prose anywhere: the file that
   * diverges is the one that explains the divergence, and it already did.
   */
  webNote: string;
  /** Everything after the header. */
  code: string;
}

/** The registry, straight out of Rust — never parsed out of the source by hand. */
function registry(): Omit<Example, "doc" | "code">[] {
  const json = execFileSync(
    "cargo",
    ["run", "-q", "-p", "rataflow-examples", "--bin", "web-examples"],
    { cwd: REPO, encoding: "utf8", maxBuffer: 8 * 1024 * 1024 },
  );
  return JSON.parse(json);
}

/**
 * Split an example into its module doc and its code.
 *
 * The header is the run of `//!` lines the file opens with — Rust only allows
 * them there, so the first line that is not one ends it. Inner attributes
 * (`#![...]`) can sit in the same block and are code, not prose.
 */
function split(rs: string): { doc: string; code: string } {
  const lines = rs.split("\n");
  const doc: string[] = [];
  let i = 0;
  for (; i < lines.length; i++) {
    const line = lines[i];
    if (line.startsWith("//!")) {
      doc.push(line.slice(3).replace(/^ /, ""));
    } else if (line.trim() === "" && doc.length && lines[i + 1]?.startsWith("//!")) {
      doc.push(""); // a blank line inside the header
    } else {
      break;
    }
  }
  return { doc: doc.join("\n").trim(), code: lines.slice(i).join("\n").trim() };
}

/**
 * Drop the doc's controls section.
 *
 * NOT an accuracy fix — a duplication one. The wasm draws the key list in its
 * own sidebar, two inches above this drawer, from `meta.rs`. That list is the
 * authoritative one and it is already web-correct: `q` is deliberately absent
 * because a browser tab has nothing to quit, and native binaries add it back
 * with `ExampleMeta::with_quit`. Rendering the doc's copy in HTML puts a second,
 * contradicting list on the same screen — which is how "press q to quit" was
 * reaching the page at all.
 *
 * Two shapes, because the examples use both: an explicit `Controls:` label, or
 * a bare trailing run of `- <key>: <action>` items.
 *
 * The second is a RUN, not a whole list, because several examples put both in
 * one list — `basic` opens with three prose items ("Drag node body to move it")
 * and then slides into eight bindings. Walking back only over binding-shaped
 * items keeps the prose and takes the bindings. A list that ends in prose is
 * left entirely alone, which is what protects things like floating_edges'
 * numbered attachment modes.
 */
function stripControls(doc: string): string {
  // A bare label, or one promoted to a rustdoc heading (`## Controls`).
  const CONTROLS = /^\s*#{0,4}\s*(controls|keys|key bindings|bindings)\s*:?\s*$/i;
  // `- f: fit view`, `- h/j/k/l: pan viewport`, `- Delete/Backspace: delete`.
  const BINDING = /^\s*-\s+[^:]{1,20}:\s/;

  const lines = doc.split("\n");

  const header = lines.findIndex((l) => CONTROLS.test(l));
  if (header !== -1) return lines.slice(0, header).join("\n").trim();

  let start = lines.length;
  for (let i = lines.length - 1; i >= 0; i--) {
    const line = lines[i];
    if (line.trim() === "" && start < lines.length) continue;
    if (!BINDING.test(line)) break;
    start = i;
  }
  return start === lines.length ? doc : lines.slice(0, start).join("\n").trim();
}

let cache: Example[] | undefined;

export function examples(): Example[] {
  if (cache) return cache;
  cache = registry().map((entry) => {
    const { doc, code } = split(readFileSync(join(REPO, entry.source), "utf8"));
    const webNote = entry.webSource
      ? split(readFileSync(join(REPO, entry.webSource), "utf8")).doc
      : "";
    return { ...entry, doc: stripControls(doc), code, webNote };
  });
  return cache;
}

/**
 * The overview is served at `/`, not at `/examples/overview/`.
 *
 * It is the example the site opens on, so `/` already shows it; generating a
 * second URL for the same demo would be two pages competing to be the result
 * for one thing. `/#overview` redirects to `/` for the same reason.
 */
export const ROOT_SLUG = "overview";

export function routed(): Example[] {
  return examples().filter((e) => e.slug !== ROOT_SLUG);
}

/** Every slug the app answers to, including the one served at `/`. */
export function allSlugs(): string[] {
  return examples().map((e) => e.slug);
}

/**
 * The one-line summary for `<head>`.
 *
 * The sidebar description is two sentences separated by a newline; its first is
 * the summary and its second is usually a detail about the setup. `events` has
 * no description at all, so its module doc's first line stands in — which is
 * the reason to read the doc even for pages that do not show much of it.
 */
export function summary(e: Example): string {
  const first = (e.description ?? e.doc).split("\n")[0]?.trim() ?? "";
  return first.replace(/\s+/g, " ");
}

/**
 * The module doc as HTML.
 *
 * Not a markdown renderer: this is rustdoc, and the two disagree in the one way
 * that matters here. An intra-doc link points at a Rust path, which is not a
 * URL, so markdown would emit a dead link — the target is dropped and the name
 * kept as code instead. What is left is the small amount of structure these
 * headers actually use: paragraphs, dash and numbered lists, inline code.
 */
export function docHtml(doc: string): string {
  const esc = (s: string) =>
    s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

  const CODE = "`";
  const inline = (s: string) =>
    esc(s)
      // [`Name`](rataflow::Name) and [`Name`] — keep the name, drop the path.
      .replace(new RegExp("\\[" + CODE + "([^" + CODE + "]+)" + CODE + "\\]\\([^)]*\\)", "g"), "<code>$1</code>")
      .replace(new RegExp("\\[" + CODE + "([^" + CODE + "]+)" + CODE + "\\]", "g"), "<code>$1</code>")
      .replace(new RegExp(CODE + "([^" + CODE + "]+)" + CODE, "g"), "<code>$1</code>");

  const out: string[] = [];
  let list: { ordered: boolean; items: string[] } | null = null;

  const flush = () => {
    if (!list) return;
    const tag = list.ordered ? "ol" : "ul";
    out.push("<" + tag + ">" + list.items.map((i) => "<li>" + i + "</li>").join("") + "</" + tag + ">");
    list = null;
  };

  for (const block of doc.split(/\n\s*\n/)) {
    let para: string[] = [];
    const flushPara = () => {
      if (para.length) out.push("<p>" + para.join(" ") + "</p>");
      para = [];
    };

    for (const line of block.split("\n")) {
      const heading = /^\s*(#{1,4})\s+(.*)$/.exec(line);
      const dash = /^\s*-\s+(.*)$/.exec(line);
      const num = /^\s*\d+\.\s+(.*)$/.exec(line);
      if (heading) {
        flushPara();
        flush();
        // Rustdoc's section heading is `##`, the level it renders as `h2` in
        // the generated docs; the page's only other heading is the wordmark's
        // `h1`, so mapping it to `h2` puts these directly under it with no
        // level skipped. Deeper ones step down from there.
        const level = Math.min(Math.max(heading[1].length, 2), 6);
        out.push(`<h${level}>${inline(heading[2])}</h${level}>`);
      } else if (dash || num) {
        flushPara();
        const ordered = !!num;
        if (!list || list.ordered !== ordered) {
          flush();
          list = { ordered, items: [] };
        }
        list.items.push(inline((dash ?? num)![1]));
      } else if (line.trim() === "") {
        continue;
      } else if (list) {
        // Indented continuation of the item above it.
        list.items[list.items.length - 1] += " " + inline(line.trim());
      } else {
        para.push(inline(line.trim()));
      }
    }
    flushPara();
    flush();
  }
  return out.join("\n");
}
