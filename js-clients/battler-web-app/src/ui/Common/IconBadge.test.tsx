import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import IconBadge from "./IconBadge";

describe("IconBadge", () => {
  it("renders label and icon correctly by default", () => {
    const html = renderToStaticMarkup(
      <IconBadge label="Test" iconSrc="/assets/test.png" background="red" />
    );
    expect(html).toContain("Test");
    expect(html).toContain('src="/assets/test.png"');
    expect(html).toContain('aria-hidden="true"');
    expect(html).toContain("iconBadgeMd");
    expect(html).toContain("fixedWidth");
  });

  it("omits icon when showIcon is false or iconSrc is omitted", () => {
    const withoutIcon = renderToStaticMarkup(<IconBadge label="NoIcon" showIcon={false} iconSrc="/test.png" />);
    expect(withoutIcon).not.toContain("<img");

    const withoutSrc = renderToStaticMarkup(<IconBadge label="NoSrc" />);
    expect(withoutSrc).not.toContain("<img");
  });

  it("applies small size class when size is sm", () => {
    const html = renderToStaticMarkup(<IconBadge label="Small" size="sm" />);
    expect(html).toContain("iconBadgeSm");
  });

  it("omits fixedWidth when fixedWidth is false", () => {
    const html = renderToStaticMarkup(<IconBadge label="Fluid" fixedWidth={false} />);
    expect(html).not.toContain("fixedWidth");
  });

  it("renders custom children and data attributes", () => {
    const html = renderToStaticMarkup(
      <IconBadge
        label="Custom"
        dataAttributes={{ "data-category": "physical" }}
      >
        <span className="custom-decoration" />
      </IconBadge>
    );
    expect(html).toContain('data-category="physical"');
    expect(html).toContain("custom-decoration");
  });

  it("appends custom class names for root, icon, and text", () => {
    const html = renderToStaticMarkup(
      <IconBadge
        label="Classes"
        iconSrc="/icon.png"
        className="root-class"
        iconClassName="icon-class"
        textClassName="text-class"
      />
    );
    expect(html).toContain("root-class");
    expect(html).toContain("icon-class");
    expect(html).toContain("text-class");
  });
});
