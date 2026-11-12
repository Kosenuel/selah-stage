import { useCallback, useEffect, useRef, useState } from "react";

import { commands, type AppError, type ServiceView } from "@/lib/bindings";
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
  loadFolder: (path: string) => void;
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

  const act = useCallback(
    (action: () => Promise<CommandResult<null>>) => {
      void action().then((result) => {
        if (!mounted.current) {
          return;
        }
        setActionError(result.status === "error" ? result.error : null);
        void refresh();
      });
    },
    [refresh],
  );

  return {
    view,
    error: pollError ?? actionError,
    go: () => {
      act(() => call(commands.go()));
    },
    skip: () => {
      act(() => call(commands.skip()));
    },
    panic: () => {
      act(() => call(commands.panicStop()));
    },
    reset: () => {
      act(() => call(commands.resetShow()));
    },
    loadFolder: (path) => {
      act(() => call(commands.loadFolder(path)));
    },
  };
}
