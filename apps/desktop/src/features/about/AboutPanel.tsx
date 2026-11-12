import { Logo } from "@/components/Logo";
import type { AppInfo } from "@/lib/bindings";

import { useAppInfo } from "./useAppInfo";

/** What Selah Stage is and exactly which build is running. */
export function AboutPanel() {
  const state = useAppInfo();

  return (
    <section
      aria-labelledby="about-title"
      className="w-full max-w-md rounded-xl border border-border bg-surface p-8"
    >
      <div className="flex items-center gap-4">
        <Logo className="size-14 shrink-0" />
        <div>
          <h2 id="about-title" className="text-xl font-semibold">
            Selah Stage
          </h2>
          <p className="text-sm text-text-muted">Run the whole service from one cue list.</p>
        </div>
      </div>

      <p className="mt-6 text-sm leading-relaxed text-text-muted">
        This is an early build. The foundation is in place; shows, cues, and outputs are still to
        come.
      </p>

      <div className="mt-6 border-t border-border pt-6">
        {state.status === "loading" && (
          <p className="text-sm text-text-faint" role="status">
            Reading app details…
          </p>
        )}
        {state.status === "failed" && (
          <p className="text-sm text-danger" role="alert">
            <span className="font-semibold">Can’t read the app details.</span> {state.message}
          </p>
        )}
        {state.status === "ready" && <Details info={state.info} />}
      </div>
    </section>
  );
}

function Details({ info }: { info: AppInfo }) {
  const rows: [label: string, value: string][] = [
    ["Version", info.debugBuild ? `${info.version} (development build)` : info.version],
    ["System", `${info.os} · ${info.arch}`],
    ["Webview", info.webviewVersion ?? "Unknown"],
    ["Licence", "Apache-2.0"],
  ];

  return (
    <dl className="grid grid-cols-[auto_1fr] gap-x-6 gap-y-2 text-sm">
      {rows.map(([label, value]) => (
        <div key={label} className="contents">
          <dt className="text-text-faint">{label}</dt>
          <dd className="font-mono select-text">{value}</dd>
        </div>
      ))}
    </dl>
  );
}
