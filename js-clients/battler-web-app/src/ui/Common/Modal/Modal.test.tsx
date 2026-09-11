import * as React from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import Modal from "./Modal";

vi.mock("react-dom", () => ({
  createPortal: (children: React.ReactNode) => children,
}));

describe("Modal", () => {
  let originalDocument: Document;

  beforeEach(() => {
    originalDocument = globalThis.document;
    globalThis.document = { body: { nodeType: 1 } } as unknown as Document;
  });

  afterEach(() => {
    globalThis.document = originalDocument;
  });

  it("renders when isOpen is true", () => {
    const html = renderToStaticMarkup(
      <Modal
        isOpen={true}
        onClose={vi.fn()}
        title="Test Modal"
        subtitle="Test Subtitle"
        headerActions={<button type="button">Action</button>}
      >
        <div>Modal Body Content</div>
      </Modal>,
    );

    expect(html).toContain("Test Modal");
    expect(html).toContain("Test Subtitle");
    expect(html).toContain("Action");
    expect(html).toContain("Modal Body Content");
    expect(html).toContain('role="dialog"');
    expect(html).toContain('aria-modal="true"');
    expect(html).toContain("modalMd");
  });

  it("does not render when isOpen is false", () => {
    const html = renderToStaticMarkup(
      <Modal isOpen={false} onClose={vi.fn()} title="Hidden Modal">
        <div>Hidden Content</div>
      </Modal>,
    );

    expect(html).toBe("");
  });

  it("applies maxWidth modifier classes correctly", () => {
    const htmlLg = renderToStaticMarkup(
      <Modal isOpen={true} onClose={vi.fn()} title="Large Modal" maxWidth="lg">
        <div>Content</div>
      </Modal>,
    );
    expect(htmlLg).toContain("modalLg");

    const htmlSm = renderToStaticMarkup(
      <Modal isOpen={true} onClose={vi.fn()} title="Small Modal" maxWidth="sm">
        <div>Content</div>
      </Modal>,
    );
    expect(htmlSm).toContain("modalSm");
  });
});
