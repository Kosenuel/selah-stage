import { Component, type ErrorInfo, type ReactNode } from "react";

interface Props {
  /** Names the part of the screen, for the message and the log. */
  label: string;
  children: ReactNode;
}

interface State {
  failed: boolean;
}

/**
 * Keeps a fault in one part of the screen from blanking the rest. The show
 * itself is run by the core, so it carries on either way.
 */
export class ErrorBoundary extends Component<Props, State> {
  override state: State = { failed: false };

  static getDerivedStateFromError(): State {
    return { failed: true };
  }

  override componentDidCatch(error: Error, info: ErrorInfo) {
    console.error(`The ${this.props.label} failed to draw:`, error, info.componentStack);
  }

  override render() {
    if (!this.state.failed) {
      return this.props.children;
    }
    return (
      <div
        role="alert"
        className="flex items-center gap-3 rounded-lg border border-danger px-4 py-3 text-sm text-danger"
      >
        <span className="flex-1">
          The {this.props.label} had a problem and was hidden. The show is still running.
        </span>
        <button
          type="button"
          className="rounded-lg border border-danger px-3 py-1"
          onClick={() => {
            this.setState({ failed: false });
          }}
        >
          Try again
        </button>
      </div>
    );
  }
}
