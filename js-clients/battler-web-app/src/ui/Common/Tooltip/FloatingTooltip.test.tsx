import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import FloatingTooltip from "./FloatingTooltip";
import { calculateFloatingCoords } from "../../../utils/floatingCoords";

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
});
