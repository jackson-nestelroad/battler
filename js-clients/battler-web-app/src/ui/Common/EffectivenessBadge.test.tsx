import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import EffectivenessBadge, { MoveEffectivenessBadge } from "./EffectivenessBadge";

describe("EffectivenessBadge", () => {
  it("renders 4× (double super effective)", () => {
    const html = renderToStaticMarkup(
      <EffectivenessBadge mult={4} targetTypes={["Water", "Ground"]} />,
    );
    expect(html).toContain("4×");
    expect(html).toContain("multQuad");
    expect(html).toContain("4× (Double super effective) against Water/Ground");
  });

  it("renders 2× (super effective)", () => {
    const html = renderToStaticMarkup(
      <EffectivenessBadge mult={2} targetTypes={["Grass"]} />,
    );
    expect(html).toContain("2×");
    expect(html).toContain("multSuper");
    expect(html).toContain("2× (Super effective) against Grass");
  });

  it("renders 1× (neutral effectiveness)", () => {
    const html = renderToStaticMarkup(<EffectivenessBadge mult={1} />);
    expect(html).toContain("1×");
    expect(html).toContain("multNeutral");
    expect(html).toContain("1× (Normal effectiveness)");
  });

  it("renders ½× (resisted)", () => {
    const html = renderToStaticMarkup(
      <EffectivenessBadge mult={0.5} targetTypes={["Fire"]} />,
    );
    expect(html).toContain("½×");
    expect(html).toContain("multResist");
    expect(html).toContain("½× (Resisted) against Fire");
  });

  it("renders ¼× (double resisted)", () => {
    const html = renderToStaticMarkup(
      <EffectivenessBadge mult={0.25} targetTypes={["Steel", "Rock"]} />,
    );
    expect(html).toContain("¼×");
    expect(html).toContain("multQuadResist");
    expect(html).toContain("¼× (Double resisted) against Steel/Rock");
  });

  it("renders 0× (immune)", () => {
    const html = renderToStaticMarkup(
      <EffectivenessBadge mult={0} targetTypes={["Ghost"]} />,
    );
    expect(html).toContain("0×");
    expect(html).toContain("multImmune");
    expect(html).toContain("0× (No effect) against Ghost");
  });

  it("applies optional className", () => {
    const html = renderToStaticMarkup(
      <EffectivenessBadge mult={2} className="custom-test-class" />,
    );
    expect(html).toContain("custom-test-class");
  });

  describe("MoveEffectivenessBadge", () => {
    const mockChart = {
      types: {
        Water: { Fire: 2 },
      },
    };

    it("returns null for Status moves", () => {
      const html = renderToStaticMarkup(
        <MoveEffectivenessBadge
          typeChart={mockChart as any}
          move={{ category: "Status", type: "Water" } as any}
          targetTypes={["Fire"]}
        />,
      );
      expect(html).toBe("");
    });

    it("returns null when targetTypes is empty", () => {
      const html = renderToStaticMarkup(
        <MoveEffectivenessBadge
          typeChart={mockChart as any}
          move={{ category: "Special", type: "Water" } as any}
          targetTypes={[]}
        />,
      );
      expect(html).toBe("");
    });

    it("renders EffectivenessBadge for damaging moves with targets", () => {
      const html = renderToStaticMarkup(
        <MoveEffectivenessBadge
          typeChart={mockChart as any}
          move={{ category: "Special", type: "Water" } as any}
          targetTypes={["Fire"]}
        />,
      );
      expect(html).toContain("2×");
      expect(html).toContain("multSuper");
    });
  });
});
