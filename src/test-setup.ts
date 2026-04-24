import "@testing-library/jest-dom/vitest";

// jsdom does not implement matchMedia — supply a minimal shim so that
// components relying on prefers-reduced-motion or prefers-color-scheme
// can render without crashing in tests.
if (typeof window !== "undefined" && !window.matchMedia) {
  window.matchMedia = (query: string): MediaQueryList => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: () => {},
    removeListener: () => {},
    addEventListener: () => {},
    removeEventListener: () => {},
    dispatchEvent: () => false,
  });
}

// jsdom does not implement HTMLCanvasElement.getContext and prints a
// loud "Not implemented" error the first time anything touches a
// canvas. axe-core's color-contrast rule uses canvas to measure text
// widths; we already allowlist that rule (see
// docs/a11y-exceptions.md) so the error is noise rather than signal.
// A minimal no-op makes CI logs legible.
if (
  typeof window !== "undefined" &&
  typeof HTMLCanvasElement !== "undefined" &&
  !HTMLCanvasElement.prototype.getContext.toString().includes("getContext-stub")
) {
  HTMLCanvasElement.prototype.getContext = function getContextStub() {
    // null return = "context unavailable" — callers fall back safely.
    return null;
  } as unknown as typeof HTMLCanvasElement.prototype.getContext;
}
