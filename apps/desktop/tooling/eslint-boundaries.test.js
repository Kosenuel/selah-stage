// @vitest-environment node
import path from "node:path";

import { RuleTester } from "eslint";
import { afterAll, describe, it } from "vitest";

import { boundaries } from "./eslint-boundaries.js";

RuleTester.afterAll = afterAll;
RuleTester.describe = describe;
RuleTester.it = it;
RuleTester.itOnly = it.only;

const srcRoot = path.resolve("src");
const options = [{ srcRoot }];
const inSrc = (file) => path.join(srcRoot, ...file.split("/"));
const importing = (file, source) => ({
  code: `import x from "${source}";`,
  filename: inSrc(file),
  options,
});

new RuleTester().run("boundaries", boundaries, {
  valid: [
    importing("app/App.tsx", "@/features/cue-list/CueList"),
    importing("features/cue-list/CueList.tsx", "./useCues"),
    importing("features/cue-list/CueList.tsx", "../cue-list/rows/Row"),
    importing("features/cue-list/CueList.tsx", "@/components/Button"),
    importing("features/cue-list/CueList.tsx", "@/lib/ipc"),
    importing("features/cue-list/CueList.tsx", "react"),
    importing("features/cue-list/CueList.test.tsx", "@/test/render"),
    importing("outputs/OutputRoot.tsx", "@/components/Logo"),
    importing("components/Button.tsx", "@/lib/ipc"),
    { code: `export * from "./ipc";`, filename: inSrc("lib/index.ts"), options },
  ],
  invalid: [
    {
      ...importing("features/cue-list/CueList.tsx", "@/features/inspector/Inspector"),
      errors: [{ messageId: "crossFeature" }],
    },
    {
      ...importing("features/cue-list/CueList.tsx", "../inspector/Inspector"),
      errors: [{ messageId: "crossFeature" }],
    },
    {
      ...importing("features/cue-list/CueList.tsx", "@/app/App"),
      errors: [{ messageId: "layer" }],
    },
    {
      ...importing("features/cue-list/CueList.tsx", "@/test/render"),
      errors: [{ messageId: "layer" }],
    },
    {
      ...importing("outputs/OutputRoot.tsx", "@/features/cue-list/CueList"),
      errors: [{ messageId: "layer" }],
    },
    {
      ...importing("components/Button.tsx", "@/features/cue-list/CueList"),
      errors: [{ messageId: "layer" }],
    },
    {
      ...importing("lib/ipc.ts", "@/components/Button"),
      errors: [{ messageId: "layer" }],
    },
    {
      code: `const page = import("@/features/inspector/Inspector");`,
      filename: inSrc("features/cue-list/CueList.tsx"),
      options,
      errors: [{ messageId: "crossFeature" }],
    },
  ],
});
