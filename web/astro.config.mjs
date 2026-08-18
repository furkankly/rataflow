import sitemap from '@astrojs/sitemap';
import { defineConfig } from "astro/config";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  // Canonical public origin — drives <link rel="canonical"> and og:url in Layout.astro.
  site: "https://rataflow.furkankly.dev",
  integrations: [sitemap()],
  vite: {
    plugins: [tailwindcss()],
  },
});
