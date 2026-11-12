import { ErrorBoundary } from "@/components/ErrorBoundary";
import { Logo } from "@/components/Logo";
import { AboutPanel } from "@/features/about/AboutPanel";
import { ServicePanel } from "@/features/service/ServicePanel";

/** The control window: the operator's view of the whole service. */
export function App() {
  return (
    <div className="flex h-full flex-col bg-canvas text-text">
      <header className="flex h-12 shrink-0 items-center gap-3 border-b border-border bg-surface px-4">
        <Logo className="size-6" />
        <h1 className="text-sm font-semibold tracking-wide">Selah Stage</h1>
        <span className="rounded-full border border-border-strong px-2 py-0.5 text-xs text-text-muted">
          Pre-alpha
        </span>
      </header>
      <main className="flex min-h-0 flex-1 overflow-auto p-6">
        <ErrorBoundary label="service panel">
          <ServicePanel />
        </ErrorBoundary>
      </main>
      <footer className="shrink-0 border-t border-border bg-surface px-4 py-2 text-sm">
        <details>
          <summary className="cursor-pointer text-text-muted">About this build</summary>
          <div className="flex justify-center py-4">
            <ErrorBoundary label="About panel">
              <AboutPanel />
            </ErrorBoundary>
          </div>
        </details>
      </footer>
    </div>
  );
}
