import * as React from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import FloatingTooltip from "./FloatingTooltip";
import { calculateFloatingCoords, getElementRect } from "../../../utils/floatingCoords";

function createMockRect(rect: Partial<DOMRect>): DOMRect {
  return {
    top: 0,
    bottom: 0,
    left: 0,
    right: 0,
    width: 0,
    height: 0,
    x: 0,
    y: 0,
    toJSON: () => {},
    ...rect,
  };
}

describe("calculateFloatingCoords", () => {
  it("places tooltip to the left when preferredPlacement is left and room allows", () => {
    const target = createMockRect({
      left: 800,
      right: 1000,
      top: 200,
      bottom: 250,
      width: 200,
      height: 50,
    });

    const coords = calculateFloatingCoords(target, 300, 400, "left", 1200, 800);
    expect(coords.placement).toBe("left");
    expect(coords.left).toBe(800 - 300 - 8); // 492
    // Vertically centered: 200 + 25 - 200 = 25
    expect(coords.top).toBe(25);
  });

  it("falls back to top/bottom when preferredPlacement is left but not enough room on left", () => {
    const target = createMockRect({
      left: 100,
      right: 300,
      top: 500,
      bottom: 550,
      width: 200,
      height: 50,
    });

    const coords = calculateFloatingCoords(target, 300, 400, "left", 1200, 800);
    expect(coords.placement).toBe("top");
    expect(coords.top).toBe(500 - 400 - 8); // 92
  });

  it("places tooltip above target when preferredPlacement is top", () => {
    const target = createMockRect({
      left: 400,
      right: 500,
      top: 500,
      bottom: 550,
      width: 100,
      height: 50,
    });

    const coords = calculateFloatingCoords(target, 200, 200, "top", 1000, 800);
    expect(coords.placement).toBe("top");
    expect(coords.top).toBe(500 - 200 - 8); // 292
  });

  it("places tooltip below target when preferredPlacement is bottom", () => {
    const target = createMockRect({
      left: 400,
      right: 500,
      top: 100,
      bottom: 150,
      width: 100,
      height: 50,
    });

    const coords = calculateFloatingCoords(target, 200, 200, "bottom", 1000, 800);
    expect(coords.placement).toBe("bottom");
    expect(coords.top).toBe(150 + 8); // 158
  });

  it("flips to bottom when top clips off-screen", () => {
    const target = createMockRect({
      left: 400,
      right: 500,
      top: 50,
      bottom: 100,
      width: 100,
      height: 50,
    });

    const coords = calculateFloatingCoords(target, 200, 200, "top", 1000, 800);
    expect(coords.placement).toBe("bottom");
    expect(coords.top).toBe(100 + 8); // 108
  });

  it("never allows tooltip to extend below bottom of viewport", () => {
    const target = createMockRect({
      left: 400,
      right: 500,
      top: 550,
      bottom: 600,
      width: 100,
      height: 50,
    });

    // Tooltip height is 350px in a 650px high viewport
    const coords = calculateFloatingCoords(target, 300, 350, "bottom", 1000, 650);
    // Even if preferred was bottom, it should flip or clamp so top + height <= 650 - 12
    expect(coords.top + 350).toBeLessThanOrEqual(650 - 12);
    expect(coords.top).toBeGreaterThanOrEqual(12);
  });

  it("places above target when target is near bottom of viewport and room allows", () => {
    const target = createMockRect({
      left: 200,
      right: 400,
      top: 500,
      bottom: 550,
      width: 200,
      height: 50,
    });

    const coords = calculateFloatingCoords(target, 320, 320, "top", 1024, 768);
    expect(coords.placement).toBe("top");
    expect(coords.top).toBe(500 - 320 - 8); // 172
    expect(coords.top + 320).toBeLessThanOrEqual(768 - 12);
  });
});

describe("FloatingTooltip", () => {
  it("renders null during server rendering / unmounted", () => {
    const html = renderToStaticMarkup(
      <FloatingTooltip isOpen={true} targetRect={createMockRect({ left: 100, top: 100 })}>
        <div>Tooltip Content</div>
      </FloatingTooltip>,
    );
    expect(html).toBe("");
  });

  it("isolates bubble events while avoiding capture-phase interference with children", () => {
    const originalDocument = globalThis.document;
    const internals = (
      React as unknown as {
        __CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE: { H: unknown };
      }
    ).__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE;
    const prevH = internals.H;

    try {
      internals.H = {
        useState: (init: unknown) => [
          typeof init === "boolean"
            ? true
            : { top: 100, left: 100, placement: "top" },
          () => {},
        ],
        useRef: (init: unknown) => ({ current: init }),
        useCallback: (fn: unknown) => fn,
        useEffect: () => {},
        useLayoutEffect: () => {},
      };
      globalThis.document = { body: { nodeType: 1 } } as unknown as Document;

      const portal = FloatingTooltip({
        isOpen: true,
        children: <button id="inner-tab">Tab 1</button>,
      });

      expect(portal).toBeDefined();
      const portalContainer = (
        portal as unknown as { children: { props: Record<string, any> } }
      ).children;
      expect(portalContainer.props.className).toContain("floatingPortal");

      // Verify capture-phase handlers are NOT present (avoid blocking interactive children)
      expect(portalContainer.props.onClickCapture).toBeUndefined();
      expect(portalContainer.props.onMouseDownCapture).toBeUndefined();
      expect(portalContainer.props.onPointerDownCapture).toBeUndefined();

      // Verify pointerdown and mousedown are NOT intercepted (allows parent/outside click tracking)
      expect(portalContainer.props.onMouseDown).toBeUndefined();
      expect(portalContainer.props.onPointerDown).toBeUndefined();

      // Verify onClick stopPropagation is called on bubble phase to protect parent ActionButton
      const stopPropagationClick = vi.fn();
      portalContainer.props.onClick({ stopPropagation: stopPropagationClick });
      expect(stopPropagationClick).toHaveBeenCalledTimes(1);
    } finally {
      internals.H = prevH;
      globalThis.document = originalDocument;
    }
  });
});

describe("getElementRect", () => {
  it("returns element bounding rect when dimensions are non-zero", () => {
    const mockEl = {
      getBoundingClientRect: () =>
        createMockRect({ left: 50, top: 100, width: 200, height: 40 }),
      firstElementChild: null,
    } as unknown as HTMLElement;

    const rect = getElementRect(mockEl);
    expect(rect.width).toBe(200);
    expect(rect.height).toBe(40);
    expect(rect.left).toBe(50);
  });

  it("falls back to firstElementChild bounding rect when container is 0x0", () => {
    const childEl = {
      getBoundingClientRect: () =>
        createMockRect({ left: 60, top: 110, width: 180, height: 36 }),
    };

    const containerEl = {
      getBoundingClientRect: () =>
        createMockRect({ left: 0, top: 0, width: 0, height: 0 }),
      firstElementChild: childEl,
    } as unknown as HTMLElement;

    const rect = getElementRect(containerEl);
    expect(rect.width).toBe(180);
    expect(rect.height).toBe(36);
    expect(rect.left).toBe(60);
  });

  it("returns 0x0 rect when element has 0 dimensions and no firstElementChild", () => {
    const emptyEl = {
      getBoundingClientRect: () =>
        createMockRect({ left: 0, top: 0, width: 0, height: 0 }),
      firstElementChild: null,
    } as unknown as HTMLElement;

    const rect = getElementRect(emptyEl);
    expect(rect.width).toBe(0);
    expect(rect.height).toBe(0);
  });
});
