// Browser smoke test for the production build: boots the game in headless Chromium,
// checks the download budget, that it starts quickly without errors, and that the bike
// actually rides. Run after `npm run build`: npm run smoke
import { readdirSync, statSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { preview } from "vite";

const root = fileURLToPath(new URL("..", import.meta.url));
const MAX_DOWNLOAD_BYTES = 10 * 1024 * 1024; // GDD budget: first download under 10 MB
const MAX_BOOT_MS = 5000; // GDD budget: playable within 5 s

function dirSize(dir) {
  return readdirSync(dir).reduce((sum, name) => {
    const p = join(dir, name);
    const s = statSync(p);
    return sum + (s.isDirectory() ? dirSize(p) : s.size);
  }, 0);
}

const failures = [];
const check = (ok, message) => {
  console.log(`${ok ? "ok  " : "FAIL"} ${message}`);
  if (!ok) failures.push(message);
};

const bytes = dirSize(join(root, "dist"));
check(bytes < MAX_DOWNLOAD_BYTES, `build is ${(bytes / 1024 / 1024).toFixed(2)} MB (budget 10 MB)`);

const server = await preview({ root, preview: { port: 4174, strictPort: false }, logLevel: "error" });
const url = server.resolvedUrls.local[0];
const browser = await chromium.launch({
  args: ["--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"],
});
try {
  const page = await browser.newPage({ viewport: { width: 1280, height: 720 } });
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  page.on("console", (m) => m.type() === "error" && errors.push(m.text()));

  const t0 = Date.now();
  await page.goto(`${url}?bots=3&quality=1`);
  await page.waitForFunction(() => window.bikePark !== undefined, null, { timeout: 20000 });
  const bootMs = Date.now() - t0;
  check(bootMs < MAX_BOOT_MS, `game ready in ${bootMs} ms`);

  const riders = await page.evaluate(() => window.bikePark.sim.riderCount);
  check(riders === 4, `one player and three bots in the room (${riders})`);

  // Steer with the autopilot hook so the check doesn't depend on the frame rate.
  const before = await page.evaluate(() => window.bikePark.sim.hud().progress);
  await page.evaluate(() => {
    window.bikePark.autopilot = () => ({ throttle: 1, brake: 0, lean: 0, trick: false, respawn: false });
  });
  await page.waitForFunction((p) => window.bikePark.sim.hud().progress > p + 0.01, before, { timeout: 20000 }).catch(() => {});
  const hud = await page.evaluate(() => window.bikePark.sim.hud());
  check(hud.progress > before + 0.01 && hud.speed > 3, `throttle moves the bike (${hud.speed.toFixed(1)} m/s)`);
  check(hud.run_time !== null, "the run timer started");

  // The canvas shows a picture, not a blank or single-colour screen.
  const shot = await page.locator("canvas").screenshot();
  const distinct = new Set();
  for (let i = 0; i < shot.length; i += 97) distinct.add(shot[i]);
  check(distinct.size > 40, `canvas renders a scene (${distinct.size} distinct byte values sampled)`);

  check(errors.length === 0, `no page errors${errors.length ? `: ${errors.join(" | ")}` : ""}`);
} finally {
  await browser.close();
  await server.close();
}

if (failures.length) {
  console.error(`\n${failures.length} check(s) failed`);
  process.exit(1);
}
console.log("\nsmoke test passed");
