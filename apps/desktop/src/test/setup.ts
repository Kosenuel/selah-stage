import "@testing-library/jest-dom/vitest";

import { clearMocks } from "@tauri-apps/api/mocks";
import { cleanup } from "@testing-library/react";
import { afterEach } from "vitest";

afterEach(() => {
  cleanup();
  // Tooling tests run in Node, where there is no window to reset.
  if (typeof window !== "undefined") {
    clearMocks();
  }
});
