/**
 * Token-Snapshot Test — Wave-4 Mission 1 (ADR-0040 TV12 Test-Strategy item 1).
 *
 * Asserts that the binding token values from ADR-0040 § Implementation-
 * Notes are present in `src/styles/globals.css` with exact values.
 *
 * Implementation note. The mission brief suggested
 * `getComputedStyle(document.documentElement)` for this assertion. In a
 * Vitest + JSDOM environment, Tailwind-v4's `@theme {}` block is not
 * always fully resolved by JSDOM's CSSOM engine — `@theme` is a
 * Tailwind-specific construct that Vite's @tailwindcss/vite plugin
 * transforms at build-time to a `:root { --token: value }` block; in the
 * test runtime, depending on plugin chain ordering, `getComputedStyle`
 * may return empty strings for the custom properties.
 *
 * To make this test deterministic and to verify against the canonical
 * spec source, we read `globals.css` as raw text and assert via regex.
 * This is stronger than computed-style because it locks the source-of-
 * truth, not the runtime resolution. If/when Wave 5 adds Playwright
 * visual-regression baselines, computed-style assertions land there
 * against the actual rendered DOM.
 */

import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, test } from "vitest";

// Resolve relative to repo root (vitest runs from there). `import.meta.url`
// is not a file:// URL under vitest's jsdom environment, so fileURLToPath
// is unavailable.
const GLOBALS_CSS_PATH = resolve(process.cwd(), "src/styles/globals.css");

const GLOBALS_CSS = readFileSync(GLOBALS_CSS_PATH, "utf8");

/**
 * Extract a CSS-custom-property value from the `@theme {}` block.
 * Tolerates whitespace and trailing comments.
 */
function token(name: string): string {
  const pattern = new RegExp(
    `${name.replace(/[-/\\^$*+?.()|[\]{}]/g, "\\$&")}:\\s*([^;]+?);`,
    "m"
  );
  const match = GLOBALS_CSS.match(pattern);
  const value = match?.[1];
  if (!value) {
    throw new Error(
      `Token ${name} not found in globals.css — Wave-4 Mission 1 spec violation`
    );
  }
  return value.trim();
}

describe("ADR-0040 binding tokens — source-of-truth snapshot", () => {
  describe("TV1 Surface Ladder + Text Ramp + Accent + Outlines", () => {
    test("--color-bg = #0a0a0d (anchor §2.1 single-value bg)", () => {
      expect(token("--color-bg")).toBe("#0a0a0d");
    });

    test("--color-surface-1 / 2 / 3 = ladder L* 7 / 10 / 13", () => {
      expect(token("--color-surface-1")).toBe("#14141a");
      expect(token("--color-surface-2")).toBe("#1c1c24");
      expect(token("--color-surface-3")).toBe("#242430");
    });

    test("--color-fg = #f5f5f7 (anchor body text)", () => {
      expect(token("--color-fg")).toBe("#f5f5f7");
    });

    test("--color-muted = rgba(245, 245, 247, 0.65) (H-11 two-step ramp)", () => {
      expect(token("--color-muted")).toBe("rgba(245, 245, 247, 0.65)");
    });

    test("--color-accent = #d4a14a (Locked Input)", () => {
      expect(token("--color-accent")).toBe("#d4a14a");
    });

    test("--color-accent-fg = #0a0a0d (dark text on light accent fill)", () => {
      expect(token("--color-accent-fg")).toBe("#0a0a0d");
    });

    test("--color-outline-focus = rgba(255, 255, 255, 0.85) (H-20 mid of 80–90 %)", () => {
      expect(token("--color-outline-focus")).toBe(
        "rgba(255, 255, 255, 0.85)"
      );
    });
  });

  describe("TV4 Card Geometry (TV-A1 / TV-A2 / TV-A3 binding)", () => {
    test("--card-lane-height = 72px (TV-A1)", () => {
      expect(token("--card-lane-height")).toBe("72px");
    });

    test("--card-width-readable-px = 160px (TV-A2 Full ↔ Compact)", () => {
      expect(token("--card-width-readable-px")).toBe("160px");
    });

    test("--card-width-image-only-px = 96px (TV-A2 Compact ↔ Sliver)", () => {
      expect(token("--card-width-image-only-px")).toBe("96px");
    });

    test("--card-hover-zoom-factor = 1.10 (TV-A3)", () => {
      expect(token("--card-hover-zoom-factor")).toBe("1.10");
    });
  });

  describe("TV6 Motion Constants", () => {
    test("--motion-hover-delay = 800ms (DV3 H-18 midpoint)", () => {
      expect(token("--motion-hover-delay")).toBe("800ms");
    });

    test("--motion-zoom-duration = 200ms (mid H-21 180–240 ms)", () => {
      expect(token("--motion-zoom-duration")).toBe("200ms");
    });

    test("--motion-frame-cadence = 400ms (DV3 v2.0.x precedent)", () => {
      expect(token("--motion-frame-cadence")).toBe("400ms");
    });

    test("--motion-easing-out = cubic-bezier(0.16, 1, 0.3, 1)", () => {
      expect(token("--motion-easing-out")).toBe(
        "cubic-bezier(0.16, 1, 0.3, 1)"
      );
    });
  });

  describe("TV5 Card Element Numerics", () => {
    test("--progress-stripe-height = 3px (Q-10 binding)", () => {
      expect(token("--progress-stripe-height")).toBe("3px");
    });

    test("--focus-outline-width = 2px (H-20 max)", () => {
      expect(token("--focus-outline-width")).toBe("2px");
    });
  });

  describe("TV9 Sidebar (TV-A4 binding)", () => {
    test("--sidebar-collapsed-width = 56px (TV-A4)", () => {
      expect(token("--sidebar-collapsed-width")).toBe("56px");
    });
  });

  describe("TV10 Hero-Slot (TV-A5 binding)", () => {
    test("--hero-slot-height = 280px (TV-A5)", () => {
      expect(token("--hero-slot-height")).toBe("280px");
    });
  });

  describe("TV7 Z-Index Stack", () => {
    test("--z-card-hover = 20 (above neighbours)", () => {
      expect(token("--z-card-hover")).toBe("20");
    });

    test("--z-modal = 50 (detail overlay + settings modal)", () => {
      expect(token("--z-modal")).toBe("50");
    });
  });

  describe("TV12 anti-pattern enforcement", () => {
    test("--color-subtle removed (H-11 two-step ramp only)", () => {
      expect(GLOBALS_CSS).not.toMatch(/--color-subtle\s*:/);
    });

    test("sightline-shimmer keyframe removed (§5 anti-pattern)", () => {
      // The literal comment "sightline-shimmer removed" is permitted;
      // the keyframe definition itself (`@keyframes sightline-shimmer`)
      // must be absent.
      expect(GLOBALS_CSS).not.toMatch(/@keyframes\s+sightline-shimmer/);
    });

    test("prefers-color-scheme: light block removed (dark-only)", () => {
      expect(GLOBALS_CSS).not.toMatch(
        /@media\s*\(\s*prefers-color-scheme:\s*light\s*\)/
      );
    });

    test("color-scheme is dark-only (no `dark light` fallback)", () => {
      expect(GLOBALS_CSS).toMatch(/color-scheme:\s*dark\s*;/);
      expect(GLOBALS_CSS).not.toMatch(/color-scheme:\s*dark\s+light/);
    });

    test("legacy --motion-fast / --motion-base / --motion-slow removed", () => {
      // Allow as substring inside a per-purpose token name, but the bare
      // token definitions must be gone.
      expect(GLOBALS_CSS).not.toMatch(/--motion-fast\s*:/);
      expect(GLOBALS_CSS).not.toMatch(/--motion-base\s*:/);
      expect(GLOBALS_CSS).not.toMatch(/--motion-slow\s*:/);
    });

    test("legacy --color-surface (without -1/2/3 suffix) removed", () => {
      // `--color-surface-1/2/3/-skeleton` are permitted; bare
      // `--color-surface:` is the v2.0.x token that must be gone.
      expect(GLOBALS_CSS).not.toMatch(/--color-surface\s*:/);
      expect(GLOBALS_CSS).not.toMatch(/--color-surface-elevated\s*:/);
      expect(GLOBALS_CSS).not.toMatch(/--color-surface-interactive\s*:/);
    });

    test("legacy --color-focus-ring removed (renamed to --color-outline-focus)", () => {
      expect(GLOBALS_CSS).not.toMatch(/--color-focus-ring\s*:/);
    });
  });
});
