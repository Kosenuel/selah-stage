import type { CueStatus, ServiceView_Serialize as ServiceView } from "@/lib/bindings";

import { flatten } from "./tree";

const STATUS: Record<CueStatus, { symbol: string; label: string }> = {
  idle: { symbol: "○", label: "Ready" },
  waiting: { symbol: "◔", label: "Waiting" },
  running: { symbol: "▶", label: "Playing" },
  paused: { symbol: "❚❚", label: "Paused" },
  done: { symbol: "✓", label: "Done" },
  stopped: { symbol: "■", label: "Stopped" },
};

const GROUP_MODE = {
  together: "all at once",
  sequence: "one after another",
  shuffle: "one at random",
} as const;

const CONTROL_VERB = {
  fade: "Fades",
  stop: "Stops",
  pause: "Pauses",
  goTo: "Moves the playhead to",
  arm: "Arms or disarms",
} as const;

export interface SelectModifiers {
  /** Ctrl or Cmd: add or remove this cue from the selection. */
  toggle: boolean;
  /** Shift: select everything from the last click to this cue. */
  range: boolean;
}

/** The cues in order, groups showing the cues inside them. */
export function CueList({
  view,
  selectedIds,
  onSelect,
}: {
  view: ServiceView | null;
  selectedIds: string[];
  onSelect: (id: string, modifiers: SelectModifiers) => void;
}) {
  if (!view || view.cues.length === 0) {
    return (
      <p className="rounded-lg border border-dashed border-border px-4 py-10 text-center text-sm text-text-muted">
        {view?.mediaRoot
          ? "This show has no cues yet. Choose Add audio to start."
          : "No show yet. Create a new show, open a saved one, or load a folder of audio files."}
      </p>
    );
  }
  return (
    <section aria-label={view.title || "Cues"} className="min-h-0 flex-1 overflow-auto">
      <ol className="flex flex-col gap-1">
        {flatten(view.cues).map(({ cue, depth }) => {
          const next = cue.id === view.playhead;
          const selected = view.editing && selectedIds.includes(cue.id);
          const status = STATUS[cue.status];
          return (
            <li
              key={cue.id}
              aria-current={next ? "step" : undefined}
              aria-selected={view.editing ? selected : undefined}
              style={{ marginLeft: `${depth * 28}px` }}
              onClick={(event) => {
                if (view.editing) {
                  onSelect(cue.id, {
                    toggle: event.ctrlKey || event.metaKey,
                    range: event.shiftKey,
                  });
                }
              }}
              className={`flex items-center gap-3 rounded-lg border px-4 py-3 ${
                selected
                  ? "border-accent bg-surface-raised"
                  : next
                    ? "border-standby bg-surface-raised"
                    : "border-border bg-surface"
              } ${view.editing ? "cursor-pointer" : ""}`}
            >
              <span className="w-8 text-sm text-text-faint">{cue.number}</span>
              <span className="flex min-w-0 flex-1 flex-col">
                <span className={`truncate ${cue.armed ? "" : "text-text-faint line-through"}`}>
                  {cue.group && <span aria-hidden="true">▾ </span>}
                  {cue.name}
                </span>
                {cue.group && (
                  <span className="text-xs text-text-faint">
                    Group: {GROUP_MODE[cue.group.mode]}
                    {cue.group.looped ? ", repeats" : ""}
                  </span>
                )}
                {cue.control && (
                  <span
                    className={`text-xs ${cue.control.targetName === null ? "text-standby" : "text-text-faint"}`}
                  >
                    {cue.control.targetName === null ? (
                      <>
                        <span aria-hidden="true">⚠ </span>The cue this acts on was deleted
                      </>
                    ) : (
                      `${CONTROL_VERB[cue.control.kind]} ${cue.control.targetName}`
                    )}
                  </span>
                )}
              </span>
              {!cue.armed && <span className="text-xs text-text-faint">Disarmed</span>}
              {next && <span className="text-xs font-semibold text-standby">NEXT</span>}
              <span className="flex items-center gap-2 text-sm text-text-muted">
                <span aria-hidden="true">{status.symbol}</span>
                {status.label}
              </span>
            </li>
          );
        })}
      </ol>
    </section>
  );
}
