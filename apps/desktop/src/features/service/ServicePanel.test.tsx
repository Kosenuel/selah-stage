import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { mockCommands, type ServiceView_Serialize } from "@/lib/bindings";

import { ServicePanel } from "./ServicePanel";

const view: ServiceView_Serialize = {
  title: "Sunday",
  cues: [
    { id: "a", number: "1", name: "Walk-in", status: "running" },
    { id: "b", number: "2", name: "Welcome", status: "idle" },
  ],
  playhead: "b",
  warnings: [{ cue: "Welcome", message: "Can't find welcome.wav. Relink it or skip." }],
  peaks: [0.5, null],
  underruns: 0,
  device: { name: "Speakers", sampleRate: 48000, channels: 2, audible: true, problem: null },
};

describe("ServicePanel", () => {
  it("shows the cues, what is next, and warnings", async () => {
    mockCommands({ serviceState: () => view });

    render(<ServicePanel />);

    expect(await screen.findByText("Walk-in")).toBeInTheDocument();
    expect(screen.getByText("Playing")).toBeInTheDocument();
    expect(screen.getByRole("listitem", { current: "step" })).toHaveTextContent("Welcome");
    expect(screen.getByText(/Can't find welcome.wav/)).toBeInTheDocument();
    expect(screen.getByText(/Speakers/)).toBeInTheDocument();
  });

  it("says plainly when there is nothing loaded", async () => {
    mockCommands({ serviceState: () => ({ ...view, title: "", cues: [], playhead: null }) });

    render(<ServicePanel />);

    expect(await screen.findByText(/No cues yet/)).toBeInTheDocument();
  });

  it("fires GO from the button and from the space bar, but not while typing", async () => {
    const go = vi.fn(() => null);
    mockCommands({ serviceState: () => view, go });
    render(<ServicePanel />);
    await screen.findByText("Walk-in");

    fireEvent.click(screen.getByRole("button", { name: /GO/ }));
    await waitFor(() => {
      expect(go).toHaveBeenCalledTimes(1);
    });

    fireEvent.keyDown(window, { code: "Space" });
    await waitFor(() => {
      expect(go).toHaveBeenCalledTimes(2);
    });

    fireEvent.keyDown(screen.getByRole("textbox"), { code: "Space", bubbles: true });
    expect(go).toHaveBeenCalledTimes(2);
  });

  it("panics on Escape", async () => {
    const panicStop = vi.fn(() => null);
    mockCommands({ serviceState: () => view, panicStop });
    render(<ServicePanel />);
    await screen.findByText("Walk-in");

    fireEvent.keyDown(window, { code: "Escape" });

    await waitFor(() => {
      expect(panicStop).toHaveBeenCalledTimes(1);
    });
  });

  it("loads the folder that was typed", async () => {
    const loadFolder = vi.fn(() => null);
    mockCommands({ serviceState: () => view, loadFolder });
    render(<ServicePanel />);

    fireEvent.change(screen.getByRole("textbox"), { target: { value: "C:\\Music\\Sunday" } });
    fireEvent.click(screen.getByRole("button", { name: "Load" }));

    await waitFor(() => {
      expect(loadFolder).toHaveBeenCalledWith({ path: "C:\\Music\\Sunday" });
    });
  });

  it("shows the core's message when something fails", async () => {
    mockCommands({
      serviceState: () => view,
      loadFolder: () => {
        // eslint-disable-next-line @typescript-eslint/only-throw-error -- mirrors what the core sends
        throw { kind: "invalidInput", message: "There are no audio files in that folder." };
      },
    });
    render(<ServicePanel />);

    fireEvent.change(screen.getByRole("textbox"), { target: { value: "x" } });
    fireEvent.click(screen.getByRole("button", { name: "Load" }));

    expect(await screen.findByRole("alert")).toHaveTextContent("There are no audio files");
  });
});
