import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import MonCard from "./MonCard";

describe("MonCard", () => {
  it("renders card variant with default props", () => {
    const html = renderToStaticMarkup(
      <MonCard
        name="Pikachu"
        level={50}
        hp={100}
        maxHp={100}
        status={null}
        active={false}
      />,
    );

    expect(html).toContain("Pikachu");
    expect(html).toContain("L50");
    expect(html).toContain("100/100");
    expect(html).toContain("teamSummaryCard");
  });

  it("renders row variant with rowIdentity and rowMeta", () => {
    const html = renderToStaticMarkup(
      <MonCard
        name="Charizard"
        level={100}
        hp={75}
        maxHp={100}
        hpText="75%"
        status="brn"
        active={true}
        variant="row"
      />,
    );

    expect(html).toContain("Charizard");
    expect(html).toContain('title="Charizard"');
    expect(html).not.toContain("L100");
    expect(html).toContain("75%");
    expect(html).toContain("teamSummaryRow");
    expect(html).toContain("rowHpGroup");
    expect(html).toContain("summaryActive");
  });

  it("renders fainted styling when hp is 0 or status is fnt", () => {
    const html = renderToStaticMarkup(
      <MonCard
        name="Blastoise"
        level={50}
        hp={0}
        maxHp={100}
        status="fnt"
        active={false}
        variant="row"
      />,
    );

    expect(html).toContain("summaryFainted");
    expect(html).toContain("fnt");
  });

  it("renders unrevealed placeholder when isUnrevealed is true", () => {
    const html = renderToStaticMarkup(
      <MonCard
        name="Unknown"
        hp={100}
        maxHp={100}
        status={null}
        active={false}
        isUnrevealed={true}
        variant="row"
      />,
    );

    expect(html).toContain("Unrevealed");
    expect(html).not.toContain("Not revealed");
    expect(html).toContain("unrevealed");
  });

  it("renders faded unbrought styling when isUnbrought is true", () => {
    const rowHtml = renderToStaticMarkup(
      <MonCard
        name="Eevee"
        level={50}
        hp={100}
        maxHp={100}
        status={null}
        active={false}
        isUnbrought={true}
        variant="row"
      />,
    );

    expect(rowHtml).toContain("summaryUnbrought");
    expect(rowHtml).toContain("unbroughtPip");

    const cardHtml = renderToStaticMarkup(
      <MonCard
        name="Eevee"
        level={50}
        hp={100}
        maxHp={100}
        status={null}
        active={false}
        isUnbrought={true}
        variant="card"
      />,
    );

    expect(cardHtml).toContain("summaryUnbrought");
  });
});
