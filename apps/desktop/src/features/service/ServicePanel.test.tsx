import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { mockCommands, type CueView_Serialize, type ServiceView_Serialize } from "@/lib/bindings";

import { ServicePanel } from "./ServicePanel";

function cue(
  overrides: Partial<CueView_Serialize> & { id: string; name: string },
): CueView_Serialize {
  return {
    number: "1",
    status: "idle",
    armed: true,
    notes: "",
    preWait: 0,
    postWait: 0,
    follow: "none",
    control: null,
    group: null,
    children: [],
    audio: {
      file: `media/${overrides.name}.wav`,
      level: 0,
      start: 0,
      end: null,
      loops: null,
      fadeIn: 0,
      fadeOut: 0,
      curve: "equalPower",
    },
    ...overrides,
  };
}

const view: ServiceView_Serialize = {
  title: "Sunday",
  cues: [
    cue({ id: "a", number: "1", name: "Walk-in", status: "running" }),
    cue({ id: "b", number: "2", name: "Welcome" }),
    cue({ id: "c", number: "3", name: "Hymn" }),
  ],
  playhead: "b",
  warnings: [{ cue: "Welcome", message: "Can't find welcome.wav. Relink it or skip." }],
  peaks: [0.5, null],
  underruns: 0,
  device: { name: "Speakers", sampleRate: 48000, channels: 2, audible: true, problem: null },
  location: "C:\\Shows\\Sunday",
  saveProblem: null,
  notice: null,
  mediaRoot: "C:\\Shows\\Sunday",
  editing: true,
  canUndo: true,
  canRedo: false,
};

const ok = () => null;

async function select(name: string) {
  fireEvent.click(await screen.findByText(name));
}

describe("operating the service", () => {
  it("shows the cues, what is next, and warnings", async () => {
    mockCommands({ serviceState: () => view });

    render(<ServicePanel />);

    expect(await screen.findByText("Walk-in")).toBeInTheDocument();
    expect(screen.getByText("Playing")).toBeInTheDocument();
    expect(screen.getByRole("listitem", { current: "step" })).toHaveTextContent("Welcome");
    expect(screen.getByText(/Can't find welcome.wav/)).toBeInTheDocument();
    expect(screen.getByText(/Speakers/)).toBeInTheDocument();
  });

  it("fires GO from the button and from the space bar, but not while typing", async () => {
    const go = vi.fn(ok);
    mockCommands({ serviceState: () => view, go });
    render(<ServicePanel />);
    await screen.findByText("Walk-in");
    await select("Welcome");

    fireEvent.click(screen.getByRole("button", { name: /GO/ }));
    await waitFor(() => {
      expect(go).toHaveBeenCalledTimes(1);
    });
    fireEvent.keyDown(window, { code: "Space" });
    await waitFor(() => {
      expect(go).toHaveBeenCalledTimes(2);
    });

    fireEvent.keyDown(screen.getByRole("textbox", { name: "Name" }), { code: "Space" });
    expect(go).toHaveBeenCalledTimes(2);
  });

  it("panics on Escape", async () => {
    const panicStop = vi.fn(ok);
    mockCommands({ serviceState: () => view, panicStop });
    render(<ServicePanel />);
    await screen.findByText("Walk-in");

    fireEvent.keyDown(window, { code: "Escape" });

    await waitFor(() => {
      expect(panicStop).toHaveBeenCalledTimes(1);
    });
  });

  it("says plainly when there is no show yet", async () => {
    mockCommands({
      serviceState: () => ({
        ...view,
        title: "",
        cues: [],
        playhead: null,
        mediaRoot: null,
        location: null,
      }),
    });
    render(<ServicePanel />);
    expect(await screen.findByText(/No show yet/)).toBeInTheDocument();
  });

  it("shows the core's message when something fails, and keeps it on screen", async () => {
    mockCommands({
      serviceState: () => view,
      saveShow: () => {
        // eslint-disable-next-line @typescript-eslint/only-throw-error -- mirrors what the core sends
        throw { kind: "invalidInput", message: "Can't save to C:\\Shows. Check the disk." };
      },
    });
    render(<ServicePanel />);
    await screen.findByText("Walk-in");

    fireEvent.click(screen.getByRole("button", { name: "Save show" }));

    expect(await screen.findByRole("alert")).toHaveTextContent("Can't save to");
    await new Promise((resolve) => setTimeout(resolve, 250));
    expect(screen.getByRole("alert")).toHaveTextContent("Can't save to");
  });
});

describe("files and folders", () => {
  it("opens a show from the folder the operator picks", async () => {
    const openShow = vi.fn(ok);
    mockCommands({
      serviceState: () => view,
      pickFolder: () => "C:\\Shows\\Sunday",
      openShow,
    });
    render(<ServicePanel />);
    await screen.findByText("Walk-in");

    fireEvent.click(screen.getByRole("button", { name: "Open show" }));

    await waitFor(() => {
      expect(openShow).toHaveBeenCalledWith({ path: "C:\\Shows\\Sunday" });
    });
  });

  it("creates a new show in the folder picked, and does nothing if they cancel", async () => {
    const newShow = vi.fn(ok);
    const pickFolder = vi.fn<() => string | null>(() => "C:\\Shows\\New");
    mockCommands({ serviceState: () => view, pickFolder, newShow });
    render(<ServicePanel />);
    await screen.findByText("Walk-in");

    fireEvent.click(screen.getByRole("button", { name: "New show" }));
    await waitFor(() => {
      expect(newShow).toHaveBeenCalledWith({ path: "C:\\Shows\\New" });
    });

    pickFolder.mockReturnValue(null);
    fireEvent.click(screen.getByRole("button", { name: "New show" }));
    await waitFor(() => {
      expect(pickFolder).toHaveBeenCalledTimes(2);
    });
    expect(newShow).toHaveBeenCalledTimes(1);
  });

  it("says where the show is saved, and lets the operator save", async () => {
    const saveShow = vi.fn(ok);
    mockCommands({ serviceState: () => view, saveShow });
    render(<ServicePanel />);

    expect(await screen.findByText(/Saved in C:\\Shows\\Sunday/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Save show" }));

    await waitFor(() => {
      expect(saveShow).toHaveBeenCalledTimes(1);
    });
  });

  it("says when the show is unsaved, and shows save problems and notices", async () => {
    mockCommands({
      serviceState: () => ({
        ...view,
        location: null,
        saveProblem: "Can't save to C:\\Shows. Check that the disk has room.",
        notice: "The show file was damaged, so the latest backup was opened.",
      }),
    });
    render(<ServicePanel />);

    expect(await screen.findByText(/Not saved yet/)).toBeInTheDocument();
    expect(screen.getByRole("alert")).toHaveTextContent("Can't save to");
    expect(screen.getByRole("status")).toHaveTextContent("latest backup was opened");
  });
});

describe("editing cues", () => {
  it("shows the settings of the selected cue", async () => {
    mockCommands({ serviceState: () => view });
    render(<ServicePanel />);

    await select("Welcome");

    const inspector = screen.getByRole("complementary", { name: "Settings for Welcome" });
    expect(within(inspector).getByRole("textbox", { name: "Name" })).toHaveValue("Welcome");
    expect(within(inspector).getByText("media/Welcome.wav")).toBeInTheDocument();
  });

  it("renames a cue when the operator leaves the box", async () => {
    const editCue = vi.fn(ok);
    mockCommands({ serviceState: () => view, editCue });
    render(<ServicePanel />);
    await select("Welcome");

    const name = screen.getByRole("textbox", { name: "Name" });
    fireEvent.change(name, { target: { value: "Greeting" } });
    fireEvent.blur(name);

    await waitFor(() => {
      expect(editCue).toHaveBeenCalledWith({
        cue: "b",
        change: { type: "name", value: "Greeting" },
      });
    });
  });

  it("sends numbers, and an empty optional box means none", async () => {
    const editCue = vi.fn(ok);
    mockCommands({ serviceState: () => view, editCue });
    render(<ServicePanel />);
    await select("Welcome");

    const fadeIn = screen.getByRole("textbox", { name: "Fade in (seconds)" });
    fireEvent.change(fadeIn, { target: { value: "2.5" } });
    fireEvent.blur(fadeIn);
    await waitFor(() => {
      expect(editCue).toHaveBeenCalledWith({ cue: "b", change: { type: "fadeIn", value: 2.5 } });
    });

    const stop = screen.getByRole("textbox", { name: "Stop at (seconds)" });
    fireEvent.change(stop, { target: { value: "30" } });
    fireEvent.blur(stop);
    await waitFor(() => {
      expect(editCue).toHaveBeenCalledWith({ cue: "b", change: { type: "end", value: 30 } });
    });
  });

  it("does not send text that is not a number", async () => {
    const editCue = vi.fn(ok);
    mockCommands({ serviceState: () => view, editCue });
    render(<ServicePanel />);
    await select("Welcome");

    const fadeIn = screen.getByRole("textbox", { name: "Fade in (seconds)" });
    fireEvent.change(fadeIn, { target: { value: "soon" } });
    fireEvent.blur(fadeIn);

    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(editCue).not.toHaveBeenCalled();
  });

  it("changes the follow mode and shows the next-cue wait only when it matters", async () => {
    const editCue = vi.fn(ok);
    mockCommands({ serviceState: () => view, editCue });
    render(<ServicePanel />);
    await select("Welcome");

    expect(screen.queryByRole("textbox", { name: /Wait before the next cue/ })).toBeNull();
    fireEvent.change(screen.getByRole("combobox", { name: "After this cue starts" }), {
      target: { value: "continue" },
    });

    await waitFor(() => {
      expect(editCue).toHaveBeenCalledWith({
        cue: "b",
        change: { type: "follow", value: "continue" },
      });
    });
  });

  it("sends level changes from the slider", async () => {
    const editCue = vi.fn(ok);
    mockCommands({ serviceState: () => view, editCue });
    render(<ServicePanel />);
    await select("Welcome");

    fireEvent.change(screen.getByRole("slider", { name: "Level" }), { target: { value: "-6" } });

    await waitFor(() => {
      expect(editCue).toHaveBeenCalledWith({ cue: "b", change: { type: "level", value: -6 } });
    });
  });

  it("arms and disarms from the inspector", async () => {
    const editCue = vi.fn(ok);
    mockCommands({ serviceState: () => view, editCue });
    render(<ServicePanel />);
    await select("Welcome");

    fireEvent.click(screen.getByRole("checkbox", { name: /Armed/ }));

    await waitFor(() => {
      expect(editCue).toHaveBeenCalledWith({ cue: "b", change: { type: "armed", value: false } });
    });
  });

  it("adds audio after the selected cue and selects what was added", async () => {
    const addAudio = vi.fn(() => ["new"]);
    mockCommands({
      serviceState: () => ({
        ...view,
        cues: [...view.cues, cue({ id: "new", number: "4", name: "Added" })],
      }),
      pickAudioFiles: () => ["C:\\Music\\a.wav"],
      addAudio,
    });
    render(<ServicePanel />);
    await select("Welcome");

    fireEvent.click(screen.getByRole("button", { name: "Add audio" }));

    await waitFor(() => {
      expect(addAudio).toHaveBeenCalledWith({ paths: ["C:\\Music\\a.wav"], before: "c" });
    });
    expect(
      await screen.findByRole("complementary", { name: "Settings for Added" }),
    ).toBeInTheDocument();
  });

  it("does nothing when the file picker is cancelled", async () => {
    const addAudio = vi.fn(() => []);
    mockCommands({ serviceState: () => view, pickAudioFiles: () => [], addAudio });
    render(<ServicePanel />);
    await screen.findByText("Walk-in");

    fireEvent.click(screen.getByRole("button", { name: "Add audio" }));

    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(addAudio).not.toHaveBeenCalled();
  });

  it("deletes, moves, and duplicates the selected cue", async () => {
    const deleteCues = vi.fn(ok);
    const moveCue = vi.fn(ok);
    // No new id, so the selection stays on "b" in this fixed fake view.
    const duplicateCue = vi.fn((): string[] => []);
    mockCommands({ serviceState: () => view, deleteCues, moveCue, duplicateCue });
    render(<ServicePanel />);
    await select("Welcome");

    fireEvent.click(screen.getByRole("button", { name: "Move up" }));
    await waitFor(() => {
      expect(moveCue).toHaveBeenCalledWith({ cue: "b", before: "a" });
    });

    fireEvent.click(screen.getByRole("button", { name: "Move down" }));
    await waitFor(() => {
      expect(moveCue).toHaveBeenCalledWith({ cue: "b", before: null });
    });

    fireEvent.click(screen.getByRole("button", { name: "Duplicate" }));
    await waitFor(() => {
      expect(duplicateCue).toHaveBeenCalledWith({ cue: "b" });
    });

    fireEvent.click(screen.getByRole("button", { name: "Delete" }));
    await waitFor(() => {
      expect(deleteCues).toHaveBeenCalledWith({ cues: ["b"] });
    });
  });

  it("uses the keyboard to delete, move, undo and redo", async () => {
    const deleteCues = vi.fn(ok);
    const moveCue = vi.fn(ok);
    const undo = vi.fn(ok);
    const redo = vi.fn(ok);
    mockCommands({
      serviceState: () => ({ ...view, canRedo: true }),
      deleteCues,
      moveCue,
      undo,
      redo,
    });
    render(<ServicePanel />);
    await select("Welcome");

    fireEvent.keyDown(window, { code: "Delete" });
    await waitFor(() => {
      expect(deleteCues).toHaveBeenCalledWith({ cues: ["b"] });
    });
    fireEvent.keyDown(window, { code: "ArrowUp", altKey: true });
    await waitFor(() => {
      expect(moveCue).toHaveBeenCalledWith({ cue: "b", before: "a" });
    });
    fireEvent.keyDown(window, { code: "KeyZ", ctrlKey: true });
    await waitFor(() => {
      expect(undo).toHaveBeenCalledTimes(1);
    });
    fireEvent.keyDown(window, { code: "KeyY", ctrlKey: true });
    await waitFor(() => {
      expect(redo).toHaveBeenCalledTimes(1);
    });
  });

  it("does not delete a cue while the operator is typing in a field", async () => {
    const deleteCues = vi.fn(ok);
    mockCommands({ serviceState: () => view, deleteCues });
    render(<ServicePanel />);
    await select("Welcome");

    fireEvent.keyDown(screen.getByRole("textbox", { name: "Name" }), { code: "Delete" });

    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(deleteCues).not.toHaveBeenCalled();
  });

  it("disables undo and redo when there is nothing to undo or redo", async () => {
    mockCommands({ serviceState: () => ({ ...view, canUndo: false, canRedo: false }) });
    render(<ServicePanel />);
    await screen.findByText("Walk-in");

    expect(screen.getByRole("button", { name: "Undo" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Redo" })).toBeDisabled();
  });

  it("shows the core's reason when an edit is refused", async () => {
    mockCommands({
      serviceState: () => view,
      editCue: () => {
        // eslint-disable-next-line @typescript-eslint/only-throw-error -- mirrors what the core sends
        throw { kind: "invalidInput", message: "Times must be between 0 and 10 hours." };
      },
    });
    render(<ServicePanel />);
    await select("Welcome");

    const wait = screen.getByRole("textbox", { name: "Wait before starting (seconds)" });
    fireEvent.change(wait, { target: { value: "-5" } });
    fireEvent.blur(wait);

    expect(await screen.findByRole("alert")).toHaveTextContent("Times must be between");
  });
});

describe("show mode", () => {
  it("switches mode from the button and from Ctrl+E", async () => {
    const setEditing = vi.fn(ok);
    mockCommands({ serviceState: () => view, setEditing });
    render(<ServicePanel />);
    await screen.findByText("Walk-in");

    fireEvent.click(screen.getByRole("button", { name: /Edit mode/ }));
    await waitFor(() => {
      expect(setEditing).toHaveBeenCalledWith({ editing: false });
    });

    fireEvent.keyDown(window, { code: "KeyE", ctrlKey: true });
    await waitFor(() => {
      expect(setEditing).toHaveBeenCalledTimes(2);
    });
  });

  it("hides every editing control in show mode and keeps GO and Panic", async () => {
    mockCommands({ serviceState: () => ({ ...view, editing: false }) });
    render(<ServicePanel />);
    await screen.findByText("Walk-in");

    expect(screen.getByRole("button", { name: /Show mode/ })).toBeInTheDocument();
    expect(screen.queryByRole("toolbar", { name: "Edit cues" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Undo" })).toBeDisabled();
    fireEvent.click(screen.getByText("Welcome"));
    expect(screen.queryByRole("complementary")).toBeNull();
    expect(screen.getByRole("button", { name: /GO/ })).toBeEnabled();
    expect(screen.getByRole("button", { name: /Panic/ })).toBeEnabled();
  });

  it("ignores editing shortcuts in show mode", async () => {
    const deleteCues = vi.fn(ok);
    const undo = vi.fn(ok);
    mockCommands({ serviceState: () => ({ ...view, editing: false }), deleteCues, undo });
    render(<ServicePanel />);
    await screen.findByText("Walk-in");

    fireEvent.keyDown(window, { code: "Delete" });
    fireEvent.keyDown(window, { code: "KeyZ", ctrlKey: true });

    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(deleteCues).not.toHaveBeenCalled();
    expect(undo).not.toHaveBeenCalled();
  });
});

function plain(id: string, name: string, number: string): CueView_Serialize {
  return cue({ id, name, number });
}

function groupCue(
  id: string,
  children: CueView_Serialize[],
  mode: "together" | "sequence" = "together",
): CueView_Serialize {
  return {
    ...cue({ id, name: "Worship set", number: "2" }),
    audio: null,
    group: { mode, looped: false },
    children,
  };
}

function controlCue(
  id: string,
  kind: "fade" | "stop" | "pause" | "goTo" | "arm",
  target: string,
  targetName: string | null,
): CueView_Serialize {
  return {
    ...cue({ id, name: `Control ${id}`, number: "9" }),
    audio: null,
    control: {
      kind,
      target,
      targetName,
      to: -60,
      duration: 3,
      curve: "equalPower",
      stopWhenDone: true,
      arms: true,
    },
  };
}

const nested: ServiceView_Serialize = {
  ...view,
  cues: [
    plain("a", "Walk-in", "1"),
    groupCue("g", [plain("x", "Song one", "2.1"), plain("y", "Song two", "2.2")]),
    plain("b", "Welcome", "3"),
  ],
  playhead: "a",
  warnings: [],
};

describe("groups", () => {
  it("shows the cues inside a group, indented, with how the group starts them", async () => {
    mockCommands({ serviceState: () => nested });
    render(<ServicePanel />);

    expect(await screen.findByText("Song one")).toBeInTheDocument();
    expect(screen.getByText(/Group: all at once/)).toBeInTheDocument();
    const child = screen.getByText("Song one").closest("li");
    expect(child).toHaveStyle({ marginLeft: "28px" });
    expect(screen.getByText("Welcome").closest("li")).toHaveStyle({ marginLeft: "0px" });
  });

  it("selects several cues with Ctrl and Shift, and groups them", async () => {
    const groupCues = vi.fn(() => ["g2"]);
    mockCommands({ serviceState: () => nested, groupCues });
    render(<ServicePanel />);

    await select("Walk-in");
    fireEvent.click(screen.getByText("Welcome"), { ctrlKey: true });
    expect(screen.getByRole("complementary", { name: "Selection" })).toHaveTextContent(
      "2 cues selected",
    );
    expect(screen.queryByRole("complementary", { name: /Settings for/ })).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "Group" }));

    await waitFor(() => {
      expect(groupCues).toHaveBeenCalledWith({ cues: ["a", "b"] });
    });
  });

  it("selects a range with Shift", async () => {
    const deleteCues = vi.fn(ok);
    mockCommands({ serviceState: () => nested, deleteCues });
    render(<ServicePanel />);

    await select("Walk-in");
    fireEvent.click(screen.getByText("Song one"), { shiftKey: true });
    fireEvent.click(screen.getByRole("button", { name: "Delete" }));

    await waitFor(() => {
      expect(deleteCues).toHaveBeenCalledWith({ cues: ["a", "g", "x"] });
    });
  });

  it("cannot group a cue that is already inside a group, but can ungroup a group", async () => {
    const ungroup = vi.fn(ok);
    mockCommands({ serviceState: () => nested, ungroup });
    render(<ServicePanel />);

    await select("Song one");
    expect(screen.getByRole("button", { name: "Group" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Ungroup" })).toBeDisabled();

    fireEvent.click(screen.getByText("Worship set"));
    expect(screen.getByRole("button", { name: "Ungroup" })).toBeEnabled();
    fireEvent.click(screen.getByRole("button", { name: "Ungroup" }));
    await waitFor(() => {
      expect(ungroup).toHaveBeenCalledWith({ cue: "g" });
    });
  });

  it("moves a cue within its group, and stops at the group's edges", async () => {
    const moveCue = vi.fn(ok);
    mockCommands({ serviceState: () => nested, moveCue });
    render(<ServicePanel />);

    await select("Song one");
    expect(screen.getByRole("button", { name: "Move up" })).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "Move down" }));
    await waitFor(() => {
      expect(moveCue).toHaveBeenCalledWith({ cue: "x", before: null });
    });
    // "Song two" is last in the group, so it cannot go further down.
    fireEvent.click(screen.getByText("Song two"));
    expect(screen.getByRole("button", { name: "Move down" })).toBeDisabled();
  });

  it("edits how a group starts its cues", async () => {
    const editCue = vi.fn(ok);
    mockCommands({ serviceState: () => nested, editCue });
    render(<ServicePanel />);
    await select("Worship set");

    fireEvent.change(screen.getByRole("combobox", { name: "Start the cues inside" }), {
      target: { value: "sequence" },
    });
    await waitFor(() => {
      expect(editCue).toHaveBeenCalledWith({
        cue: "g",
        change: { type: "groupMode", value: "sequence" },
      });
    });
    fireEvent.click(screen.getByRole("checkbox", { name: /Start again when finished/ }));
    await waitFor(() => {
      expect(editCue).toHaveBeenCalledWith({ cue: "g", change: { type: "looped", value: true } });
    });
  });

  it("adds new cues after the selected one even when it is last in a group", async () => {
    const addMemo = vi.fn(() => []);
    mockCommands({ serviceState: () => nested, addMemo });
    render(<ServicePanel />);
    await select("Song two");

    fireEvent.click(screen.getByRole("button", { name: "Add note" }));

    await waitFor(() => {
      expect(addMemo).toHaveBeenCalledWith({ before: "b" });
    });
  });
});

describe("control cues", () => {
  const withControl = (control: CueView_Serialize): ServiceView_Serialize => ({
    ...nested,
    cues: [...nested.cues, control],
  });

  it("adds a control cue that acts on the selected cue", async () => {
    const addControl = vi.fn(() => ["new"]);
    mockCommands({ serviceState: () => nested, addControl });
    render(<ServicePanel />);
    await select("Walk-in");

    fireEvent.change(screen.getByRole("combobox", { name: "Add control cue" }), {
      target: { value: "stop" },
    });

    await waitFor(() => {
      expect(addControl).toHaveBeenCalledWith({ kind: "stop", target: "a", before: "g" });
    });
  });

  it("needs a selected cue to add a control cue", async () => {
    mockCommands({ serviceState: () => nested });
    render(<ServicePanel />);
    await screen.findByText("Walk-in");
    expect(screen.getByRole("combobox", { name: "Add control cue" })).toBeDisabled();
  });

  it("says what a control cue does in the list", async () => {
    mockCommands({ serviceState: () => withControl(controlCue("f", "fade", "a", "Walk-in")) });
    render(<ServicePanel />);

    expect(await screen.findByText("Fades Walk-in")).toBeInTheDocument();
  });

  it("warns, in the list and the inspector, when the target was deleted", async () => {
    mockCommands({ serviceState: () => withControl(controlCue("f", "stop", "gone", null)) });
    render(<ServicePanel />);

    expect(await screen.findByText(/The cue this acts on was deleted$/)).toBeInTheDocument();
    fireEvent.click(screen.getByText("Control f"));
    expect(screen.getByRole("status")).toHaveTextContent("so it does nothing");
  });

  it("changes the target from a list of the other cues", async () => {
    const editCue = vi.fn(ok);
    mockCommands({
      serviceState: () => withControl(controlCue("f", "stop", "a", "Walk-in")),
      editCue,
    });
    render(<ServicePanel />);
    await select("Control f");

    const acts = screen.getByRole("combobox", { name: "Acts on" });
    const options = within(acts)
      .getAllByRole("option")
      .map((o) => o.textContent);
    expect(options).toContain("3. Welcome");
    expect(options).not.toContain("9. Control f");

    fireEvent.change(acts, { target: { value: "b" } });
    await waitFor(() => {
      expect(editCue).toHaveBeenCalledWith({ cue: "f", change: { type: "target", value: "b" } });
    });
  });

  it("only offers cues in the main list as the target of a go-to", async () => {
    mockCommands({ serviceState: () => withControl(controlCue("f", "goTo", "a", "Walk-in")) });
    render(<ServicePanel />);
    await select("Control f");

    const options = within(screen.getByRole("combobox", { name: "Acts on" }))
      .getAllByRole("option")
      .map((o) => o.textContent);
    expect(options).toContain("3. Welcome");
    expect(options).not.toContain("2.1. Song one");
  });

  it("edits the settings of a fade cue", async () => {
    const editCue = vi.fn(ok);
    mockCommands({
      serviceState: () => withControl(controlCue("f", "fade", "a", "Walk-in")),
      editCue,
    });
    render(<ServicePanel />);
    await select("Control f");

    const to = screen.getByRole("textbox", { name: /Fade to/ });
    fireEvent.change(to, { target: { value: "-20" } });
    fireEvent.blur(to);
    await waitFor(() => {
      expect(editCue).toHaveBeenCalledWith({ cue: "f", change: { type: "fadeTo", value: -20 } });
    });

    const time = screen.getByRole("textbox", { name: "Fade time (seconds)" });
    fireEvent.change(time, { target: { value: "8" } });
    fireEvent.blur(time);
    await waitFor(() => {
      expect(editCue).toHaveBeenCalledWith({
        cue: "f",
        change: { type: "fadeDuration", value: 8 },
      });
    });

    fireEvent.click(screen.getByRole("checkbox", { name: /Stop it when the fade ends/ }));
    await waitFor(() => {
      expect(editCue).toHaveBeenCalledWith({
        cue: "f",
        change: { type: "stopWhenDone", value: false },
      });
    });
  });

  it("switches an arm cue between arming and disarming", async () => {
    const editCue = vi.fn(ok);
    mockCommands({
      serviceState: () => withControl(controlCue("f", "arm", "a", "Walk-in")),
      editCue,
    });
    render(<ServicePanel />);
    await select("Control f");

    fireEvent.change(screen.getByRole("combobox", { name: "Action" }), {
      target: { value: "disarm" },
    });

    await waitFor(() => {
      expect(editCue).toHaveBeenCalledWith({
        cue: "f",
        change: { type: "armAction", value: false },
      });
    });
  });

  it("shows no audio settings for a control cue", async () => {
    mockCommands({ serviceState: () => withControl(controlCue("f", "stop", "a", "Walk-in")) });
    render(<ServicePanel />);
    await select("Control f");

    expect(screen.queryByRole("slider", { name: "Level" })).toBeNull();
    expect(screen.queryByRole("textbox", { name: /Fade in/ })).toBeNull();
  });
});
