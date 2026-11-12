import { useCallback, useEffect, useRef, useState } from "react";

import {
  commands,
  type AppError,
  type ControlType,
  type CueChangeInput_Deserialize as CueChange,
  type ServiceView_Serialize as ServiceView,
} from "@/lib/bindings";
import { call, type CommandResult } from "@/lib/ipc";

/** How often the control window asks the core what is happening. */
const POLL_MS = 100;

export interface Service {
  /** The latest state, or `null` until the first answer arrives. */
  view: ServiceView | null;
  /** The last problem from asking or from an action, in plain words. */
  error: AppError | null;
  go: () => void;
  skip: () => void;
  panic: () => void;
  reset: () => void;
  saveShow: () => void;
  undo: () => void;
  redo: () => void;
  setEditing: (editing: boolean) => void;
  /** Asks for a folder, then opens the show in it. */
  openShow: () => void;
  /** Asks for a folder, then creates an empty show in it. */
  newShow: () => void;
  /** Asks for a folder of audio files and loads them as cues, unsaved. */
  loadFolder: () => void;
  /** Resolve to `true` if the core accepted the change. */
  edit: (cue: string, change: CueChange) => Promise<boolean>;
  deleteCues: (cues: string[]) => Promise<boolean>;
  moveCue: (cue: string, before: string | null) => Promise<boolean>;
  renumber: () => Promise<boolean>;
  /** Resolve to the ids of what was created, so the caller can select them. */
  duplicateCue: (cue: string) => Promise<string[]>;
  addMemo: (before: string | null) => Promise<string[]>;
  /** Asks for audio files, then adds them as cues before `before`. */
  addAudio: (before: string | null) => Promise<string[]>;
  /** Adds a fade, stop, pause, go-to or arm cue that acts on `target`. */
  addControl: (kind: ControlType, target: string, before: string | null) => Promise<string[]>;
  /** Puts main-list cues into a new group. Resolves to the group's id. */
  groupCues: (cues: string[]) => Promise<string[]>;
  ungroup: (cue: string) => Promise<boolean>;
}

/**
 * The running service: polls the core for its state and exposes the operator's
 * actions. The core owns the state; this only shows it and sends commands.
 */
export function useService(): Service {
  const [view, setView] = useState<ServiceView | null>(null);
  // Kept apart so that a successful poll cannot wipe the message from a
  // failed action before the operator has seen it.
  const [pollError, setPollError] = useState<AppError | null>(null);
  const [actionError, setActionError] = useState<AppError | null>(null);
  const mounted = useRef(true);

  const refresh = useCallback(async () => {
    const result = await call(commands.serviceState());
    if (!mounted.current) {
      return;
    }
    if (result.status === "ok") {
      setView(result.data);
      setPollError(null);
    } else {
      setPollError(result.error);
    }
  }, []);

  useEffect(() => {
    mounted.current = true;
    const first = window.setTimeout(() => void refresh(), 0);
    const timer = window.setInterval(() => void refresh(), POLL_MS);
    return () => {
      mounted.current = false;
      window.clearTimeout(first);
      window.clearInterval(timer);
    };
  }, [refresh]);

  /** Runs an action, records its error, and refreshes. `null` if unmounted. */
  const attempt = useCallback(
    async <T>(action: () => Promise<CommandResult<T>>): Promise<CommandResult<T> | null> => {
      const result = await action();
      if (!mounted.current) {
        return null;
      }
      setActionError(result.status === "error" ? result.error : null);
      void refresh();
      return result;
    },
    [refresh],
  );

  const fire = useCallback(
    (action: () => Promise<CommandResult<null>>) => {
      void attempt(action);
    },
    [attempt],
  );
  const succeeded = useCallback(
    async (action: () => Promise<CommandResult<null>>) => (await attempt(action))?.status === "ok",
    [attempt],
  );
  const created = useCallback(
    async (action: () => Promise<CommandResult<string[]>>) => {
      const result = await attempt(action);
      return result?.status === "ok" ? result.data : [];
    },
    [attempt],
  );
  /** Asks for a folder, then runs `then` with it. Cancelling does nothing. */
  const withFolder = useCallback(
    (title: string, then: (path: string) => Promise<CommandResult<null>>) => {
      void (async () => {
        const picked = await attempt(() => call(commands.pickFolder(title)));
        const path = picked?.status === "ok" ? picked.data : null;
        if (path !== null) {
          await attempt(() => then(path));
        }
      })();
    },
    [attempt],
  );

  return {
    view,
    error: pollError ?? actionError,
    go: () => {
      fire(() => call(commands.go()));
    },
    skip: () => {
      fire(() => call(commands.skip()));
    },
    panic: () => {
      fire(() => call(commands.panicStop()));
    },
    reset: () => {
      fire(() => call(commands.resetShow()));
    },
    saveShow: () => {
      fire(() => call(commands.saveShow()));
    },
    undo: () => {
      fire(() => call(commands.undo()));
    },
    redo: () => {
      fire(() => call(commands.redo()));
    },
    setEditing: (editing) => {
      fire(() => call(commands.setEditing(editing)));
    },
    openShow: () => {
      withFolder("Open a show", (path) => call(commands.openShow(path)));
    },
    newShow: () => {
      withFolder("Choose an empty folder for the new show", (path) => call(commands.newShow(path)));
    },
    loadFolder: () => {
      withFolder("Choose a folder of audio files", (path) => call(commands.loadFolder(path)));
    },
    edit: (cue, change) => succeeded(() => call(commands.editCue(cue, change))),
    deleteCues: (cues) => succeeded(() => call(commands.deleteCues(cues))),
    moveCue: (cue, before) => succeeded(() => call(commands.moveCue(cue, before))),
    renumber: () => succeeded(() => call(commands.renumberCues())),
    duplicateCue: (cue) => created(() => call(commands.duplicateCue(cue))),
    addMemo: (before) => created(() => call(commands.addMemo(before))),
    addControl: (kind, target, before) =>
      created(() => call(commands.addControl(kind, target, before))),
    groupCues: (cues) => created(() => call(commands.groupCues(cues))),
    ungroup: (cue) => succeeded(() => call(commands.ungroup(cue))),
    addAudio: async (before) => {
      const picked = await attempt(() => call(commands.pickAudioFiles()));
      if (picked?.status !== "ok" || picked.data.length === 0) {
        return [];
      }
      const paths = picked.data;
      return created(() => call(commands.addAudio(paths, before)));
    },
  };
}
