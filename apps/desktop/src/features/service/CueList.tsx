import type { CueStatus, ServiceView_Serialize as ServiceView } from "@/lib/bindings";

const STATUS: Record<CueStatus, { symbol: string; label: string }> = {
  idle: { symbol: "○", label: "Ready" },
  waiting: { symbol: "◔", label: "Waiting" },
  running: { symbol: "▶", label: "Playing" },
  paused: { symbol: "❚❚", label: "Paused" },
  done: { symbol: "✓", label: "Done" },
  stopped: { symbol: "■", label: "Stopped" },
};

/** The cues in order. In edit mode a row can be selected to edit it. */
export function CueList({
  view,
  selectedId,
  onSelect,
}: {
  view: ServiceView | null;
  selectedId: string | null;
  onSelect: (id: string) => void;
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
        {view.cues.map((cue) => {
          const next = cue.id === view.playhead;
          const selected = view.editing && cue.id === selectedId;
          const status = STATUS[cue.status];
          return (
            <li
              key={cue.id}
              aria-current={next ? "step" : undefined}
              aria-selected={view.editing ? selected : undefined}
              onClick={() => {
                if (view.editing) {
                  onSelect(cue.id);
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
              <span
                className={`flex-1 truncate ${cue.armed ? "" : "text-text-faint line-through"}`}
              >
                {cue.name}
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
