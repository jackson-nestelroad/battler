import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import TypeBadge from "./TypeBadge";

describe("TypeBadge", () => {
  it("renders type text and icon correctly by default", () => {
    const html = renderToStaticMarkup(<TypeBadge type="Electric" />);
    expect(html).toContain("Electric");
    expect(html).toContain('data-type="electric"');
    expect(html).toContain("var(--color-type-electric, var(--border-color))");
    expect(html).toContain('src="/assets/types/electric.png"');
    expect(html).toContain('aria-hidden="true"');
  });

  it("omits the icon when showIcon is false", () => {
    const html = renderToStaticMarkup(<TypeBadge type="Electric" showIcon={false} />);
    expect(html).toContain("Electric");
    expect(html).not.toContain("<img");
  });

  it("maps '???' type to 'unknown.png' asset and unknown color", () => {
    const html = renderToStaticMarkup(<TypeBadge type="???" />);
    expect(html).toContain("???");
    expect(html).toContain('data-type="unknown"');
    expect(html).toContain('src="/assets/types/unknown.png"');
    expect(html).toContain("var(--color-type-unknown");
  });

  it("applies stellar background variable for Stellar type", () => {
    const html = renderToStaticMarkup(<TypeBadge type="Stellar" />);
    expect(html).toContain("var(--background-type-stellar");
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
    expect(html).toContain('src="/assets/types/psychic.png"');
  });

  it("applies fixedWidth class when fixedWidth is true", () => {
    const html = renderToStaticMarkup(<TypeBadge type="Bug" fixedWidth />);
    expect(html).toContain("fixedWidth");
  });
});
