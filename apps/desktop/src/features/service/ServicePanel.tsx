import { useEffect, useRef, useState } from "react";

import type { ServiceView_Serialize as ServiceView } from "@/lib/bindings";

import { CueInspector } from "./CueInspector";
import { CueList } from "./CueList";
import { useService, type Service } from "./useService";

const BUTTON = "border-border-strong bg-surface-raised rounded-lg border px-3 py-1.5 text-sm";
const DISABLED = "disabled:opacity-40";

/** The operator's view of the service: build it, then run it. */
export function ServicePanel() {
  const service = useService();
  const { view, error } = service;
  const [selectedId, setSelectedId] = useState<string | null>(null);

  const cues = view?.cues ?? [];
  const index = cues.findIndex((c) => c.id === selectedId);
  const selected = index >= 0 ? cues[index] : undefined;
  const editing = view?.editing ?? true;

  const addAfterSelection = cues[index + 1]?.id ?? null;

  const addAudio = async () => {
    const [first] = await service.addAudio(addAfterSelection);
    if (first) {
      setSelectedId(first);
    }
  };
  const addNote = async () => {
    const [first] = await service.addMemo(addAfterSelection);
    if (first) {
      setSelectedId(first);
    }
  };
  const duplicate = async () => {
    if (selected) {
      const [first] = await service.duplicateCue(selected.id);
      if (first) {
        setSelectedId(first);
      }
    }
  };
  const remove = async () => {
    if (selected) {
      const neighbour = cues[index + 1]?.id ?? cues[index - 1]?.id ?? null;
      if (await service.deleteCues([selected.id])) {
        setSelectedId(neighbour);
      }
    }
  };
  const move = (direction: -1 | 1) => {
    if (!selected) {
      return;
    }
    if (direction === -1 && index > 0) {
      void service.moveCue(selected.id, cues[index - 1]?.id ?? null);
    } else if (direction === 1 && index < cues.length - 1) {
      void service.moveCue(selected.id, cues[index + 2]?.id ?? null);
    }
  };

  useGlobalKeys(service, editing, { remove, move });

  return (
    <div className="flex h-full w-full gap-4">
      <div className="flex h-full min-w-0 flex-1 flex-col gap-3">
        <ShowBar service={service} />
        {view && <ShowLine view={view} />}

        {error && (
          <p role="alert" className="rounded-lg border border-danger px-4 py-3 text-sm text-danger">
            {error.message}
          </p>
        )}

        {view && <DeviceLine view={view} />}

        {editing && view?.mediaRoot && (
          <div role="toolbar" aria-label="Edit cues" className="flex flex-wrap gap-2">
            <button type="button" className={BUTTON} onClick={() => void addAudio()}>
              Add audio
            </button>
            <button type="button" className={BUTTON} onClick={() => void addNote()}>
              Add note
            </button>
            <button
              type="button"
              className={`${BUTTON} ${DISABLED}`}
              disabled={!selected}
              onClick={() => void duplicate()}
            >
              Duplicate
            </button>
            <button
              type="button"
              className={`${BUTTON} ${DISABLED}`}
              disabled={!selected || index === 0}
              onClick={() => {
                move(-1);
              }}
            >
              Move up
            </button>
            <button
              type="button"
              className={`${BUTTON} ${DISABLED}`}
              disabled={!selected || index === cues.length - 1}
              onClick={() => {
                move(1);
              }}
            >
              Move down
            </button>
            <button
              type="button"
              className={`${BUTTON} ${DISABLED}`}
              disabled={!selected}
              onClick={() => void remove()}
            >
              Delete
            </button>
            <button
              type="button"
              className={`${BUTTON} ${DISABLED}`}
              disabled={cues.length === 0}
              onClick={() => void service.renumber()}
            >
              Renumber
            </button>
          </div>
        )}

        <CueList view={view} selectedId={selectedId} onSelect={setSelectedId} />

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

      {editing && selected && (
        <CueInspector
          key={selected.id}
          cue={selected}
          onChange={(change) => {
            void service.edit(selected.id, change);
          }}
        />
      )}
    </div>
  );
}

function ShowBar({ service }: { service: Service }) {
  const { view } = service;
  const editing = view?.editing ?? true;
  return (
    <div className="flex flex-wrap items-center gap-2">
      <button type="button" className={BUTTON} onClick={service.newShow}>
        New show
      </button>
      <button type="button" className={BUTTON} onClick={service.openShow}>
        Open show
      </button>
      <button type="button" className={BUTTON} onClick={service.loadFolder}>
        Load folder
      </button>
      <button
        type="button"
        className={`${BUTTON} ${DISABLED}`}
        disabled={!view?.mediaRoot}
        onClick={service.saveShow}
      >
        Save show
      </button>
      <span className="flex-1" />
      <button
        type="button"
        className={`${BUTTON} ${DISABLED}`}
        disabled={!editing || !view?.canUndo}
        onClick={service.undo}
      >
        Undo
      </button>
      <button
        type="button"
        className={`${BUTTON} ${DISABLED}`}
        disabled={!editing || !view?.canRedo}
        onClick={service.redo}
      >
        Redo
      </button>
      <button
        type="button"
        aria-pressed={editing}
        onClick={() => {
          service.setEditing(!editing);
        }}
        className={`rounded-lg border px-3 py-1.5 text-sm font-semibold ${
          editing ? "border-accent text-accent" : "border-live text-live"
        }`}
      >
        {editing ? "✎ Edit mode" : "▶ Show mode"}
      </button>
    </div>
  );
}

function ShowLine({ view }: { view: ServiceView }) {
  if (!view.mediaRoot) {
    return null;
  }
  return (
    <div className="flex flex-col gap-1 text-sm">
      <p className="truncate text-text-muted">
        {view.location
          ? `Saved in ${view.location}. Changes are saved automatically.`
          : "Not saved yet. Save it to keep your changes."}
      </p>
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

interface EditKeys {
  remove: () => Promise<void>;
  move: (direction: -1 | 1) => void;
}

/**
 * Keyboard control. Space is GO and Esc is panic. Ctrl+E switches mode, and in
 * edit mode Ctrl+Z, Ctrl+Y, Delete, and Alt+Up or Down edit. Nothing but the
 * mode switch fires while the operator is typing in a field.
 */
function useGlobalKeys(service: Service, editing: boolean, keys: EditKeys) {
  // Always the latest handlers, without re-subscribing on every render.
  const latest = useRef({ service, editing, keys });
  useEffect(() => {
    latest.current = { service, editing, keys };
  });

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const { service, editing, keys } = latest.current;
      const target = event.target;
      const typing =
        target instanceof HTMLElement &&
        (target.tagName === "INPUT" ||
          target.tagName === "TEXTAREA" ||
          target.tagName === "SELECT" ||
          target.isContentEditable);
      const command = event.ctrlKey || event.metaKey;

      if (command && event.code === "KeyE") {
        event.preventDefault();
        service.setEditing(!editing);
        return;
      }
      if (typing || event.repeat) {
        return;
      }
      if (command && event.code === "KeyS") {
        event.preventDefault();
        service.saveShow();
      } else if (command && editing && event.code === "KeyZ") {
        event.preventDefault();
        if (event.shiftKey) {
          service.redo();
        } else {
          service.undo();
        }
      } else if (command && editing && event.code === "KeyY") {
        event.preventDefault();
        service.redo();
      } else if (!command && event.code === "Space") {
        event.preventDefault();
        service.go();
      } else if (!command && event.code === "Escape") {
        event.preventDefault();
        service.panic();
      } else if (editing && event.code === "Delete") {
        event.preventDefault();
        void keys.remove();
      } else if (editing && event.altKey && event.code === "ArrowUp") {
        event.preventDefault();
        keys.move(-1);
      } else if (editing && event.altKey && event.code === "ArrowDown") {
        event.preventDefault();
        keys.move(1);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
    };
  }, []);
}
