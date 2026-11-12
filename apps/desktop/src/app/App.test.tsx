import { render, screen } from "@testing-library/react";
import { expect, it } from "vitest";

import { mockCommands } from "@/lib/bindings";

import { App } from "./App";

it("renders the control window shell", () => {
  mockCommands({ appInfo: () => new Promise(() => undefined) });

  render(<App />);

  expect(screen.getByRole("heading", { level: 1, name: "Selah Stage" })).toBeInTheDocument();
  expect(screen.getByRole("main")).toBeInTheDocument();
});
