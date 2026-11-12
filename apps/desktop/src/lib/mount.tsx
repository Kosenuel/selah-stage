import { StrictMode, type ReactNode } from "react";
import { createRoot } from "react-dom/client";

/** Renders a window's root component. Every entry point starts here. */
export function mount(node: ReactNode): void {
  const container = document.getElementById("root");
  if (!container) {
    throw new Error("The page is missing its #root element.");
  }
  createRoot(container).render(<StrictMode>{node}</StrictMode>);
}
