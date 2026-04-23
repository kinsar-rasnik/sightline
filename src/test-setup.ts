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
