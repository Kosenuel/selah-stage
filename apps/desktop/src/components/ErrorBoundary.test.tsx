import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { ErrorBoundary } from "./ErrorBoundary";

let shouldFail = true;

function Fragile() {
  if (shouldFail) {
    throw new Error("boom");
  }
  return <p>Fine now</p>;
}

describe("ErrorBoundary", () => {
  it("hides a broken part, keeps its siblings, and can try again", () => {
    vi.spyOn(console, "error").mockImplementation(() => undefined);
    shouldFail = true;

    render(
      <>
        <p>Still here</p>
        <ErrorBoundary label="settings panel">
          <Fragile />
        </ErrorBoundary>
      </>,
    );

    expect(screen.getByText("Still here")).toBeInTheDocument();
    expect(screen.getByRole("alert")).toHaveTextContent(
      "The settings panel had a problem and was hidden. The show is still running.",
    );

    shouldFail = false;
    fireEvent.click(screen.getByRole("button", { name: "Try again" }));

    expect(screen.getByText("Fine now")).toBeInTheDocument();
  });
});
