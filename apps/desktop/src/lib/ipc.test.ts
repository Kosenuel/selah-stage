import { assert, describe, expect, it, vi } from "vitest";

import { call } from "./ipc";

describe("call", () => {
  it("passes a successful result through", async () => {
    const result = await call(Promise.resolve({ status: "ok" as const, data: 42 }));

    expect(result).toEqual({ status: "ok", data: 42 });
  });

  it("keeps an error that follows the core's contract", async () => {
    const error = { kind: "internal", message: "The show could not be saved. Check the disk." };

    const result = await call(Promise.resolve({ status: "error" as const, error }));

    expect(result).toEqual({ status: "error", error });
  });

  it("replaces a failure from outside the contract with a plain message", async () => {
    const logged = vi.spyOn(console, "error").mockImplementation(() => undefined);

    const result = await call(
      Promise.resolve({ status: "error" as const, error: "command app_info not found" }),
    );

    assert(result.status === "error");
    expect(result.error.kind).toBe("internal");
    expect(result.error.message).toContain("Something went wrong");
    // Developer-facing text stays out of the UI but is kept for diagnosis.
    expect(result.error.message).not.toContain("app_info");
    expect(logged).toHaveBeenCalledOnce();
  });
});
