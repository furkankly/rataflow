import { execFileSync } from "node:child_process";
import sitemap from '@astrojs/sitemap';
import { defineConfig } from "astro/config";
import tailwindcss from "@tailwindcss/vite";

// ── <lastmod>, from git rather than from the clock ───────────────────────────
//
// The sitemap shipped a bare <loc>, which gives a crawler no reason to come
// back to a page it has already seen — the thing that matters after an edit it
// needs to notice.
//
// The date is the last commit that touched what the page is built from, NOT the
// build time. `new Date()` is the easy version and it is a lie: the page would
// claim to have changed on every deploy, and Google's guidance is that it uses
// lastmod when a site reports it consistently and accurately, so a sitemap that
// cries wolf is worth less than no lastmod at all.
//
// This is a single-page site, so "what the page is built from" is the page, its
// layout and every component it pulls in — all of which inline into the one
// document a crawler fetches.
const SOURCES = ["src/pages", "src/layouts", "src/components"];

// A shallow clone (actions/checkout's default fetch-depth: 1, and Vercel's own
// clone) has one commit, so every file would date to that commit and the answer
// would be the build time wearing a disguise. Drop lastmod rather than emit a
// wrong one.
const shallow = (() => {
  try {
    return execFileSync("git", ["rev-parse", "--is-shallow-repository"], {
      cwd: import.meta.dirname,
      encoding: "utf8",
    }).trim() === "true";
  } catch {
    return true; // no git at all — same conclusion, no lastmod
  }
})();

if (shallow) {
  console.warn("[sitemap] shallow or missing git history — emitting no <lastmod>");
}

const lastmod = (() => {
  if (shallow) return undefined;
  try {
    const iso = execFileSync("git", ["log", "-1", "--format=%cI", "--", ...SOURCES], {
      cwd: import.meta.dirname,
      encoding: "utf8",
    }).trim();
    return iso || undefined;
  } catch {
    return undefined;
  }
})();

export default defineConfig({
  // Canonical public origin — drives <link rel="canonical"> and og:url in Layout.astro.
  site: "https://rataflow.furkankly.dev",
  integrations: [
    sitemap({
      serialize(item) {
        if (lastmod) item.lastmod = lastmod;
        return item;
      },
    }),
  ],
  vite: {
    plugins: [tailwindcss()],
  },
});
