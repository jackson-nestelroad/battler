import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import StatusBadge from "./StatusBadge";

describe("StatusBadge", () => {
  it("renders OK when healthy with no status", () => {
    const html = renderToStaticMarkup(<StatusBadge />);
    expect(html).toBe('<span class="status-badge ok">OK</span>');
  });

  it("renders OK when status is null or empty", () => {
    expect(renderToStaticMarkup(<StatusBadge status={null} />)).toBe(
      '<span class="status-badge ok">OK</span>',
    );
    expect(renderToStaticMarkup(<StatusBadge status="" />)).toBe(
      '<span class="status-badge ok">OK</span>',
    );
  });

  it("renders OK when status is ok", () => {
    const html = renderToStaticMarkup(<StatusBadge status="ok" />);
    expect(html).toBe('<span class="status-badge ok">OK</span>');
  });

  it("renders FNT when isFainted is true even with another status", () => {
    const html = renderToStaticMarkup(<StatusBadge isFainted status="brn" />);
    expect(html).toBe('<span class="status-badge fnt">FNT</span>');
  });

  it("renders FNT when status is fnt or fainted", () => {
    expect(renderToStaticMarkup(<StatusBadge status="fnt" />)).toBe(
      '<span class="status-badge fnt">FNT</span>',
    );
    expect(renderToStaticMarkup(<StatusBadge status="fainted" />)).toBe(
      '<span class="status-badge fnt">FNT</span>',
    );
  });

  it("renders standard status conditions correctly", () => {
    expect(renderToStaticMarkup(<StatusBadge status="brn" />)).toBe(
      '<span class="status-badge brn">BRN</span>',
    );
    expect(renderToStaticMarkup(<StatusBadge status="psn" />)).toBe(
      '<span class="status-badge psn">PSN</span>',
    );
    expect(renderToStaticMarkup(<StatusBadge status="tox" />)).toBe(
      '<span class="status-badge tox">TOX</span>',
    );
    expect(renderToStaticMarkup(<StatusBadge status="par" />)).toBe(
      '<span class="status-badge par">PAR</span>',
    );
    expect(renderToStaticMarkup(<StatusBadge status="slp" />)).toBe(
      '<span class="status-badge slp">SLP</span>',
    );
    expect(renderToStaticMarkup(<StatusBadge status="frz" />)).toBe(
      '<span class="status-badge frz">FRZ</span>',
    );
  });

  it("handles verbose status names", () => {
    expect(renderToStaticMarkup(<StatusBadge status="Burn" />)).toBe(
      '<span class="status-badge brn">BRN</span>',
    );
    expect(renderToStaticMarkup(<StatusBadge status="Paralysis" />)).toBe(
      '<span class="status-badge par">PAR</span>',
    );
    expect(renderToStaticMarkup(<StatusBadge status="Bad Poison" />)).toBe(
      '<span class="status-badge tox">TOX</span>',
    );
  });

  it("applies custom className", () => {
    const html = renderToStaticMarkup(
      <StatusBadge status="brn" className="custom-class" />,
    );
    expect(html).toBe('<span class="status-badge brn custom-class">BRN</span>');
  });
});
