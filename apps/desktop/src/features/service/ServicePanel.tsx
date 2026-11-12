import { useEffect, useRef, useState } from "react";

import type { ControlType, ServiceView_Serialize as ServiceView } from "@/lib/bindings";

import { CueInspector, type TargetOption } from "./CueInspector";
import { CueList, type SelectModifiers } from "./CueList";
import { flatten, nextAfter, rangeBetween, siblingsOf } from "./tree";
import { useService, type Service } from "./useService";

const BUTTON = "border-border-strong bg-surface-raised rounded-lg border px-3 py-1.5 text-sm";
const DISABLED = "disabled:opacity-40";

/** The operator's view of the service: build it, then run it. */
export function ServicePanel() {
  const service = useService();
  const { view, error } = service;
  const [selection, setSelection] = useState<{ ids: string[]; anchor: string | null }>({
    ids: [],
    anchor: null,
  });

  const cues = view?.cues ?? [];
  const rows = flatten(cues);
  // Ignore selected cues that no longer exist (deleted, or undone away).
  const selectedIds = selection.ids.filter((id) => rows.some((r) => r.cue.id === id));
  const only = selectedIds.length === 1 ? rows.find((r) => r.cue.id === selectedIds[0]) : undefined;
  const selected = only?.cue;
  const place = selected ? siblingsOf(cues, selected.id) : null;
  const editing = view?.editing ?? true;

  const choose = (ids: string[], anchor: string | null = ids[0] ?? null) => {
    setSelection({ ids, anchor });
  };
  const select = (id: string, modifiers: SelectModifiers) => {
    if (modifiers.range && selection.anchor) {
      choose(rangeBetween(rows, selection.anchor, id), selection.anchor);
    } else if (modifiers.toggle) {
      const ids = selectedIds.includes(id)
        ? selectedIds.filter((i) => i !== id)
        : [...selectedIds, id];
      choose(ids, id);
    } else {
      choose([id], id);
    }
  };

  const insertBefore = selected ? nextAfter(cues, selected.id) : null;
  const selectFirstOf = (ids: string[]) => {
    if (ids[0]) {
      choose([ids[0]]);
    }
  };

  const addAudio = async () => {
    selectFirstOf(await service.addAudio(insertBefore));
  };
  const addNote = async () => {
    selectFirstOf(await service.addMemo(insertBefore));
  };
  const addControl = async (kind: ControlType) => {
    if (selected) {
      selectFirstOf(await service.addControl(kind, selected.id, insertBefore));
    }
  };
  const duplicate = async () => {
    if (selected) {
      selectFirstOf(await service.duplicateCue(selected.id));
    }
  };
  const remove = async () => {
    if (selectedIds.length === 0) {
      return;
    }
    const last = rows.findLast((r) => selectedIds.includes(r.cue.id));
    const lastIndex = last ? rows.indexOf(last) : -1;
    const neighbour =
      rows.slice(lastIndex + 1).find((r) => !selectedIds.includes(r.cue.id))?.cue.id ??
      rows.findLast((r) => !selectedIds.includes(r.cue.id))?.cue.id ??
      null;
    if (await service.deleteCues(selectedIds)) {
      choose(neighbour ? [neighbour] : []);
    }
  };
  const group = async () => {
    selectFirstOf(await service.groupCues(selectedIds));
  };
  const ungroup = async () => {
    if (selected && (await service.ungroup(selected.id))) {
      choose([]);
    }
  };
  const move = (direction: -1 | 1) => {
    if (!selected || !place) {
      return;
    }
    const { siblings, index } = place;
    if (direction === -1 && index > 0) {
      void service.moveCue(selected.id, siblings[index - 1]?.id ?? null);
    } else if (direction === 1 && index < siblings.length - 1) {
      void service.moveCue(selected.id, siblings[index + 2]?.id ?? null);
    }
  };
  const canMoveUp = !!place && place.index > 0;
  const canMoveDown = !!place && place.index < place.siblings.length - 1;
  // Only cues in the main list can be grouped.
  const groupable =
    selectedIds.length > 0 &&
    selectedIds.every((id) => rows.find((r) => r.cue.id === id)?.depth === 0);

  const targets: TargetOption[] = rows.map((r) => ({
    id: r.cue.id,
    label: `${r.cue.number}. ${r.cue.name}`,
    topLevel: r.depth === 0,
  }));

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
            <label className="sr-only" htmlFor="add-control">
              Add control cue
            </label>
            <select
              id="add-control"
              className={`${BUTTON} ${DISABLED}`}
              disabled={!selected}
              value=""
              onChange={(e) => {
                if (e.target.value) {
                  void addControl(e.target.value as ControlType);
                }
              }}
            >
              <option value="">Add control cue…</option>
              <option value="fade">Fade the selected cue</option>
              <option value="stop">Stop the selected cue</option>
              <option value="pause">Pause the selected cue</option>
              <option value="goTo">Go to the selected cue</option>
              <option value="arm">Arm or disarm the selected cue</option>
            </select>
            <button
              type="button"
              className={`${BUTTON} ${DISABLED}`}
              disabled={!groupable}
              onClick={() => void group()}
            >
              Group
            </button>
            <button
              type="button"
              className={`${BUTTON} ${DISABLED}`}
              disabled={!selected?.group}
              onClick={() => void ungroup()}
            >
              Ungroup
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
              disabled={!canMoveUp}
              onClick={() => {
                move(-1);
              }}
            >
              Move up
            </button>
            <button
              type="button"
              className={`${BUTTON} ${DISABLED}`}
              disabled={!canMoveDown}
              onClick={() => {
                move(1);
              }}
            >
              Move down
            </button>
            <button
              type="button"
              className={`${BUTTON} ${DISABLED}`}
              disabled={selectedIds.length === 0}
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

        <CueList view={view} selectedIds={selectedIds} onSelect={select} />

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
          targets={targets}
          onChange={(change) => {
            void service.edit(selected.id, change);
          }}
        />
      )}
      {editing && selectedIds.length > 1 && (
        <aside
          aria-label="Selection"
          className="w-80 shrink-0 rounded-xl border border-border bg-surface p-4 text-sm text-text-muted"
        >
          {selectedIds.length} cues selected. You can group, delete, or move on to another cue.
        </aside>
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
