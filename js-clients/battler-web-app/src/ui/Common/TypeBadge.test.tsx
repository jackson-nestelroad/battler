import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import TypeBadge from "./TypeBadge";

describe("TypeBadge", () => {
  it("renders type text correctly", () => {
    const html = renderToStaticMarkup(<TypeBadge type="Electric" />);
    expect(html).toContain("Electric");
    expect(html).toContain('data-type="electric"');
    expect(html).toContain("var(--color-type-electric, var(--border-color))");
  });

  it("applies medium size class by default", () => {
    const html = renderToStaticMarkup(<TypeBadge type="Fire" />);
    expect(html).toContain("typeBadgeMd");
  });

  it("applies small size class when size is sm", () => {
    const html = renderToStaticMarkup(<TypeBadge type="Water" size="sm" />);
    expect(html).toContain("typeBadgeSm");
  });

  it("appends custom className when provided", () => {
    const html = renderToStaticMarkup(<TypeBadge type="Grass" className="custom-test-class" />);
    expect(html).toContain("custom-test-class");
  });

  it("normalizes type names with whitespace or uppercase", () => {
    const html = renderToStaticMarkup(<TypeBadge type="  Psychic  " />);
    expect(html).toContain('data-type="psychic"');
    expect(html).toContain("var(--color-type-psychic, var(--border-color))");
  });
});
