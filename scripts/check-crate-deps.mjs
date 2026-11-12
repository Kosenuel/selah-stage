#!/usr/bin/env node
// Enforces the crate dependency rules from CLAUDE.md. Part of `pnpm check`.
//
//   1. selah-core depends on none of our other crates.
//   2. selah-engine never depends on selah-audio (it is driven through traits).
//   3. Only the desktop shell depends on Tauri, so every crate stays testable
//      without a window.
//   4. Nothing depends on the desktop shell.
import { execFileSync } from "node:child_process";

const SHELL = "selah-desktop";

const metadata = JSON.parse(
  execFileSync("cargo", ["metadata", "--no-deps", "--format-version", "1"], {
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
  }),
);

const problems = [];

for (const pkg of metadata.packages) {
  const deps = pkg.dependencies.map((dependency) => dependency.name);
  const ours = deps.filter((name) => name.startsWith("selah-"));

  if (pkg.name === "selah-core" && ours.length > 0) {
    problems.push(`selah-core must not depend on other selah crates (found ${ours.join(", ")}).`);
  }
  if (pkg.name === "selah-engine" && deps.includes("selah-audio")) {
    problems.push("selah-engine must not depend on selah-audio; drive audio through a trait.");
  }
  if (pkg.name !== SHELL) {
    const tauri = deps.filter((name) => name === "tauri" || name.startsWith("tauri-"));
    if (tauri.length > 0) {
      problems.push(`${pkg.name} must not depend on Tauri (found ${tauri.join(", ")}).`);
    }
    if (deps.includes(SHELL)) {
      problems.push(`${pkg.name} must not depend on ${SHELL}.`);
    }
  }
}

if (problems.length > 0) {
  console.error("Crate dependency rules broken:\n");
  for (const problem of problems) {
    console.error(`  - ${problem}`);
  }
  console.error("\nSee 'Coding conventions' in CLAUDE.md.");
  process.exit(1);
}

console.log(`Crate dependency rules hold for ${metadata.packages.length} crates.`);
