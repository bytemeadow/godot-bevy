import { execFile } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs, promisify } from "node:util";

const runFile = promisify(execFile);
const adapter = path.dirname(fileURLToPath(import.meta.url));
const scene = "res://browser/orbit.tscn";
const readyLine = `CAPTURE_READY scenario=browser-orbit scene=${scene} frame=0`;
const usage = "node itest/capture/browser/browser.mjs --url http://127.0.0.1:8060 --variant web-nothreads --output <fresh-evidence-leaf> [--timeout 60] [--threshold 0.001] [--python python3]";

async function main() {
  let options;
  try {
    const { values } = parseArgs({
      options: {
        url: { type: "string" },
        variant: { type: "string" },
        output: { type: "string" },
        timeout: { type: "string", default: "60" },
        threshold: { type: "string", default: "0.001" },
        python: { type: "string", default: "python3" },
        help: { type: "boolean", default: false },
      },
    });
    if (values.help) {
      console.log(usage);
      return 0;
    }
    const url = new URL(values.url);
    const timeout = Number(values.timeout) * 1000;
    const threshold = Number(values.threshold);
    if (!["http:", "https:"].includes(url.protocol) || !values.output ||
        !["web", "web-nothreads"].includes(values.variant) ||
        !Number.isFinite(timeout) || timeout <= 0 || timeout > 3600000 ||
        !Number.isFinite(threshold) || threshold <= 0 || threshold >= 1) {
      throw new Error(usage);
    }
    url.searchParams.set("capture-browser", "1");
    options = { ...values, url: url.href, timeout, threshold };
  } catch (error) {
    console.error(JSON.stringify({ browser: "untested", error: error.message, usage }));
    return 2;
  }

  const leaf = path.resolve(options.output);
  const output = path.join(leaf, "browser");
  try {
    await mkdir(path.dirname(leaf), { recursive: true });
    await mkdir(leaf);
    await mkdir(output);
  } catch (error) {
    console.error(JSON.stringify({ browser: "untested", error: error.message }));
    return 2;
  }

  const report = {
    browser: "untested",
    gate: "godot-bevy#268",
    url: options.url,
    variant: options.variant,
    viewport: [960, 640],
    frames_apart: 60,
    threshold: options.threshold,
    checks: { readiness: "untested", headers: "untested", motion: "untested" },
    required_artifacts: ["browser/console.log", "browser/responses.json", "browser/frame-000000.png", "browser/frame-000060.png", "browser/motion.json", "browser/probe.json"],
    errors: [],
    exit_code: 1,
  };
  const logs = [];
  const responses = [];
  const readyLines = [];
  let browser;
  let launch;
  let work;
  let deadline;
  let interrupt;
  const controller = new AbortController();
  try {
    let playwright;
    try {
      playwright = await import("playwright");
    } catch (error) {
      if (error.code !== "ERR_MODULE_NOT_FOUND") throw error;
      try {
        playwright = await import("playwright-core");
      } catch {
        report.exit_code = 2;
        throw new Error("Playwright is unavailable. Use the manual path in itest/capture/browser/README.md; browser remains untested.");
      }
    }

    const aborted = new Promise((_, reject) => {
      interrupt = () => reject(new Error("Browser probe interrupted"));
      process.on("SIGINT", interrupt);
      process.on("SIGTERM", interrupt);
      deadline = setTimeout(() => reject(new Error("Browser probe timed out")), options.timeout);
    });

    const probe = async () => {
      launch = playwright.chromium.launch({ headless: true, timeout: options.timeout });
      browser = await launch;
      report.chromium = browser.version();
      const context = await browser.newContext({
        viewport: { width: 960, height: 640 },
        deviceScaleFactor: 1,
        serviceWorkers: "block",
      });
      const page = await context.newPage();
      page.setDefaultTimeout(options.timeout);
      page.on("console", (message) => {
        const line = message.text();
        logs.push(`${message.type()}: ${line}`);
        if (line.split(/\s/, 1)[0] === "CAPTURE_READY") readyLines.push(line);
        if (message.type() === "error") report.errors.push(line);
      });
      page.on("pageerror", (error) => report.errors.push(error.message));
      page.on("crash", () => report.errors.push("Chromium page crashed"));
      page.on("response", (response) => {
        if (new URL(response.url()).origin !== new URL(options.url).origin) return;
        const headers = response.headers();
        responses.push({
          url: response.url(), status: response.status(),
          coop: headers["cross-origin-opener-policy"] ?? null,
          coep: headers["cross-origin-embedder-policy"] ?? null,
          content_type: headers["content-type"] ?? null,
        });
      });
      const response = await page.goto(options.url, { waitUntil: "domcontentloaded" });
      if (!response?.ok()) throw new Error("Export page failed to load");
      const environment = await page.evaluate(() => ({
        isolated: window.crossOriginIsolated,
        variant: document.querySelector('meta[name="browser-variant"]')?.content,
      }));
      const threaded = options.variant === "web";
      if (environment.variant !== options.variant || environment.isolated !== threaded) {
        throw new Error(`Export variant or isolation mismatch: ${JSON.stringify(environment)}`);
      }
      report.environment = environment;

      await page.waitForFunction(() => window.browserCapture?.ready && window.browserCapture.paused && window.browserCapture.frame === 0);
      if (readyLines.length !== 1 || readyLines[0] !== readyLine) {
        throw new Error("Missing, duplicate or incorrect CAPTURE_READY console line");
      }
      report.ready = readyLines[0];
      report.godot = await page.evaluate(() => window.browserCapture.godot);
      report.threaded = await page.evaluate(() => window.browserCapture.threaded);
      if (report.threaded !== threaded) throw new Error("Godot thread support does not match the requested variant");
      report.checks.readiness = "pass";

      const screenshot = async (frame) => {
        const state = () => page.evaluate(() => ({
          paused: window.browserCapture.paused,
          frame: window.browserCapture.frame,
          scene: window.browserCapture.scene,
        }));
        const before = await state();
        if (!before.paused || before.frame !== frame || before.scene !== scene) {
          throw new Error(`Screenshot is not held at frame ${frame}`);
        }
        await page.locator("#canvas").screenshot({ path: path.join(output, `frame-${String(frame).padStart(6, "0")}.png`) });
        if (JSON.stringify(before) !== JSON.stringify(await state())) {
          throw new Error("Scene advanced during screenshot");
        }
      };
      await screenshot(0);
      await page.evaluate(() => window.browserCapture.advance());
      await page.waitForFunction(() => window.browserCapture?.paused && window.browserCapture.frame === 60);
      await screenshot(60);

      if (readyLines.length !== 1) throw new Error("Duplicate CAPTURE_READY console line");
      if (responses.some((item) => item.coop !== (threaded ? "same-origin" : null) ||
          item.coep !== (threaded ? "require-corp" : null) || item.status >= 400)) {
        throw new Error("Export resources have incorrect isolation headers or HTTP errors");
      }
      report.checks.headers = "pass";
      let measurement;
      try {
        measurement = await runFile(options.python, [
          path.join(adapter, "verdict.py"),
          path.join(output, "frame-000000.png"),
          path.join(output, "frame-000060.png"),
          "--threshold", String(options.threshold),
        ], { timeout: options.timeout, signal: controller.signal });
      } catch (error) {
        if (error.code !== 1 || !error.stdout) throw error;
        measurement = error;
      }
      const motion = JSON.parse(measurement.stdout);
      await writeFile(path.join(output, "motion.json"), JSON.stringify(motion, null, 2) + "\n");
      if (motion.viewport?.[0] !== 960 || motion.viewport?.[1] !== 640 || !motion.motion?.detected) {
        report.checks.motion = "fail";
        throw new Error(motion.error ?? "Visible motion did not exceed the threshold at the required viewport");
      }
      report.checks.motion = "pass";
      if (report.errors.length) throw new Error("Browser or engine errors were recorded");
    };
    work = probe();
    await Promise.race([work, aborted]);
    report.exit_code = 0;
  } catch (error) {
    report.errors.push(error.message);
    if (report.exit_code !== 2) report.exit_code = 1;
  } finally {
    clearTimeout(deadline);
    controller.abort();
    try {
      browser ??= await launch?.catch(() => null);
      await browser?.close();
    } catch (error) {
      report.errors.push(`Browser cleanup: ${error.message}`);
      report.exit_code = 1;
    }
    await work?.catch(() => {});
    if (interrupt) {
      process.removeListener("SIGINT", interrupt);
      process.removeListener("SIGTERM", interrupt);
    }
    await writeFile(path.join(output, "console.log"), logs.join("\n") + "\n");
    await writeFile(path.join(output, "responses.json"), JSON.stringify(responses, null, 2) + "\n");
    await writeFile(path.join(output, "probe.json"), JSON.stringify(report, null, 2) + "\n");
  }
  console.log(JSON.stringify(report));
  return report.exit_code;
}

main().then((code) => { process.exitCode = code; }).catch((error) => {
  console.error(JSON.stringify({ browser: "untested", error: error.message }));
  process.exitCode = 2;
});
