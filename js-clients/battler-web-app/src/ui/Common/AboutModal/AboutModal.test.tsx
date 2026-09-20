import * as React from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import AboutModal from "./AboutModal";

vi.mock("react-dom", () => ({
  createPortal: (children: React.ReactNode) => children,
}));

describe("AboutModal", () => {
  let originalDocument: Document;

  beforeEach(() => {
    originalDocument = globalThis.document;
    globalThis.document = { body: { nodeType: 1 } } as unknown as Document;
  });

  afterEach(() => {
    globalThis.document = originalDocument;
  });

  it("renders when isOpen is true with battler.rs attribution, tech icons, and disclaimer", () => {
    const html = renderToStaticMarkup(
      <AboutModal isOpen={true} onClose={vi.fn()} />,
    );

    expect(html).toContain("About");
    expect(html).toContain("Battler");
    expect(html).toContain("Pokémon battle engine");
    expect(html).toContain("Powered by");
    expect(html).toContain("battler.rs");
    expect(html).toContain("https://battler.rs");
    expect(html).toContain("Rust");
    expect(html).toContain("WebAssembly");
    expect(html).toContain("React");
    expect(html).toContain("Vite");
    expect(html).toContain(".NET");
    expect(html).toContain("Google Gemini");
    const currentYear = new Date().getFullYear();
    expect(html).toContain(`© ${currentYear} Jackson Nestelroad`);
    expect(html).toContain(
      `Pokémon is © 1995–${currentYear} Nintendo, Creatures, and Game Freak. Battler is an unofficial fan project and is not affiliated with Nintendo or The Pokémon Company.`,
    );
    expect(html).not.toContain("MIT License");
    expect(html).not.toContain("v0.1.0");
    expect(html).toContain('src="/assets/icons/rust.svg"');
    expect(html).toContain('src="/assets/icons/webassembly.svg"');
    expect(html).toContain('src="/assets/icons/react.svg"');
    expect(html).toContain('src="/assets/icons/vite.svg"');
    expect(html).toContain('src="/assets/icons/dotnet.svg"');
    expect(html).toContain('src="/assets/icons/googlegemini.svg"');
    expect(html).toContain('src="/assets/icons/github.svg"');
  });

  it("does not render when isOpen is false", () => {
    const html = renderToStaticMarkup(
      <AboutModal isOpen={false} onClose={vi.fn()} />,
    );

    expect(html).toBe("");
  });
});
