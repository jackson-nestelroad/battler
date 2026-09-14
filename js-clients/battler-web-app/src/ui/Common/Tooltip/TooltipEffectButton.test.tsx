import * as React from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import TooltipEffectButton from "./TooltipEffectButton";
import { FxLangModalContext } from "../FxLang/FxLangModalContext";

describe("TooltipEffectButton", () => {
  it("renders with Effect text and accessible label", () => {
    const html = renderToStaticMarkup(
      <TooltipEffectButton type="move" name="Thunderbolt" />,
    );

    expect(html).toContain("Effect");
    expect(html).toContain('aria-label="View effect details for Thunderbolt"');
    expect(html).toContain('title="View effect details for Thunderbolt"');
  });

  it("uses displayName in accessible labels when provided", () => {
    const html = renderToStaticMarkup(
      <TooltipEffectButton
        type="species"
        name="calyrexshadow"
        displayName="Calyrex-Shadow"
      />,
    );

    expect(html).toContain('aria-label="View effect details for Calyrex-Shadow"');
    expect(html).toContain('title="View effect details for Calyrex-Shadow"');
  });

  it("dispatches openFxLangModal and stops event propagation on click", () => {
    const openFxLangModal = vi.fn();
    const closeFxLangModal = vi.fn();

    // Call the inner component directly with mock context
    let capturedButton: React.ReactElement | null = null;
    function Wrapper() {
      capturedButton = (
        <FxLangModalContext.Provider
          value={{ openFxLangModal, closeFxLangModal }}
        >
          <TooltipEffectButton
            type="item"
            name="choiceband"
            displayName="Choice Band"
            tab="special"
          />
        </FxLangModalContext.Provider>
      );
      return capturedButton;
    }

    renderToStaticMarkup(<Wrapper />);
    expect(capturedButton).toBeDefined();

    // Directly test TooltipEffectButton with mocked hook via context
    let renderedElement: React.ReactElement | null = null;
    function Consumer() {
      renderedElement = TooltipEffectButton({
        type: "item",
        name: "choiceband",
        displayName: "Choice Band",
        tab: "special",
      });
      return renderedElement;
    }

    renderToStaticMarkup(
      <FxLangModalContext.Provider
        value={{ openFxLangModal, closeFxLangModal }}
      >
        <Consumer />
      </FxLangModalContext.Provider>,
    );

    expect(renderedElement).toBeDefined();
    const stopPropagation = vi.fn();
    (renderedElement as unknown as React.ReactElement<{ onClick: (e: unknown) => void }>).props.onClick({
      stopPropagation,
    });

    expect(stopPropagation).toHaveBeenCalledTimes(1);
    expect(openFxLangModal).toHaveBeenCalledWith({
      type: "item",
      name: "choiceband",
      displayName: "Choice Band",
      tab: "special",
    });
  });
});
