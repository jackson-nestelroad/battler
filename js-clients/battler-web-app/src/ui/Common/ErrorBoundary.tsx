import { Component, type ErrorInfo, type ReactNode } from "react";
import BugReportModal from "./BugReportModal/BugReportModal";

interface Props {
  children: ReactNode;
  fallback?: ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
  errorInfo: ErrorInfo | null;
  showBugReportModal: boolean;
}

export default class ErrorBoundary extends Component<Props, State> {
  public state: State = {
    hasError: false,
    error: null,
    errorInfo: null,
    showBugReportModal: false,
  };

  public static getDerivedStateFromError(error: Error): Partial<State> {
    return { hasError: true, error };
  }

  public componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error("[React ErrorBoundary caught error]:", error, errorInfo);
    this.setState({ errorInfo });
  }

  public render() {
    if (this.state.hasError) {
      if (this.props.fallback) {
        return this.props.fallback;
      }

      return (
        <div className="page-container flex-col align-center gap-m text-center">
          <div className="alert alert-danger w-full">
            <div className="flex-col align-start gap-xs text-left">
              <h4>Something went wrong</h4>
              <p>{this.state.error?.message || "An unexpected UI error occurred."}</p>
            </div>
          </div>

          <div className="flex-row gap-s align-center flex-wrap justify-center">
            <button
              className="btn btn-primary"
              onClick={() => {
                this.setState({ hasError: false, error: null, errorInfo: null, showBugReportModal: false });
                window.location.reload();
              }}
            >
              Reload application
            </button>
            <button
              type="button"
              className="btn btn-secondary"
              onClick={() => this.setState({ showBugReportModal: true })}
            >
              Report error
            </button>
          </div>

          {this.state.showBugReportModal && (
            <BugReportModal
              isOpen={this.state.showBugReportModal}
              onClose={() => this.setState({ showBugReportModal: false })}
              reactCrash={
                this.state.error
                  ? {
                      message: this.state.error.message,
                      stack: this.state.error.stack,
                      componentStack: this.state.errorInfo?.componentStack ?? undefined,
                    }
                  : undefined
              }
            />
          )}
        </div>
      );
    }

    return this.props.children;
  }
}
