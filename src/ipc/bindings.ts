// GENERATED — HAND-MAINTAINED IN PHASE 1.
//
// In Phase 2, this file will be emitted by tauri-specta from the Rust
// `#[tauri::command]`-annotated functions (see ADR-0004). Until then we
// keep a small, carefully-kept copy that mirrors the Rust shape 1:1.
// If you change a command signature in Rust, update this file in the
// same commit and add a note to docs/api-contracts.md.

import { invoke } from "@tauri-apps/api/core";

/** Canonical health report. Mirror of `src-tauri/src/domain/health.rs`. */
export type HealthReport = {
  appName: string;
  appVersion: string;
  schemaVersion: number;
  startedAt: number;
  checkedAt: number;
};

/** Canonical error union. Mirror of `src-tauri/src/error.rs`. */
export type AppError =
  | { kind: "db"; detail: string }
  | { kind: "io"; detail: string }
  | { kind: "invalid_input"; detail: string }
  | { kind: "not_found" }
  | { kind: "internal"; detail: string };

/** Error thrown by typed-IPC wrappers. Carries the decoded `AppError`. */
export class IpcError extends Error {
  readonly appError: AppError;
  constructor(appError: AppError) {
    const detail = "detail" in appError ? appError.detail : "no detail";
    super(`${appError.kind}: ${detail}`);
    this.appError = appError;
    this.name = "IpcError";
  }
}

async function invokeTyped<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (err) {
    if (
      err &&
      typeof err === "object" &&
      "kind" in err &&
      typeof (err as { kind: unknown }).kind === "string"
    ) {
      throw new IpcError(err as AppError);
    }
    throw err;
  }
}

/** Command wrappers. One per `#[tauri::command]` export on the Rust side. */
export const commands = {
  health: () => invokeTyped<HealthReport>("health"),
};
