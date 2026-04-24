// Wrapper around Tauri's `convertFileSrc` so components don't need to
// care whether the app is running inside the webview or a plain
// browser (`pnpm dev`, Vitest). In a pure-browser context we fall
// back to a data URI for the rare cases that care; for the majority
// path the caller uses the empty string and the UI renders a
// placeholder.
//
// Note: security-critical invariant — the backend service
// `media_assets.rs::guarded_path` is what *actually* prevents the
// renderer from asking for a file outside the library root. This
// helper is just URL plumbing.

import { convertFileSrc as tauriConvertFileSrc } from "@tauri-apps/api/core";

export function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/**
 * Turn an absolute local file path into a webview-reachable URL via
 * Tauri's asset protocol. Returns the empty string when the path is
 * null/empty or when running outside Tauri — callers should handle
 * the empty-string branch with a visual placeholder rather than a
 * broken `<img>`.
 */
export function assetUrl(path: string | null | undefined): string {
  if (!path) return "";
  if (!isTauriRuntime()) return "";
  return tauriConvertFileSrc(path);
}
