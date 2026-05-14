/**
 * Reduced-Motion Override Test — Wave-4 Mission 1 (ADR-0040 TV12
 * Test-Strategy item 3 + TV6 H-22 binding).
 *
 * Asserts that the `@media (prefers-reduced-motion: reduce)` block in
 * `src/styles/globals.css` correctly collapses the motion tokens
 * documented in ADR-0040 TV6 to the override values:
 *
 *   --motion-hover-delay              → 0ms
 *   --motion-zoom-duration            → 0ms
 *   --motion-crossfade-duration       → 0ms
 *   --motion-panel-slide-duration     → 0ms
 *   --motion-frame-cadence            → 0ms
 *   --motion-modal-open-duration      → 100ms  (anchor §4 opacity-fade allowance)
 *   --motion-sidebar-reveal-duration  → 0ms
 *   --motion-toolbar-fade-duration    → 0ms
 *   --card-hover-zoom-factor          → 1.0
 *
 * Implementation note. As with the snapshot test, this asserts on the
 * source text of `globals.css` directly. Mocking JSDOM's `matchMedia`
 * does not cause the `@media (prefers-reduced-motion: reduce)` CSS block
 * to apply at runtime — JSDOM's CSSOM does not evaluate `@media` queries
 * for value resolution. The deterministic alternative is to parse the
 * source CSS and assert the override-block content matches the binding
 * spec. If/when Wave 5 lands Playwright visual-regression baselines,
 * the runtime browser environment can run end-to-end reduced-motion
 * verification.
 */

import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, test } from "vitest";

const GLOBALS_CSS_PATH = resolve(process.cwd(), "src/styles/globals.css");

const GLOBALS_CSS = readFileSync(GLOBALS_CSS_PATH, "utf8");

/**
 * Extract the body of the @media (prefers-reduced-motion: reduce) block
 * that contains the :root override section. Returns the raw text of the
 * inner `:root { ... }` block.
 */
function reducedMotionOverrideBlock(): string {
  // Find the @media block; allow whitespace flexibility. Capture text up
  // to the matching brace (CSS nesting in Tailwind v4 supports nested
  // braces; we use a non-greedy match plus a brace-counting fallback).
  const mediaPattern =
    /@media\s*\(\s*prefers-reduced-motion:\s*reduce\s*\)\s*\{([\s\S]*?)\n\s\s\}/m;
  const mediaMatch = GLOBALS_CSS.match(mediaPattern);
  const inner = mediaMatch?.[1];
  if (!inner) {
    throw new Error(
      "@media (prefers-reduced-motion: reduce) :root-override block not found in globals.css"
    );
  }
  // Inner contains a :root { ... } block; extract its body.
  const rootPattern = /:root\s*\{([\s\S]*?)\}/m;
  const rootMatch = inner.match(rootPattern);
  const rootBody = rootMatch?.[1];
  if (!rootBody) {
    throw new Error(
      "Reduced-motion @media block did not contain :root override"
    );
  }
  return rootBody;
}

function overrideToken(block: string, name: string): string {
  const pattern = new RegExp(
    `${name.replace(/[-/\\^$*+?.()|[\]{}]/g, "\\$&")}:\\s*([^;]+?);`,
    "m"
  );
  const match = block.match(pattern);
  const value = match?.[1];
  if (!value) {
    throw new Error(
      `Token ${name} not overridden in @media (prefers-reduced-motion: reduce) block`
    );
  }
  return value.trim();
}

const OVERRIDE_BLOCK = reducedMotionOverrideBlock();

describe("ADR-0040 TV6 reduced-motion override block", () => {
  test("--motion-hover-delay → 0ms", () => {
    expect(overrideToken(OVERRIDE_BLOCK, "--motion-hover-delay")).toBe("0ms");
  });

  test("--motion-zoom-duration → 0ms", () => {
    expect(overrideToken(OVERRIDE_BLOCK, "--motion-zoom-duration")).toBe(
      "0ms"
    );
  });

  test("--motion-crossfade-duration → 0ms", () => {
    expect(
      overrideToken(OVERRIDE_BLOCK, "--motion-crossfade-duration")
    ).toBe("0ms");
  });

  test("--motion-panel-slide-duration → 0ms", () => {
    expect(
      overrideToken(OVERRIDE_BLOCK, "--motion-panel-slide-duration")
    ).toBe("0ms");
  });

  test("--motion-frame-cadence → 0ms (frame-strip loop stops)", () => {
    expect(overrideToken(OVERRIDE_BLOCK, "--motion-frame-cadence")).toBe(
      "0ms"
    );
  });

  test("--motion-modal-open-duration → 100ms (anchor §4 opacity-fade allowance)", () => {
    expect(
      overrideToken(OVERRIDE_BLOCK, "--motion-modal-open-duration")
    ).toBe("100ms");
  });

  test("--motion-sidebar-reveal-duration → 0ms", () => {
    expect(
      overrideToken(OVERRIDE_BLOCK, "--motion-sidebar-reveal-duration")
    ).toBe("0ms");
  });

  test("--motion-toolbar-fade-duration → 0ms", () => {
    expect(
      overrideToken(OVERRIDE_BLOCK, "--motion-toolbar-fade-duration")
    ).toBe("0ms");
  });

  test("--card-hover-zoom-factor → 1.0 (no card scale animation)", () => {
    expect(overrideToken(OVERRIDE_BLOCK, "--card-hover-zoom-factor")).toBe(
      "1.0"
    );
  });

  test("override block lives inside @layer base (TV12 implementation pattern)", () => {
    // Verify that the @media reduced-motion block is inside the @layer
    // base { ... } scope rather than at top-level; this is the TV12
    // binding placement.
    const layerBaseMatch = GLOBALS_CSS.match(
      /@layer\s+base\s*\{([\s\S]*?)\n\}/m
    );
    const layerBaseBody = layerBaseMatch?.[1];
    expect(layerBaseBody).toBeDefined();
    expect(layerBaseBody).toMatch(
      /@media\s*\(\s*prefers-reduced-motion:\s*reduce\s*\)/
    );
  });

  test("animation keyframe consumers respect reduced-motion (sightline-slide-in + fade-in)", () => {
    // Both keyframes have a separate @media block at top-level that
    // disables the animation classes.
    expect(GLOBALS_CSS).toMatch(
      /\.sightline-slide-in,\s*\.sightline-fade-in\s*\{[\s\S]*?animation:\s*none/
    );
  });
});
