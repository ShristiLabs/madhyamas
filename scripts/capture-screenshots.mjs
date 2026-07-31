#!/usr/bin/env node
/**
 * Capture screenshots of the Madhyamas web UI for documentation.
 * Requires Madhyamas to be running on http://localhost:3001
 *
 * Usage:
 *   node scripts/capture-screenshots.mjs
 */
import { chromium } from "playwright";
import { mkdirSync } from "fs";
import { resolve, dirname } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const OUT_DIR = resolve(__dirname, "../docs-site/public/screenshots");
const BASE_URL = "http://localhost:3001";

mkdirSync(OUT_DIR, { recursive: true });

// View IDs matching the NavRail order
const VIEWS = [
  { id: "traffic", label: "Traffic", file: "traffic-view.png" },
  { id: "breakpoints", label: "Breakpoints", file: "breakpoints-view.png" },
  { id: "throttle", label: "Throttle", file: "throttle-view.png" },
  { id: "mocks", label: "Mocks", file: "mocks-view.png" },
  { id: "rewrites", label: "Rewrites", file: "rewrites-view.png" },
  { id: "replay", label: "Replay", file: "replay-view.png" },
  { id: "grpc", label: "gRPC", file: "grpc-view.png" },
  { id: "scripts", label: "Scripts", file: "scripts-view.png" },
  { id: "plugins", label: "Plugins", file: "plugins-view.png" },
  { id: "sessions", label: "Sessions", file: "sessions-view.png" },
];

async function main() {
  const browser = await chromium.launch({ headless: true });
  const context = await browser.newContext({
    viewport: { width: 1440, height: 900 },
    deviceScaleFactor: 2,
  });
  const page = await context.newPage();

  console.log("Navigating to", BASE_URL);
  await page.goto(BASE_URL, { waitUntil: "networkidle", timeout: 15000 });
  await page.waitForTimeout(1500);

  // Capture the header/help dropdown
  console.log("Capturing: header");
  await page.screenshot({
    path: resolve(OUT_DIR, "header.png"),
    clip: { x: 0, y: 0, width: 1440, height: 44 },
  });

  // Capture each view
  for (const view of VIEWS) {
    console.log(`Capturing: ${view.label} (${view.id})`);
    // Click the nav rail button for this view
    const navButton = page.locator(`[aria-label="${view.label}"]`);
    await navButton.click();
    await page.waitForTimeout(1200);
    await page.screenshot({
      path: resolve(OUT_DIR, view.file),
      fullPage: false,
    });
  }

  // Capture traffic detail view (click first traffic entry)
  console.log("Capturing: traffic detail");
  const trafficBtn = page.locator('[aria-label="Traffic"]');
  await trafficBtn.click();
  await page.waitForTimeout(1000);
  // Click the first row in the traffic list (rows have role="button")
  const firstRow = page.locator('main [role="button"]').first();
  if (await firstRow.count() > 0) {
    await firstRow.click();
    await page.waitForTimeout(1500);
    await page.screenshot({
      path: resolve(OUT_DIR, "traffic-detail.png"),
      fullPage: false,
    });
  }

  // Capture the Setup/Certificate dialog
  console.log("Capturing: setup dialog");
  const setupBtn = page.locator('button[title="Certificate setup"]');
  if (await setupBtn.count() > 0) {
    await setupBtn.click();
    await page.waitForTimeout(1500);
    await page.screenshot({
      path: resolve(OUT_DIR, "setup-dialog.png"),
      fullPage: false,
    });
    // Close dialog
    await page.keyboard.press("Escape");
    await page.waitForTimeout(500);
  }

  // Capture the Config dialog
  console.log("Capturing: config dialog");
  const configBtn = page.locator('button[title="Configuration"]');
  if (await configBtn.count() > 0) {
    await configBtn.click();
    await page.waitForTimeout(1500);
    await page.screenshot({
      path: resolve(OUT_DIR, "config-dialog.png"),
      fullPage: false,
    });
    await page.keyboard.press("Escape");
    await page.waitForTimeout(500);
  }

  // Capture the Help dropdown
  console.log("Capturing: help dropdown");
  const helpBtn = page.locator('button[title="Help"]');
  if (await helpBtn.count() > 0) {
    await helpBtn.click();
    await page.waitForTimeout(800);
    await page.screenshot({
      path: resolve(OUT_DIR, "help-dropdown.png"),
      fullPage: false,
    });
    await page.keyboard.press("Escape");
    await page.waitForTimeout(500);
  }

  // Capture full app overview (traffic view)
  console.log("Capturing: app overview");
  await trafficBtn.click();
  await page.waitForTimeout(1000);
  await page.screenshot({
    path: resolve(OUT_DIR, "app-overview.png"),
    fullPage: false,
  });

  await browser.close();
  console.log(`\nAll screenshots saved to: ${OUT_DIR}`);
}

main().catch((err) => {
  console.error("Screenshot capture failed:", err);
  process.exit(1);
});
