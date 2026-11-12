import { fileURLToPath } from "node:url";

import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

const fromRoot = (path: string) => fileURLToPath(new URL(path, import.meta.url));

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: { "@": fromRoot("./src") },
  },
  // Keep Rust compiler output visible when Tauri runs the dev server.
  clearScreen: false,
  server: {
    // Tauri's `devUrl` points at this exact port, so never fall back to another.
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  build: {
    rolldownOptions: {
      // One entry per kind of window. Output windows load nothing from the
      // control UI, so they stay small and can be reloaded on their own.
      input: {
        control: fromRoot("./index.html"),
        output: fromRoot("./output.html"),
      },
    },
  },
  test: {
    environment: "jsdom",
    setupFiles: ["./src/test/setup.ts"],
    restoreMocks: true,
  },
});
