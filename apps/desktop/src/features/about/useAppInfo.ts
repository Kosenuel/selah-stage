import { useEffect, useState } from "react";

import { commands, type AppInfo } from "@/lib/bindings";
import { call } from "@/lib/ipc";

export type AppInfoState =
  | { status: "loading" }
  | { status: "ready"; info: AppInfo }
  | { status: "failed"; message: string };

/** Reads the app details from the core once, when the component mounts. */
export function useAppInfo(): AppInfoState {
  const [state, setState] = useState<AppInfoState>({ status: "loading" });

  useEffect(() => {
    let cancelled = false;

    void call(commands.appInfo()).then((result) => {
      if (cancelled) {
        return;
      }
      setState(
        result.status === "ok"
          ? { status: "ready", info: result.data }
          : { status: "failed", message: result.error.message },
      );
    });

    return () => {
      cancelled = true;
    };
  }, []);

  return state;
}
