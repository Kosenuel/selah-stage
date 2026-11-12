import path from "node:path";

import js from "@eslint/js";
import { defineConfig, globalIgnores } from "eslint/config";
import reactHooks from "eslint-plugin-react-hooks";
import globals from "globals";
import tseslint from "typescript-eslint";

import { boundaries } from "./tooling/eslint-boundaries.js";

const selah = { rules: { boundaries } };

export default defineConfig([
  // `bindings.ts` is generated from Rust; see src-tauri/src/ipc.rs.
  globalIgnores(["dist", "src-tauri", "src/lib/bindings.ts"]),
  { linterOptions: { reportUnusedDisableDirectives: "error" } },
  {
    files: ["**/*.{ts,tsx}"],
    extends: [
      js.configs.recommended,
      tseslint.configs.strictTypeChecked,
      tseslint.configs.stylisticTypeChecked,
      reactHooks.configs.flat.recommended,
    ],
    plugins: { selah },
    languageOptions: {
      globals: globals.browser,
      parserOptions: { projectService: true, tsconfigRootDir: import.meta.dirname },
    },
    rules: {
      "selah/boundaries": ["error", { srcRoot: path.join(import.meta.dirname, "src") }],
      // Numbers and booleans are fine in template strings.
      "@typescript-eslint/restrict-template-expressions": [
        "error",
        { allowNumber: true, allowBoolean: true },
      ],
    },
  },
  {
    files: ["**/*.js"],
    extends: [js.configs.recommended],
    languageOptions: { globals: globals.node },
  },
]);
