#!/usr/bin/env node
/**
 * Capture screenshots of Madhyamas enterprise web UI for documentation.
 * Requires the enterprise stack to be running (./startup-local.sh --tier enterprise).
 *
 * Usage:
 *   node scripts/capture-enterprise-screenshots.mjs
 */
import { chromium } from "playwright";
import { mkdirSync } from "fs";
import { resolve, dirname } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const OUT_DIR = resolve(__dirname, "../docs-site/public/screenshots");
const BASE_URL = "http://localhost:14000";
const USERNAME = "admin";
const PASSWORD = "testpass123";

mkdirSync(OUT_DIR, { recursive: true });

async function main() {
  const browser = await chromium.launch({ headless: true });
  const context = await browser.newContext({
    viewport: { width: 1440, height: 900 },
    deviceScaleFactor: 2,
  });
  const page = await context.newPage();

  // 1. Login page (capture before logging in)
  console.log("Capturing: enterprise-login.png");
  await page.goto(BASE_URL, { waitUntil: "networkidle", timeout: 15000 });
  await page.waitForTimeout(1500);
  await page.screenshot({
    path: resolve(OUT_DIR, "enterprise-login.png"),
    fullPage: false,
  });

  // Log in
  console.log("Logging in as admin...");
  const usernameInput = page.locator("#username");
  await usernameInput.fill(USERNAME);
  await page.locator("#password").fill(PASSWORD);
  await page.locator('button[type="submit"]').click();
  await page.waitForTimeout(2500);

  // 2. User menu dropdown
  console.log("Capturing: enterprise-user-menu.png");
  // The user menu trigger shows the username text
  const userMenuBtn = page.locator('button:has-text("admin")').first();
  if (await userMenuBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
    await userMenuBtn.click();
    await page.waitForTimeout(800);
    await page.screenshot({
      path: resolve(OUT_DIR, "enterprise-user-menu.png"),
      fullPage: false,
    });
    await page.keyboard.press("Escape");
    await page.waitForTimeout(300);
  } else {
    console.log("  (user menu not found, capturing header)");
    await page.screenshot({ path: resolve(OUT_DIR, "enterprise-user-menu.png") });
  }

  // Helper to click a NavRail button by aria-label and capture
  async function capturePanel(ariaLabel, filename) {
    console.log(`Capturing: ${filename}`);
    const navBtn = page.locator(`button[aria-label="${ariaLabel}"]`).first();
    if (await navBtn.isVisible({ timeout: 5000 }).catch(() => false)) {
      await navBtn.click();
      await page.waitForTimeout(1500);
      await page.screenshot({
        path: resolve(OUT_DIR, filename),
        fullPage: false,
      });
    } else {
      console.log(`  (NavRail button "${ariaLabel}" not found)`);
    }
  }

  // 3-8. Admin panels via NavRail aria-label
  await capturePanel("Users", "enterprise-users-panel.png");
  await capturePanel("Audit Log", "enterprise-audit-panel.png");
  await capturePanel("Metrics", "enterprise-metrics-panel.png");
  await capturePanel("License", "enterprise-license-panel.png");
  await capturePanel("API Keys", "enterprise-apikeys-panel.png");
  await capturePanel("Instances", "enterprise-instances-panel.png");

  await browser.close();
  console.log("Done! Screenshots saved to", OUT_DIR);
}

main().catch((err) => {
  console.error("Screenshot capture failed:", err);
  process.exit(1);
});
