/**
 * The one way the UI calls the core.
 *
 * `bindings.ts` is generated from the Rust command signatures. A failed call
 * there carries an `unknown` error, because it may be our `AppError` or a
 * failure from Tauri itself (an unknown command, a bad argument, no core at
 * all). `call` narrows that in one place, so features always receive a
 * well-formed `AppError` and never have to catch.
 */
import type { AppError } from "./bindings";

type RawResult<T> = { status: "ok"; data: T } | { status: "error"; error: unknown };

export type CommandResult<T> = { status: "ok"; data: T } | { status: "error"; error: AppError };

/** Awaits a generated command. Never throws. */
export async function call<T>(command: Promise<RawResult<T>>): Promise<CommandResult<T>> {
  const result = await command;
  if (result.status === "ok") {
    return result;
  }
  return { status: "error", error: toAppError(result.error) };
}

const FALLBACK_MESSAGE = "Something went wrong inside Selah Stage. Try again, or restart the app.";

function toAppError(error: unknown): AppError {
  if (isAppError(error)) {
    return error;
  }
  // Not one of ours, so its text is written for developers, not operators.
  // Keep it out of the UI but leave it in the console for diagnosis.
  console.error("Command failed outside the core's error contract:", error);
  return { kind: "internal", message: FALLBACK_MESSAGE };
}

function isAppError(error: unknown): error is AppError {
  if (typeof error !== "object" || error === null) {
    return false;
  }
  const candidate = error as Record<string, unknown>;
  return typeof candidate.kind === "string" && typeof candidate.message === "string";
}
