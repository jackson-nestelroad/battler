import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import ActionButton from "./ActionButton";

describe("ActionButton", () => {
  it("renders a standard button when no infoResourceType is specified", () => {
    const html = renderToStaticMarkup(
      <ActionButton
        title="Pass"
        subtitle="Leave slot empty"
        className="custom-pass-btn"
        style={{ margin: "10px" }}
      />,
    );

    expect(html).toContain("<button");
    expect(html).toContain("custom-pass-btn");
    expect(html).toContain("Pass");
    expect(html).toContain("Leave slot empty");
    expect(html).toContain("margin:10px");
  });

  it("renders role=button container without nested <button> when infoResourceType is provided", () => {
    const html = renderToStaticMarkup(
      <ActionButton
        title="Thunderbolt"
        subtitle="Electric | PP: 15/15"
        infoResourceType="move"
        infoResourceName="Thunderbolt"
        typeColor="var(--color-type-electric)"
        badgeText="Z-Move"
      />,
    );

    expect(html).not.toMatch(/<button[\s\S]*?<button/);
    expect(html).toContain('role="button"');
    expect(html).toContain('tabindex="0"');
    expect(html).toContain("Thunderbolt");
    expect(html).toContain("Z-Move");
    expect(html).toContain("--type-color:var(--color-type-electric)");
    expect(html).toContain('aria-label="View Thunderbolt details"');
    expect(html).toContain('title="View Thunderbolt details"');
  });

  it("handles disabled state properly for both variants", () => {
    const standardHtml = renderToStaticMarkup(
      <ActionButton title="Disabled Pass" disabled={true} />,
    );
    expect(standardHtml).toContain("<button");
    expect(standardHtml).toContain("disabled");

    const infoHtml = renderToStaticMarkup(
      <ActionButton
        title="Disabled Move"
        disabled={true}
        infoResourceType="move"
        infoResourceName="Tackle"
      />,
    );
    expect(infoHtml).toContain('aria-disabled="true"');
    expect(infoHtml).toContain('tabindex="-1"');
  });

  it("omits moveHeaderRight container when neither badgeText nor info button exists", () => {
    const plainHtml = renderToStaticMarkup(<ActionButton title="Pass" />);
    expect(plainHtml).not.toContain("moveHeaderRight");

    const badgeHtml = renderToStaticMarkup(
      <ActionButton title="Thunderbolt" badgeText="Z-Move" />,
    );
    expect(badgeHtml).toContain("moveHeaderRight");
    expect(badgeHtml).toContain("Z-Move");
  });

  it("invokes onClick when Enter or Space is pressed on role=button container", () => {
    const handleClick = vi.fn();
    const element = ActionButton({
      title: "Thunderbolt",
      infoResourceType: "move",
      infoResourceName: "Thunderbolt",
      onClick: handleClick,
    });

    expect(element.props.role).toBe("button");
    const preventDefault = vi.fn();
    const fakeTarget = {};

    // Enter triggers onClick
    element.props.onKeyDown({
      key: "Enter",
      target: fakeTarget,
      currentTarget: fakeTarget,
      preventDefault,
    });
    expect(handleClick).toHaveBeenCalledTimes(1);
    expect(preventDefault).toHaveBeenCalledTimes(1);

    // Space triggers onClick
    element.props.onKeyDown({
      key: " ",
      target: fakeTarget,
      currentTarget: fakeTarget,
      preventDefault,
    });
    expect(handleClick).toHaveBeenCalledTimes(2);

    // Other keys do not trigger onClick
    element.props.onKeyDown({
      key: "Tab",
      target: fakeTarget,
      currentTarget: fakeTarget,
      preventDefault,
    });
    expect(handleClick).toHaveBeenCalledTimes(2);

    // Child target does not trigger parent button onClick
    const childTarget = {};
    element.props.onKeyDown({
      key: "Enter",
      target: childTarget,
      currentTarget: fakeTarget,
      preventDefault,
    });
    expect(handleClick).toHaveBeenCalledTimes(2);

    // Disabled action button ignores keyboard activations
    const disabledClick = vi.fn();
    const disabledElement = ActionButton({
      title: "Thunderbolt",
      infoResourceType: "move",
      infoResourceName: "Thunderbolt",
      disabled: true,
      onClick: disabledClick,
    });
    disabledElement.props.onKeyDown({
      key: "Enter",
      target: fakeTarget,
      currentTarget: fakeTarget,
      preventDefault,
    });
    expect(disabledClick).not.toHaveBeenCalled();
  });

  it("supports badgeVariant for zmove and maxMove", () => {
    const zHtml = renderToStaticMarkup(
      <ActionButton
        title="Gigavolt Havoc"
        badgeText="Z-Move"
        badgeVariant="zmove"
      />,
    );
    expect(zHtml).toContain("zmoveBadge");
    expect(zHtml).toContain("Z-Move");

    const maxHtml = renderToStaticMarkup(
      <ActionButton
        title="Max Lightning"
        badgeText="Max Move"
        badgeVariant="maxMove"
      />,
    );
    expect(maxHtml).toContain("maxMoveBadge");
    expect(maxHtml).toContain("Max Move");
  });

  it("automatically infers badgeVariant from badgeText when omitted", () => {
    const inferredZHtml = renderToStaticMarkup(
      <ActionButton title="Gigavolt Havoc" badgeText="Z-Move" />,
    );
    expect(inferredZHtml).toContain("zmoveBadge");

    const inferredMaxHtml = renderToStaticMarkup(
      <ActionButton title="Max Lightning" badgeText="Max Move" />,
    );
    expect(inferredMaxHtml).toContain("maxMoveBadge");
  });

  it("handles mouse click bindings properly for enabled and disabled states", () => {
    // 1. Standard button: onClick attached when enabled, undefined when disabled
    const handleStandardClick = vi.fn();
    const standardEnabled = ActionButton({
      title: "Pass",
      onClick: handleStandardClick,
    });
    expect(standardEnabled.props.onClick).toBe(handleStandardClick);
    standardEnabled.props.onClick();
    expect(handleStandardClick).toHaveBeenCalledTimes(1);

    const standardDisabled = ActionButton({
      title: "Pass",
      disabled: true,
      onClick: handleStandardClick,
    });
    expect(standardDisabled.props.disabled).toBe(true);
    expect(standardDisabled.props.onClick).toBeUndefined();

    // 2. Info button (role=button container): onClick attached when enabled, undefined when disabled
    const handleInfoClick = vi.fn();
    const infoEnabled = ActionButton({
      title: "Thunderbolt",
      infoResourceType: "move",
      infoResourceName: "Thunderbolt",
      onClick: handleInfoClick,
    });
    expect(infoEnabled.props.role).toBe("button");
    expect(infoEnabled.props.onClick).toBe(handleInfoClick);
    infoEnabled.props.onClick();
    expect(handleInfoClick).toHaveBeenCalledTimes(1);

    const infoDisabled = ActionButton({
      title: "Thunderbolt",
      infoResourceType: "move",
      infoResourceName: "Thunderbolt",
      disabled: true,
      onClick: handleInfoClick,
    });
    expect(infoDisabled.props["aria-disabled"]).toBe(true);
    expect(infoDisabled.props.tabIndex).toBe(-1);
    expect(infoDisabled.props.onClick).toBeUndefined();
  });
});

