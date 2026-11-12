/**
 * Enforces the front-end layering from CLAUDE.md:
 *
 * - a feature never imports from another feature;
 * - `components/` and `lib/` never reach up into features or windows;
 * - output windows never import control-UI code, so they stay plain renderers.
 *
 * Usage in eslint.config.js: `"selah/boundaries": ["error", { srcRoot }]`.
 */
import path from "node:path";

/** For each top-level folder under `src/`, the folders it may import from. */
const ALLOWED = {
  app: ["app", "features", "components", "lib", "styles"],
  features: ["features", "components", "lib", "styles"],
  outputs: ["outputs", "components", "lib", "styles"],
  components: ["components", "lib", "styles"],
  lib: ["lib"],
  styles: ["styles"],
  test: ["test", "components", "lib", "styles"],
};

const TEST_FILE = /\.test\.[jt]sx?$/;

/** @returns {{ layer: string, feature?: string } | null} */
function classify(srcRoot, file) {
  const relative = path.relative(srcRoot, file);
  if (relative.startsWith("..") || path.isAbsolute(relative)) {
    return null;
  }
  const [layer, second] = relative.split(path.sep);
  if (layer === undefined || !(layer in ALLOWED)) {
    return null;
  }
  return layer === "features" ? { layer, feature: second } : { layer };
}

function resolveImport(srcRoot, file, source) {
  if (source.startsWith("@/")) {
    return path.join(srcRoot, source.slice(2));
  }
  if (source.startsWith(".")) {
    return path.resolve(path.dirname(file), source);
  }
  return null; // A package import; not ours to police.
}

/** @type {import("eslint").Rule.RuleModule} */
export const boundaries = {
  meta: {
    type: "problem",
    docs: { description: "Keep imports within the layering described in CLAUDE.md." },
    schema: [
      {
        type: "object",
        properties: { srcRoot: { type: "string" } },
        required: ["srcRoot"],
        additionalProperties: false,
      },
    ],
    messages: {
      crossFeature:
        "Feature '{{from}}' must not import from feature '{{to}}'. Share code through components/ or lib/.",
      layer: "{{from}}/ must not import from {{to}}/. It may import from: {{allowed}}.",
    },
  },
  create(context) {
    const [{ srcRoot }] = context.options;
    const from = classify(srcRoot, context.filename);
    if (from === null) {
      return {};
    }
    const allowed = [...ALLOWED[from.layer]];
    if (TEST_FILE.test(context.filename)) {
      allowed.push("test"); // Only tests may use test helpers.
    }

    function check(node) {
      if (node === null || node === undefined || typeof node.value !== "string") {
        return;
      }
      const target = resolveImport(srcRoot, context.filename, node.value);
      const to = target === null ? null : classify(srcRoot, target);
      if (to === null) {
        return;
      }
      if (!allowed.includes(to.layer)) {
        context.report({
          node,
          messageId: "layer",
          data: { from: from.layer, to: to.layer, allowed: allowed.join(", ") },
        });
      } else if (
        from.layer === "features" &&
        to.layer === "features" &&
        from.feature !== to.feature
      ) {
        context.report({
          node,
          messageId: "crossFeature",
          data: { from: from.feature, to: to.feature },
        });
      }
    }

    return {
      ImportDeclaration: (node) => check(node.source),
      ExportNamedDeclaration: (node) => check(node.source),
      ExportAllDeclaration: (node) => check(node.source),
      ImportExpression: (node) => check(node.source),
    };
  },
};
