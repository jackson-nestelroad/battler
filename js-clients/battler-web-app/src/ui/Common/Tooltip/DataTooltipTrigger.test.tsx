import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import DataTooltipTrigger from "./DataTooltipTrigger";
import { useTooltipChildTracker } from "./TooltipContext";

describe("DataTooltipTrigger", () => {
  it("renders children cleanly", () => {
    const html = renderToStaticMarkup(
      <DataTooltipTrigger resourceType="move" name="Thunderbolt">
        <span>Thunderbolt</span>
      </DataTooltipTrigger>,
    );
    expect(html).toContain("Thunderbolt");
  });

  it("renders as simple element if name is empty", () => {
    const html = renderToStaticMarkup(
      <DataTooltipTrigger resourceType="move" name="">
        <span>Empty</span>
      </DataTooltipTrigger>,
    );
    expect(html).toContain("Empty");

    const btnHtml = renderToStaticMarkup(
      <DataTooltipTrigger resourceType="move" name="" as="button">
        Empty
      </DataTooltipTrigger>,
    );
    expect(btnHtml).toContain('<button type="button"');
    expect(btnHtml).toContain("Empty");
  });

  it("supports as button", () => {
    const html = renderToStaticMarkup(
      <DataTooltipTrigger resourceType="ability" name="Levitate" as="button">
        Levitate
      </DataTooltipTrigger>,
    );
    expect(html).toContain("<button");
    expect(html).toContain("Levitate");
  });

  it("renders nested triggers cleanly", () => {
    const html = renderToStaticMarkup(
      <DataTooltipTrigger resourceType="species" name="Pikachu">
        <div>
          <span>Pikachu</span>
          <DataTooltipTrigger resourceType="ability" name="Static">
            <span>Static</span>
          </DataTooltipTrigger>
        </div>
      </DataTooltipTrigger>,
    );
    expect(html).toContain("Pikachu");
    expect(html).toContain("Static");
  });

  it("includes aria-haspopup and aria-expanded attributes for accessibility", () => {
    const spanHtml = renderToStaticMarkup(
      <DataTooltipTrigger resourceType="item" name="Leftovers">
        <span>Leftovers</span>
      </DataTooltipTrigger>,
    );
    expect(spanHtml).toContain('aria-haspopup="dialog"');
    expect(spanHtml).toContain('aria-expanded="false"');

    const btnHtml = renderToStaticMarkup(
      <DataTooltipTrigger resourceType="item" name="Leftovers" as="button">
        Leftovers
      </DataTooltipTrigger>,
    );
    expect(btnHtml).toContain('aria-haspopup="dialog"');
    expect(btnHtml).toContain('aria-expanded="false"');
  });

  it("enforces mutual exclusion among sibling children and supports closeChild", () => {
    let tracker!: ReturnType<typeof useTooltipChildTracker>;
    function TrackerConsumer({ onReady }: { onReady: (t: typeof tracker) => void }) {
      const t = useTooltipChildTracker();
      onReady(t);
      return <div />;
    }

    renderToStaticMarkup(<TrackerConsumer onReady={(t) => { tracker = t; }} />);

    const closeChild1 = vi.fn();
    const closeChild2 = vi.fn();

    // Child 1 opens
    const unregister1 = tracker.contextValue.openChild!("child-1", closeChild1);
    expect(closeChild1).not.toHaveBeenCalled();

    // Opening Child 2 under same parent must automatically close Child 1
    const unregister2 = tracker.contextValue.openChild!("child-2", closeChild2);
    expect(closeChild1).toHaveBeenCalledTimes(1);
    expect(closeChild2).not.toHaveBeenCalled();

    // Parent closes child (e.g. clicking on parent surface)
    tracker.closeChild();
    expect(closeChild2).toHaveBeenCalledTimes(1);

    unregister1();
    unregister2();
  });

  it("tracks child content elements for hit testing", () => {
    let tracker!: ReturnType<typeof useTooltipChildTracker>;
    function TrackerConsumer({ onReady }: { onReady: (t: typeof tracker) => void }) {
      const t = useTooltipChildTracker();
      onReady(t);
      return <div />;
    }

    renderToStaticMarkup(<TrackerConsumer onReady={(t) => { tracker = t; }} />);

    const childEl = {
      contains: (node: unknown) => node === "inside-child",
    } as unknown as HTMLElement;

    const unregister = tracker.contextValue.registerChildContent!(childEl);
    expect(tracker.isTargetInChild("inside-child" as unknown as Node)).toBe(true);
    expect(tracker.isTargetInChild("outside-child" as unknown as Node)).toBe(false);

    unregister();
    expect(tracker.isTargetInChild("inside-child" as unknown as Node)).toBe(false);
  });
});
