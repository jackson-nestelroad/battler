import * as React from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import TypeChartModal from "./TypeChartModal";
import * as typeChartHook from "../../hooks/useTypeChart";

vi.mock("react-dom", () => ({
  createPortal: (children: React.ReactNode) => children,
}));

describe("TypeChartModal", () => {
  let originalDocument: Document;

  beforeEach(() => {
    originalDocument = globalThis.document;
    globalThis.document = { body: { nodeType: 1 } } as unknown as Document;
    vi.spyOn(typeChartHook, "useTypeChart").mockReturnValue({
      typeChart: { types: {} },
      loading: false,
      error: null,
    });
  });

  afterEach(() => {
    globalThis.document = originalDocument;
    vi.restoreAllMocks();
  });

  it("renders modal with TypeChartGrid when isOpen is true", () => {
    const html = renderToStaticMarkup(
      <TypeChartModal isOpen={true} onClose={vi.fn()} />,
    );

    expect(html).toContain("Type Chart");
    expect(html).toContain("modalLg");
    expect(html).toContain('role="dialog"');
    expect(html).toContain("DEF");
    expect(html).toContain("ATK");
  });

  it("does not render when isOpen is false", () => {
    const html = renderToStaticMarkup(
      <TypeChartModal isOpen={false} onClose={vi.fn()} />,
    );

    expect(html).toBe("");
  });
});
