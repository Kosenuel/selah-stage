/**
 * The root of an output window (main, stage, or lower third).
 *
 * Output windows are plain renderers: they draw the render state the core
 * sends and hold no show logic, so one can be reloaded mid-service without
 * affecting anything else. Until that state exists, an output shows what a
 * cleared output must always show: black.
 */
export function OutputRoot() {
  return <div className="h-full w-full bg-output-backdrop" data-testid="output-root" />;
}
