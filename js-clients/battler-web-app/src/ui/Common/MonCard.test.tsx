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
    expect(html).not.toContain("L50");
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
    const rowHtml = renderToStaticMarkup(
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

    expect(rowHtml).toContain("Unrevealed");
    expect(rowHtml).not.toContain("Not revealed");
    expect(rowHtml).toContain("unrevealed");
    expect(rowHtml).toContain("rowIdentity");
    expect(rowHtml).toContain("unrevealedPip");

    const cardHtml = renderToStaticMarkup(
      <MonCard
        name="Unknown"
        hp={100}
        maxHp={100}
        status={null}
        active={false}
        isUnrevealed={true}
        variant="card"
      />,
    );

    expect(cardHtml).toContain("Unrevealed");
    expect(cardHtml).toContain("cardMain");
    expect(cardHtml).toContain("cardDetails");
    expect(cardHtml).toContain("unrevealedPip");
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
    expect(rowHtml).toContain("status-badge unbrought");
    expect(rowHtml).toContain("—");

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
    expect(cardHtml).toContain("status-badge unbrought");
    expect(cardHtml).toContain("—");
  });

  it("renders Mon icon in card variant", () => {
    const html = renderToStaticMarkup(
      <MonCard
        name="Pikachu"
        level={50}
        hp={100}
        maxHp={100}
        variant="card"
      />,
    );

    expect(html).toContain('src="/assets/mons/icons/pikachu.png"');
    expect(html).toContain("cardMonIcon");
  });

  it("renders Mon icon in row variant", () => {
    const html = renderToStaticMarkup(
      <MonCard
        name="Charizard"
        level={100}
        hp={75}
        maxHp={100}
        variant="row"
      />,
    );

    expect(html).toContain('src="/assets/mons/icons/charizard.png"');
    expect(html).toContain("rowMonIcon");
  });

  it("omits Mon icon when isUnrevealed is true", () => {
    const html = renderToStaticMarkup(
      <MonCard
        name="Unknown"
        hp={100}
        maxHp={100}
        isUnrevealed={true}
        variant="card"
      />,
    );

    expect(html).not.toContain("/assets/mons/icons/");
  });
});
