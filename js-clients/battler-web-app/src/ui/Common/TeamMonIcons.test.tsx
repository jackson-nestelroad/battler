import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import TeamMonIcons from "./TeamMonIcons";

describe("TeamMonIcons", () => {
  it("renders empty markup if no members provided", () => {
    const html = renderToStaticMarkup(<TeamMonIcons members={[]} />);
    expect(html).toBe("");
  });

  it("renders empty markup if members is undefined", () => {
    const html = renderToStaticMarkup(<TeamMonIcons />);
    expect(html).toBe("");
  });

  it("renders sprite icons with title tooltips for team members", () => {
    const team = [
      { name: "Pikachu", species: "Pikachu" },
      { name: "Charizard", species: "Charizard" },
      { name: "Bulbasaur", species: "Bulbasaur" },
    ];

    const html = renderToStaticMarkup(<TeamMonIcons members={team} />);
    expect(html).toContain('src="/assets/mons/icons/pikachu.png"');
    expect(html).toContain('src="/assets/mons/icons/charizard.png"');
    expect(html).toContain('src="/assets/mons/icons/bulbasaur.png"');

    expect(html).toContain('title="Pikachu"');
    expect(html).toContain('title="Charizard"');
    expect(html).toContain('title="Bulbasaur"');
  });

  it("caps rendering to maxMons prop and renders +N overflow badge with tooltip", () => {
    const team = [
      { species: "Pikachu" },
      { species: "Charizard" },
      { species: "Bulbasaur" },
      { species: "Squirtle" },
    ];

    const html = renderToStaticMarkup(<TeamMonIcons members={team} maxMons={2} />);
    expect(html).toContain('src="/assets/mons/icons/pikachu.png"');
    expect(html).toContain('src="/assets/mons/icons/charizard.png"');
    expect(html).not.toContain('src="/assets/mons/icons/bulbasaur.png"');
    expect(html).not.toContain('src="/assets/mons/icons/squirtle.png"');
    expect(html).toContain("+2");
    expect(html).toContain('title="+2 more: Bulbasaur, Squirtle"');
  });

  it("applies size class correctly", () => {
    const team = [{ species: "Pikachu" }];
    const html = renderToStaticMarkup(<TeamMonIcons members={team} size="sm" />);
    expect(html).toContain("sm");
  });
});
