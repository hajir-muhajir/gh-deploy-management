/**
 * Renders the system tray icon set from the Claude Design source.
 *
 * Windows has no badge API for tray icons, so every badge has to be baked into
 * the bitmap. Rasterising SVG at runtime would mean a new Rust dependency for a
 * 16px image, and hand-drawing the glyph in Rust would mean writing antialiasing
 * and a bitmap font. Rendering once, here, keeps the result pixel-identical to
 * the design because it is drawn by the same engine the design was drawn in.
 *
 * Folder names describe the TASKBAR, not the glyph: `on-light` holds the dark
 * glyph meant for a light taskbar.
 *
 *   node scripts/make-tray-icons.mjs
 */
import { chromium } from "playwright";
import { mkdir, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const OUT = join(ROOT, "src-tauri", "icons", "tray");

/** 4x the 16px logical size, so Windows still has pixels to downscale from. */
const SIZE = 64;
const SCALE = SIZE / 16;

/** Straight from `Tray Icon.dc.html`. */
const GLYPH = `
<circle cx="4.5" cy="3.5" r="1.75"></circle>
<circle cx="4.5" cy="12.5" r="1.75"></circle>
<circle cx="11.5" cy="5" r="1.75"></circle>
<path d="M4.5 5.25v5.5M11.5 6.75c0 3-2.6 3.6-5.6 4.4"></path>`;

const THEMES = {
  // `ring` approximates the taskbar behind the icon, matching the design's
  // `box-shadow: 0 0 0 1.5px var(--strip)` that separates badge from glyph.
  "on-light": { fg: "#1a1a1a", ring: "#f3f3f3" },
  "on-dark": { fg: "#ffffff", ring: "#202020" },
};

const BADGE_COLORS = { failing: "#FF3B30", review: "#FF9500" };
const DOT_COLOR = "#0A84FF";

/**
 * The design puts the badge outside the glyph's bounds (`top:3 right:2` in a
 * 34x36 box around a 16px icon). A tray icon has no outside, so the glyph gives
 * up the corner instead — but only when there is something to put there. Idle
 * and running use the whole canvas, which matters at 16px where every pixel of
 * the glyph counts.
 */
const GLYPH_SCALE = { plain: 1, badged: 0.8 };

function page({ fg, ring }, badge, dot) {
  const glyphPx = SIZE * (badge ? GLYPH_SCALE.badged : GLYPH_SCALE.plain);
  const badgePx = SIZE * 0.47;
  const fontPx = Math.round(SIZE * 0.3);
  return `<!doctype html><meta charset="utf-8">
<style>
  html,body{margin:0;background:transparent}
  #c{position:relative;width:${SIZE}px;height:${SIZE}px}
  #g{position:absolute;left:0;bottom:0}
  #b{position:absolute;top:0;right:0;height:${badgePx}px;min-width:${badgePx}px;
     box-sizing:border-box;padding:0 ${Math.round(SIZE * 0.055)}px;
     border-radius:${badgePx / 2}px;background:${badge?.color};color:#fff;
     font:700 ${fontPx}px/${badgePx}px -apple-system,"Segoe UI",system-ui,sans-serif;
     text-align:center;box-shadow:0 0 0 ${SIZE * 0.047}px ${ring}}
  #d{position:absolute;right:${SIZE * 0.055}px;bottom:${SIZE * 0.055}px;
     width:${SIZE * 0.22}px;height:${SIZE * 0.22}px;border-radius:50%;
     background:${DOT_COLOR};box-shadow:0 0 0 ${SIZE * 0.047}px ${ring}}
</style>
<div id="c">
  <svg id="g" width="${glyphPx}" height="${glyphPx}" viewBox="0 0 16 16" fill="none"
       stroke="${fg}" stroke-width="1.5" stroke-linecap="round">${GLYPH}</svg>
  ${badge ? `<div id="b">${badge.text}</div>` : ""}
  ${dot ? `<div id="d"></div>` : ""}
</div>`;
}

/** 1..9 render as themselves; anything more is unreadable at 16px. */
const COUNTS = [...Array(9)].map((_, i) => [String(i + 1), String(i + 1)]).concat([["9plus", "9+"]]);

const browser = await chromium.launch();
const ctx = await browser.newContext({ viewport: { width: SIZE, height: SIZE }, deviceScaleFactor: 1 });
const tab = await ctx.newPage();

let written = 0;
for (const [theme, colors] of Object.entries(THEMES)) {
  await mkdir(join(OUT, theme), { recursive: true });

  const shoot = async (name, badge, dot) => {
    await tab.setContent(page(colors, badge, dot));
    const png = await tab.locator("#c").screenshot({ omitBackground: true });
    await writeFile(join(OUT, theme, `${name}.png`), png);
    written++;
  };

  await shoot("idle", null, false);
  await shoot("running", null, true);
  for (const [kind, color] of Object.entries(BADGE_COLORS)) {
    for (const [slug, text] of COUNTS) {
      await shoot(`${kind}-${slug}`, { color, text }, false);
    }
  }
}

await browser.close();
console.log(`wrote ${written} icons to ${OUT} at ${SIZE}x${SIZE} (scale ${SCALE}x)`);
