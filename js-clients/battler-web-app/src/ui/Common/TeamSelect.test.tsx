import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import TeamSelect from "./TeamSelect";

describe("TeamSelect", () => {
  const teams = {
    "Team A": [
      { name: "Pikachu", species: "Pikachu" },
      { name: "Raichu", species: "Raichu" },
    ],
    "Team B": [{ name: "Charmander", species: "Charmander" }],
  };
  const teamNames = ["Team A", "Team B"];

  it("renders clean options without count text", () => {
    const html = renderToStaticMarkup(
      <TeamSelect
        value=""
        onChange={() => {}}
        teamNames={teamNames}
        teams={teams}
      />,
    );
    expect(html).toContain("Select team");
    expect(html).toContain('<option value="Team A">Team A</option>');
    expect(html).toContain('<option value="Team B">Team B</option>');
    expect(html).not.toContain("Team A (2)");
    expect(html).not.toContain("Team B (1)");
  });

  it("renders preview with TeamMonIcons when a valid team is selected", () => {
    const html = renderToStaticMarkup(
      <TeamSelect
        value="Team A"
        onChange={() => {}}
        teamNames={teamNames}
        teams={teams}
      />,
    );
    expect(html).toContain('src="/assets/mons/icons/pikachu.png"');
    expect(html).toContain('src="/assets/mons/icons/raichu.png"');
  });

  it("omits preview when showPreview is false", () => {
    const html = renderToStaticMarkup(
      <TeamSelect
        value="Team A"
        onChange={() => {}}
        teamNames={teamNames}
        teams={teams}
        showPreview={false}
      />,
    );
    expect(html).not.toContain('src="/assets/mons/icons/pikachu.png"');
  });
});
