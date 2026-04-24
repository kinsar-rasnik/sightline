// Hand-written thin wrapper over the generated tauri-specta bindings.
// Source of truth: `src-tauri/src/commands/**` + `src-tauri/src/ipc.rs`.
// See docs/adr/0007-ipc-typegen.md.
//
// The generator emits `Result<T, AppError>`-style commands
// (`{ status: "ok" } | { status: "error" }`). This file unwraps them into
// throw-style Promises so hooks and components can use idiomatic
// `await command(...)` calls, while keeping the discriminated `AppError`
// union intact via the `IpcError` class.

import {
  commands as generatedCommands,
  type AppError,
  type HealthReport,
} from "@/ipc/bindings";

export type { AppError, HealthReport };

/**
 * Wraps a typed `AppError` so React hooks can `throw` on failure while
 * still exposing the full discriminated union to callers.
 */
export class IpcError extends Error {
  readonly appError: AppError;
  constructor(appError: AppError) {
    const detail = "detail" in appError ? appError.detail : appError.kind;
    super(`${appError.kind}: ${detail}`);
    this.appError = appError;
    this.name = "IpcError";
  }
}

type IpcResult<T> =
  | { status: "ok"; data: T }
  | { status: "error"; error: AppError };

function unwrap<T>(result: IpcResult<T>): T {
  if (result.status === "ok") return result.data;
  throw new IpcError(result.error);
}

/**
 * Throw-style wrappers around the generated Result-style commands. One
 * entry per Rust `#[tauri::command]`. Type inference flows from the
 * generator so new fields show up here automatically.
 */
export const commands = {
  health: async (): Promise<HealthReport> => unwrap(await generatedCommands.health()),
};

/**
 * Raw access to the generator's Result shape, for code paths that prefer
 * branching on `status` rather than catching.
 */
export const rawCommands = generatedCommands;
