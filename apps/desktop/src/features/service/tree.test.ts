import { describe, expect, it } from "vitest";

import type { CueView_Serialize as CueView } from "@/lib/bindings";

import { flatten, nextAfter, rangeBetween, siblingsOf } from "./tree";

function cue(id: string, children: CueView[] = []): CueView {
  return {
    id,
    number: id,
    name: id,
    status: "idle",
    armed: true,
    notes: "",
    preWait: 0,
    postWait: 0,
    follow: "none",
    audio: null,
    control: null,
    group: children.length > 0 ? { mode: "together", looped: false } : null,
    children,
  };
}

// a, g(x, y), b
const cues = [cue("a"), cue("g", [cue("x"), cue("y")]), cue("b")];

describe("tree helpers", () => {
  it("flattens in display order with depths and parents", () => {
    expect(flatten(cues).map((r) => [r.cue.id, r.depth, r.parentId])).toEqual([
      ["a", 0, null],
      ["g", 0, null],
      ["x", 1, "g"],
      ["y", 1, "g"],
      ["b", 0, null],
    ]);
  });

  it("finds a cue's siblings at any depth", () => {
    expect(siblingsOf(cues, "y")).toMatchObject({ index: 1, parentId: "g" });
    expect(siblingsOf(cues, "b")).toMatchObject({ index: 2, parentId: null });
    expect(siblingsOf(cues, "nope")).toBeNull();
  });

  it("finds where to insert so something lands just after a cue", () => {
    expect(nextAfter(cues, "a")).toBe("g");
    expect(nextAfter(cues, "x")).toBe("y");
    expect(nextAfter(cues, "y")).toBe("b"); // last in its group: after the group
    expect(nextAfter(cues, "b")).toBeNull();
  });

  it("selects a range in display order, in either direction", () => {
    const rows = flatten(cues);
    expect(rangeBetween(rows, "a", "x")).toEqual(["a", "g", "x"]);
    expect(rangeBetween(rows, "y", "g")).toEqual(["g", "x", "y"]);
    expect(rangeBetween(rows, "a", "missing")).toEqual(["missing"]);
  });
});
