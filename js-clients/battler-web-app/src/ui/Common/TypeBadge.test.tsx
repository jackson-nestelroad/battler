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

  it("renders 'None' type badge with none.png asset and none color", () => {
    const html = renderToStaticMarkup(<TypeBadge type="None" />);
    expect(html).toContain("None");
    expect(html).toContain('data-type="none"');
    expect(html).toContain('src="/assets/types/none.png"');
    expect(html).toContain("var(--color-type-none");
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

  it("applies fixedWidth class by default", () => {
    const html = renderToStaticMarkup(<TypeBadge type="Bug" />);
    expect(html).toContain("fixedWidth");
  });

  it("omits fixedWidth class when fixedWidth is false", () => {
    const html = renderToStaticMarkup(<TypeBadge type="Bug" fixedWidth={false} />);
    expect(html).not.toContain("fixedWidth");
  });

  it("renders standard variant by default without tera crystal caps", () => {
    const html = renderToStaticMarkup(<TypeBadge type="Ghost" />);
    expect(html).toContain('data-variant="standard"');
    expect(html).not.toContain("typeBadgeTera");
    expect(html).not.toContain("teraCapLeft");
    expect(html).not.toContain("teraCapRight");
  });

  it("renders crystalline caps and tera class when variant is tera", () => {
    const html = renderToStaticMarkup(<TypeBadge type="Ghost" variant="tera" />);
    expect(html).toContain('data-variant="tera"');
    expect(html).toContain("typeBadgeTera");
    expect(html).toContain("teraCapLeft");
    expect(html).toContain("teraCapRight");
    expect(html).toContain("facetHighlight");
    expect(html).toContain("facetShadow");
    expect(html).toContain("Ghost");
    expect(html).toContain('src="/assets/types/ghost.png"');
  });

  it("supports small size with tera variant", () => {
    const html = renderToStaticMarkup(<TypeBadge type="Water" size="sm" variant="tera" />);
    expect(html).toContain("typeBadgeTera");
    expect(html).toContain("typeBadgeSm");
    expect(html).toContain('data-variant="tera"');
  });
});
