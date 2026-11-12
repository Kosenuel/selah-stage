import type { CueView_Serialize as CueView } from "@/lib/bindings";

/** A cue as it appears in the list: which level it sits at, and in what. */
export interface Row {
  cue: CueView;
  depth: number;
  parentId: string | null;
}

/** Every cue in the order the list shows them, groups followed by their children. */
export function flatten(cues: CueView[], depth = 0, parentId: string | null = null): Row[] {
  return cues.flatMap((cue) => [
    { cue, depth, parentId },
    ...flatten(cue.children, depth + 1, cue.id),
  ]);
}

/** The cues beside this one (in the same list or group), and where it is. */
export function siblingsOf(
  cues: CueView[],
  id: string,
): { siblings: CueView[]; index: number; parentId: string | null } | null {
  const search = (
    list: CueView[],
    parentId: string | null,
  ): { siblings: CueView[]; index: number; parentId: string | null } | null => {
    const index = list.findIndex((c) => c.id === id);
    if (index >= 0) {
      return { siblings: list, index, parentId };
    }
    for (const cue of list) {
      const found = search(cue.children, cue.id);
      if (found) {
        return found;
      }
    }
    return null;
  };
  return search(cues, null);
}

/**
 * The cue to insert before so that something lands just after `id`: its next
 * sibling, or failing that the next sibling of its group, and so on. `null`
 * means the end of the main list.
 */
export function nextAfter(cues: CueView[], id: string): string | null {
  const found = siblingsOf(cues, id);
  if (!found) {
    return null;
  }
  const next = found.siblings[found.index + 1];
  if (next) {
    return next.id;
  }
  return found.parentId === null ? null : nextAfter(cues, found.parentId);
}

/** The ids from `from` to `to` inclusive, in list order. */
export function rangeBetween(rows: Row[], from: string, to: string): string[] {
  const a = rows.findIndex((r) => r.cue.id === from);
  const b = rows.findIndex((r) => r.cue.id === to);
  if (a < 0 || b < 0) {
    return [to];
  }
  const [low, high] = a < b ? [a, b] : [b, a];
  return rows.slice(low, high + 1).map((r) => r.cue.id);
}
