/**
 * Renders the 1024x1024 master app icon from the same branch glyph as the tray.
 *
 * Unlike the tray icon this one needs a field behind it: a monoline glyph on
 * transparency would vanish against the Start Menu and the installer chrome.
 *
 *   node scripts/make-app-icon.mjs && npx tauri icon app-icon.png
 *
 * The second command writes the whole set — .ico, .icns and every Square*Logo —
 * into src-tauri/icons/.
 */
import { chromium } from "playwright";
import { writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const OUT = join(ROOT, "app-icon.png");
const SIZE = 1024;

/** Straight from `Tray Icon.dc.html`. */
const GLYPH = `
<circle cx="4.5" cy="3.5" r="1.75"></circle>
<circle cx="4.5" cy="12.5" r="1.75"></circle>
<circle cx="11.5" cy="5" r="1.75"></circle>
<path d="M4.5 5.25v5.5M11.5 6.75c0 3-2.6 3.6-5.6 4.4"></path>`;

const browser = await chromium.launch();
const ctx = await browser.newContext({ viewport: { width: SIZE, height: SIZE }, deviceScaleFactor: 1 });
const tab = await ctx.newPage();

await tab.setContent(`<!doctype html><meta charset="utf-8">
<style>
  html,body{margin:0;background:transparent}
  /* The dark swatch the design shows the glyph on, with the corner radius
     Windows and macOS both expect of an app tile. */
  #c{width:${SIZE}px;height:${SIZE}px;border-radius:${SIZE * 0.22}px;background:#202020;
     display:grid;place-items:center}
</style>
<div id="c">
  <svg width="${SIZE * 0.62}" height="${SIZE * 0.62}" viewBox="0 0 16 16" fill="none"
       stroke="#fff" stroke-width="1.4" stroke-linecap="round">${GLYPH}</svg>
</div>`);

await writeFile(OUT, await tab.locator("#c").screenshot({ omitBackground: true }));
await browser.close();
console.log(`wrote ${OUT} at ${SIZE}x${SIZE}`);
