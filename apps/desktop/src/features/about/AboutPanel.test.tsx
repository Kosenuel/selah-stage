import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { mockCommands } from "@/lib/bindings";

import { AboutPanel } from "./AboutPanel";

describe("AboutPanel", () => {
  it("shows the details the core reports", async () => {
    mockCommands({
      appInfo: () => ({
        name: "Selah Stage",
        version: "1.2.3",
        os: "windows",
        arch: "x86_64",
        webviewVersion: "130.0.2849.56",
        debugBuild: false,
      }),
    });

    render(<AboutPanel />);

    expect(screen.getByRole("status")).toHaveTextContent("Reading app details");
    expect(await screen.findByText("1.2.3")).toBeInTheDocument();
    expect(screen.getByText("windows · x86_64")).toBeInTheDocument();
    expect(screen.getByText("130.0.2849.56")).toBeInTheDocument();
  });

  it("marks development builds and copes with an unknown webview", async () => {
    mockCommands({
      appInfo: () => ({
        name: "Selah Stage",
        version: "1.2.3",
        os: "linux",
        arch: "x86_64",
        webviewVersion: null,
        debugBuild: true,
      }),
    });

    render(<AboutPanel />);

    expect(await screen.findByText("1.2.3 (development build)")).toBeInTheDocument();
    expect(screen.getByText("Unknown")).toBeInTheDocument();
  });

  it("shows the core's message when the details cannot be read", async () => {
    mockCommands({
      appInfo: () => {
        // Commands reject with the serialised `AppError`, not an `Error`.
        // eslint-disable-next-line @typescript-eslint/only-throw-error -- mirrors what the core sends
        throw { kind: "internal", message: "Restart Selah Stage and try again." };
      },
    });

    render(<AboutPanel />);

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Can’t read the app details. Restart Selah Stage and try again.",
    );
  });
});
