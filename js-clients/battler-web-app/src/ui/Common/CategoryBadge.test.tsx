import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import CategoryBadge from "./CategoryBadge";

describe("CategoryBadge", () => {
  it("renders Physical category correctly by default", () => {
    const html = renderToStaticMarkup(<CategoryBadge category="Physical" />);
    expect(html).toContain("Physical");
    expect(html).toContain('data-category="physical"');
    expect(html).toContain('src="/assets/categories/physical.png"');
    expect(html).toContain("var(--color-category-physical");
    expect(html).toContain("categoryBadgeMd");
    expect(html).toContain("fixedWidth");
  });

  it("renders Special category correctly", () => {
    const html = renderToStaticMarkup(<CategoryBadge category="Special" />);
    expect(html).toContain("Special");
    expect(html).toContain('data-category="special"');
    expect(html).toContain('src="/assets/categories/special.png"');
    expect(html).toContain("var(--color-category-special");
  });

  it("renders Status category correctly", () => {
    const html = renderToStaticMarkup(<CategoryBadge category="Status" />);
    expect(html).toContain("Status");
    expect(html).toContain('data-category="status"');
    expect(html).toContain('src="/assets/categories/status.png"');
    expect(html).toContain("var(--color-category-status");
  });

  it("falls back gracefully for unknown category", () => {
    const html = renderToStaticMarkup(<CategoryBadge category="Unknown" />);
    expect(html).toContain("Unknown");
    expect(html).toContain('data-category="status"');
    expect(html).toContain('src="/assets/categories/status.png"');
  });

  it("normalizes category name with whitespace or lowercase", () => {
    const html = renderToStaticMarkup(<CategoryBadge category="  physical  " />);
    expect(html).toContain('data-category="physical"');
    expect(html).toContain('src="/assets/categories/physical.png"');
  });

  it("omits the icon when showIcon is false", () => {
    const html = renderToStaticMarkup(<CategoryBadge category="Special" showIcon={false} />);
    expect(html).toContain("Special");
    expect(html).not.toContain("<img");
  });

  it("applies small size class when size is sm", () => {
    const html = renderToStaticMarkup(<CategoryBadge category="Status" size="sm" />);
    expect(html).toContain("categoryBadgeSm");
  });

  it("omits fixedWidth when fixedWidth is false", () => {
    const html = renderToStaticMarkup(<CategoryBadge category="Physical" fixedWidth={false} />);
    expect(html).not.toContain("fixedWidth");
  });

  it("appends custom className when provided", () => {
    const html = renderToStaticMarkup(<CategoryBadge category="Special" className="custom-class" />);
    expect(html).toContain("custom-class");
  });
});
