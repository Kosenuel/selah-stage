import { useEffect, useState, type SyntheticEvent } from "react";

import type { CueStatus, ServiceView } from "@/lib/bindings";

import { useService } from "./useService";

const STATUS: Record<CueStatus, { symbol: string; label: string }> = {
  idle: { symbol: "○", label: "Ready" },
  waiting: { symbol: "◔", label: "Waiting" },
  running: { symbol: "▶", label: "Playing" },
  paused: { symbol: "❚❚", label: "Paused" },
  done: { symbol: "✓", label: "Done" },
  stopped: { symbol: "■", label: "Stopped" },
};

/** The operator's view of the service: the list, GO, and the safety keys. */
export function ServicePanel() {
  const service = useService();
  const { view, error } = service;

  useGlobalKeys(service);

  return (
    <div className="flex h-full w-full max-w-3xl flex-col gap-4">
      <FolderForm onLoadFolder={service.loadFolder} onOpenShow={service.openShow} />
      {view && <ShowLine view={view} onSave={service.saveShow} />}

      {error && (
        <p role="alert" className="rounded-lg border border-danger px-4 py-3 text-sm text-danger">
          {error.message}
        </p>
      )}

      {view && <DeviceLine view={view} />}

      <CueList view={view} onSetArmed={service.setArmed} />

      {view && view.warnings.length > 0 && (
        <ul aria-label="Warnings" className="flex flex-col gap-1 text-sm">
          {view.warnings.map((w, i) => (
            <li key={i} className="text-standby">
              <span aria-hidden="true">⚠ </span>
              <span className="font-medium">{w.cue}: </span>
              {w.message}
            </li>
          ))}
        </ul>
      )}

      <div className="mt-auto flex items-stretch gap-3">
        <button
          type="button"
          onClick={service.go}
          className="min-h-16 flex-1 rounded-xl bg-accent-strong text-xl font-semibold text-on-accent"
        >
          GO <span className="text-sm font-normal opacity-80">(Space)</span>
        </button>
        <button
          type="button"
          onClick={service.skip}
          className="rounded-xl border border-border-strong bg-surface-raised px-5"
        >
          Skip
        </button>
        <button
          type="button"
          onClick={service.reset}
          className="rounded-xl border border-border-strong bg-surface-raised px-5"
        >
          Reset
        </button>
        <button
          type="button"
          onClick={service.panic}
          className="rounded-xl border-2 border-danger px-5 font-semibold text-danger"
        >
          Panic <span className="text-xs font-normal">(Esc)</span>
        </button>
      </div>
    </div>
  );
}

function FolderForm({
  onLoadFolder,
  onOpenShow,
}: {
  onLoadFolder: (path: string) => void;
  onOpenShow: (path: string) => void;
}) {
  const [path, setPath] = useState("");
  const run = (action: (path: string) => void) => (event: SyntheticEvent) => {
    event.preventDefault();
    if (path.trim() !== "") {
      action(path);
    }
  };
  return (
    <form onSubmit={run(onOpenShow)} className="flex gap-2">
      <label className="flex-1">
        <span className="sr-only">Folder</span>
        <input
          type="text"
          value={path}
          onChange={(e) => {
            setPath(e.target.value);
          }}
          placeholder="A show folder, or a folder of audio files, for example C:\Music\Sunday"
          className="w-full rounded-lg border border-border bg-surface px-3 py-2 text-sm text-text select-text"
        />
      </label>
      <button
        type="submit"
        className="rounded-lg border border-border-strong bg-surface-raised px-4 text-sm"
      >
        Open show
      </button>
      <button
        type="button"
        onClick={run(onLoadFolder)}
        className="rounded-lg border border-border-strong bg-surface-raised px-4 text-sm"
      >
        Load folder
      </button>
    </form>
  );
}

function ShowLine({ view, onSave }: { view: ServiceView; onSave: () => void }) {
  if (view.cues.length === 0) {
    return null;
  }
  return (
    <div className="flex flex-col gap-1 text-sm">
      <div className="flex items-center gap-3 text-text-muted">
        <span className="flex-1 truncate">
          {view.location
            ? `Saved in ${view.location}. Changes are saved automatically.`
            : "Not saved yet. Save it to keep your changes."}
        </span>
        <button
          type="button"
          onClick={onSave}
          className="rounded-lg border border-border-strong bg-surface-raised px-3 py-1"
        >
          Save show
        </button>
      </div>
      {view.notice && (
        <p role="status" className="text-standby">
          <span aria-hidden="true">⚠ </span>
          {view.notice}
        </p>
      )}
      {view.saveProblem && (
        <p role="alert" className="text-danger">
          {view.saveProblem}
        </p>
      )}
    </div>
  );
}

function DeviceLine({ view }: { view: ServiceView }) {
  const { device } = view;
  const peak = Math.max(0, ...view.peaks.map((p) => p ?? 0));
  return (
    <div className="flex items-center gap-3 text-sm text-text-muted">
      <span>
        {device.audible ? `Sound: ${device.name} · ${device.sampleRate / 1000} kHz` : "No sound"}
      </span>
      <div
        role="meter"
        aria-label="Output level"
        aria-valuemin={0}
        aria-valuemax={1}
        aria-valuenow={Math.min(1, peak)}
        className="h-2 w-40 overflow-hidden rounded-full bg-surface-raised"
      >
        <div className="h-full bg-live" style={{ width: `${Math.min(1, peak) * 100}%` }} />
      </div>
      {view.underruns > 0 && <span className="text-standby">Dropouts: {view.underruns}</span>}
      {device.problem && <span className="text-danger">{device.problem}</span>}
    </div>
  );
}

function CueList({
  view,
  onSetArmed,
}: {
  view: ServiceView | null;
  onSetArmed: (cue: string, armed: boolean) => void;
}) {
  if (!view || view.cues.length === 0) {
    return (
      <p className="rounded-lg border border-dashed border-border px-4 py-10 text-center text-sm text-text-muted">
        No cues yet. Open a saved show, or load a folder of audio files.
      </p>
    );
  }
  return (
    <section aria-label={view.title || "Cues"} className="min-h-0 flex-1 overflow-auto">
      <ol className="flex flex-col gap-1">
        {view.cues.map((cue) => {
          const next = cue.id === view.playhead;
          const status = STATUS[cue.status];
          return (
            <li
              key={cue.id}
              aria-current={next ? "step" : undefined}
              className={`flex items-center gap-3 rounded-lg border px-4 py-3 ${
                next ? "border-standby bg-surface-raised" : "border-border bg-surface"
              }`}
            >
              <span className="w-8 text-sm text-text-faint">{cue.number}</span>
              <span
                className={`flex-1 truncate ${cue.armed ? "" : "text-text-faint line-through"}`}
              >
                {cue.name}
              </span>
              <label className="flex items-center gap-1 text-xs text-text-muted">
                <input
                  type="checkbox"
                  aria-label={`Armed: ${cue.name}`}
                  checked={cue.armed}
                  onChange={(e) => {
                    onSetArmed(cue.id, e.target.checked);
                  }}
                />
                Armed
              </label>
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

/** Space is GO and Esc is panic, unless the operator is typing. */
function useGlobalKeys({ go, panic }: { go: () => void; panic: () => void }) {
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const target = event.target;
      const typing =
        target instanceof HTMLElement &&
        (target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.isContentEditable);
      if (typing || event.repeat) {
        return;
      }
      if (event.code === "Space") {
        event.preventDefault();
        go();
      } else if (event.code === "Escape") {
        event.preventDefault();
        panic();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
    };
  }, [go, panic]);
}
