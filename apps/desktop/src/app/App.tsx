import { Logo } from "@/components/Logo";
import { AboutPanel } from "@/features/about/AboutPanel";

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
      <main className="flex min-h-0 flex-1 items-center justify-center overflow-auto p-8">
        <AboutPanel />
      </main>
    </div>
  );
}
