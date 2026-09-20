import * as React from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { Provider } from "react-redux";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { store } from "../../../store/store";
import BugReportModal from "./BugReportModal";

vi.mock("react-dom", () => ({
  createPortal: (children: React.ReactNode) => children,
}));

describe("BugReportModal", () => {
  let originalDocument: Document;

  beforeEach(() => {
    originalDocument = globalThis.document;
    globalThis.document = { body: { nodeType: 1 } } as unknown as Document;
  });

  afterEach(() => {
    globalThis.document = originalDocument;
  });

  it("does not render when isOpen is false", () => {
    const html = renderToStaticMarkup(
      <Provider store={store}>
        <BugReportModal isOpen={false} onClose={vi.fn()} />
      </Provider>,
    );

    expect(html).toBe("");
  });

  it("renders form fields and diagnostics when isOpen is true", () => {
    const html = renderToStaticMarkup(
      <Provider store={store}>
        <BugReportModal isOpen={true} onClose={vi.fn()} />
      </Provider>,
    );

    expect(html).toContain("Report Bug");
    expect(html).toContain("bug-title");
    expect(html).toContain("bug-description");
    expect(html).toContain("Context");
    expect(html).toContain("Submit");
    expect(html).toContain("Export");
  });

  it("pre-populates title with crash message when reactCrash is provided", () => {
    const mockCrash = {
      message: "Render crashed unexpectedly",
      stack: "Error at render()",
    };

    const html = renderToStaticMarkup(
      <Provider store={store}>
        <BugReportModal isOpen={true} onClose={vi.fn()} reactCrash={mockCrash} />
      </Provider>,
    );

    expect(html).toContain("Crash: Render crashed unexpectedly");
    expect(html).toContain("Context");
  });
});
