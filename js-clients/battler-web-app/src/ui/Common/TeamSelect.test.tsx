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

  it("renders placeholder and accessible combobox role when unselected", () => {
    const html = renderToStaticMarkup(
      <TeamSelect
        id="test-team-select"
        value=""
        onChange={() => {}}
        teamNames={teamNames}
        teams={teams}
      />,
    );

    expect(html).toContain('role="combobox"');
    expect(html).toContain('id="test-team-select"');
    expect(html).toContain('aria-expanded="false"');
    expect(html).toContain("Select team");
    expect(html).not.toContain('src="/assets/mons/icons/pikachu.png"');
  });

  it("renders selected team name and TeamMonIcons inside the trigger", () => {
    const html = renderToStaticMarkup(
      <TeamSelect
        id="test-team-select"
        value="Team A"
        onChange={() => {}}
        teamNames={teamNames}
        teams={teams}
      />,
    );

    expect(html).toContain("Team A");
    expect(html).toContain('src="/assets/mons/icons/pikachu.png"');
    expect(html).toContain('src="/assets/mons/icons/raichu.png"');
    expect(html).toContain('aria-label="Selected team: Team A"');
  });

  it("renders custom placeholder when provided", () => {
    const html = renderToStaticMarkup(
      <TeamSelect
        value=""
        onChange={() => {}}
        teamNames={teamNames}
        teams={teams}
        placeholder="Choose your squad"
      />,
    );

    expect(html).toContain("Choose your squad");
  });

  it("disables the trigger button when disabled prop is true", () => {
    const html = renderToStaticMarkup(
      <TeamSelect
        value="Team A"
        onChange={() => {}}
        teamNames={teamNames}
        teams={teams}
        disabled={true}
      />,
    );

    expect(html).toContain("disabled");
  });

  it("includes hidden input for form validation support", () => {
    const html = renderToStaticMarkup(
      <TeamSelect
        id="battle-team"
        value="Team A"
        onChange={() => {}}
        teamNames={teamNames}
        teams={teams}
        required={true}
      />,
    );

    expect(html).toContain('type="hidden"');
    expect(html).toContain('name="battle-team"');
    expect(html).toContain('value="Team A"');
    expect(html).toContain("required");
  });

  it("renders with combobox attributes and aria-controls linking to listbox", () => {
    const html = renderToStaticMarkup(
      <TeamSelect
        id="accessible-team-select"
        value="Team A"
        onChange={() => {}}
        teamNames={teamNames}
        teams={teams}
      />,
    );

    expect(html).toContain('role="combobox"');
    expect(html).toContain('aria-controls="accessible-team-select-listbox"');
    expect(html).toContain('aria-haspopup="listbox"');
    expect(html).toContain('aria-expanded="false"');
  });
});


