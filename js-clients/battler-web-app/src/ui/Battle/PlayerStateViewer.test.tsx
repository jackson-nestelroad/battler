import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import PlayerStateViewer from "./PlayerStateViewer";
import type { BattleState } from "battler-state";

describe("PlayerStateViewer", () => {
  it("renders None when battleState is null or empty", () => {
    const htmlNull = renderToStaticMarkup(<PlayerStateViewer battleState={null} />);
    expect(htmlNull).toContain("None");

    const htmlEmpty = renderToStaticMarkup(
      <PlayerStateViewer battleState={{ field: { sides: [] } } as unknown as BattleState} />,
    );
    expect(htmlEmpty).toContain("None");
  });

  it("renders sides, players, active conditions, and Mon roster", () => {
    const mockState = {
      field: {
        sides: [
          {
            id: 0,
            name: "Side 1",
            conditions: {
              "Stealth Rock": {},
            },
            players: {
              "player-1": {
                id: "player-1",
                name: "Alice",
                team_size: 2,
                mons: [
                  {
                    physical_appearance: { name: "Pikachu", species: "Pikachu", gender: "M" },
                    fainted: false,
                    brought: true,
                    volatile_data: { types: [] },
                    battle_appearances: [
                      {
                        inactive: {
                          health: { known: [100, 100] },
                          status: { known: "" },
                          ability: { known: "" },
                          item: { known: "" },
                          terastallization: { known: null },
                        },
                      },
                    ],
                  },
                  {
                    physical_appearance: { name: "Charizard", species: "Charizard", gender: "M" },
                    fainted: true,
                    brought: true,
                    volatile_data: { types: [] },
                    battle_appearances: [
                      {
                        inactive: {
                          health: { known: [0, 100] },
                          status: { known: "fnt" },
                          ability: { known: "" },
                          item: { known: "" },
                          terastallization: { known: null },
                        },
                      },
                    ],
                  },
                ],
              },
            },
          },
          {
            id: 1,
            name: "Side 2",
            conditions: {},
            players: {
              "player-2": {
                id: "player-2",
                name: "Bob",
                team_size: 3,
                mons: [
                  {
                    physical_appearance: { name: "Gengar", species: "Gengar", gender: "F" },
                    fainted: false,
                    brought: true,
                    volatile_data: { types: [] },
                    battle_appearances: [
                      {
                        inactive: {
                          health: { known: [80, 100] },
                          status: { known: "" },
                          ability: { known: "" },
                          item: { known: "" },
                          terastallization: { known: null },
                        },
                      },
                    ],
                  },
                ],
              },
            },
          },
        ],
      },
    } as unknown as BattleState;

    const html = renderToStaticMarkup(
      <PlayerStateViewer
        battleState={mockState}
        localPlayerId="player-1"
      />,
    );

    // Verify Sides and conditions (consolidated header in 1v1 shows player names and side conditions)
    expect(html).toContain("Stealth Rock");

    // Verify Players
    expect(html).toContain("Alice");
    expect(html).toContain("Bob");
    expect(html).not.toContain("@Alice");
    expect(html).not.toContain("@Bob");
    expect(html).toContain("You"); // Alice has You badge

    // Verify Mon
    expect(html).toContain("Pikachu");
    expect(html).toContain("Charizard");
    expect(html).toContain("Gengar");

    // Verify Unrevealed slot for Bob (team_size 3, only 1 revealed)
    expect(html).toContain("Unrevealed");
    expect(html).not.toContain("Not revealed");

    // Verify Score count (without "alive" suffix)
    expect(html).toContain("1/2");
    expect(html).not.toContain("1/2 alive");
    expect(html).toContain("3/3");
    expect(html).not.toContain("3/3 alive");
  });

  it("renders side title when side has multiple players", () => {
    const multiPlayerState = {
      field: {
        sides: [
          {
            id: 0,
            name: "Team Rocket",
            conditions: {},
            players: {
              "player-1": {
                id: "player-1",
                name: "Jessie",
                team_size: 1,
                mons: [],
              },
              "player-2": {
                id: "player-2",
                name: "James",
                team_size: 1,
                mons: [],
              },
            },
          },
        ],
      },
    } as unknown as BattleState;

    const html = renderToStaticMarkup(<PlayerStateViewer battleState={multiPlayerState} />);
    expect(html).toContain("Team Rocket");
    expect(html).toContain("Jessie");
    expect(html).toContain("James");
    expect(html).not.toContain("@Jessie");
    expect(html).not.toContain("@James");
  });

  it("orders players on a side by position rather than alphabetically", () => {
    const multiPlayerState = {
      field: {
        sides: [
          {
            id: 0,
            name: "Side 1",
            conditions: {},
            players: {
              "player-a": {
                id: "player-a",
                name: "Aaron",
                position: 1,
                team_size: 1,
                mons: [],
              },
              "player-z": {
                id: "player-z",
                name: "Zara",
                position: 0,
                team_size: 1,
                mons: [],
              },
            },
          },
        ],
      },
    } as unknown as BattleState;

    const html = renderToStaticMarkup(<PlayerStateViewer battleState={multiPlayerState} />);
    const zaraIndex = html.indexOf("Zara");
    const aaronIndex = html.indexOf("Aaron");
    expect(zaraIndex).toBeGreaterThan(-1);
    expect(aaronIndex).toBeGreaterThan(-1);
    expect(zaraIndex).toBeLessThan(aaronIndex);
  });

  it("renders private playerData when provided for local player", () => {
    const mockState = {
      field: {
        sides: [
          {
            id: 0,
            name: "Side 1",
            conditions: {},
            players: {
              "player-1": {
                id: "player-1",
                name: "Alice",
                team_size: 1,
                mons: [],
              },
            },
          },
        ],
      },
    } as unknown as BattleState;

    const mockPlayerData = {
      mons: [
        {
          name: "Sparky",
          summary: { level: 50 },
          hp: 120,
          max_hp: 120,
          status: null,
          active: true,
        },
      ],
    };

    const html = renderToStaticMarkup(
      <PlayerStateViewer
        battleState={mockState}
        playerData={mockPlayerData as any}
        localPlayerId="player-1"
      />,
    );

    expect(html).toContain("Alice");
    expect(html).not.toContain("@Alice");
    expect(html).toContain("Sparky");
    expect(html).toContain("120/120");
    expect(html).toContain("1/1");
    expect(html).not.toContain("1/1 alive");
  });

  it("renders (withdrew) when player has left the battle", () => {
    const mockState = {
      field: {
        sides: [
          {
            id: 0,
            name: "Side 1",
            conditions: {},
            players: {
              "player-1": {
                id: "player-1",
                name: "Alice",
                left_battle: true,
                team_size: 1,
                mons: [],
              },
            },
          },
        ],
      },
    } as unknown as BattleState;

    const html = renderToStaticMarkup(<PlayerStateViewer battleState={mockState} />);
    expect(html).toContain("(withdrew)");
    expect(html).not.toContain("Left");
  });

  it("renders ally player data with exact HP and without an Ally badge", () => {
    const multiPlayerState = {
      field: {
        sides: [
          {
            id: 0,
            name: "Side 1",
            conditions: {},
            players: {
              "player-1": {
                id: "player-1",
                name: "Alice",
                team_size: 1,
                mons: [],
              },
              "player-3": {
                id: "player-3",
                name: "Charlie",
                team_size: 1,
                mons: [],
              },
            },
          },
        ],
      },
    } as unknown as BattleState;

    const mockPlayerData = {
      mons: [
        {
          name: "AliceMon",
          summary: { level: 50 },
          hp: 150,
          max_hp: 150,
          status: null,
          active: true,
        },
      ],
    };

    const mockAllyPlayerData = {
      "player-3": {
        mons: [
          {
            name: "CharlieMon",
            summary: { level: 50 },
            hp: 220,
            max_hp: 220,
            status: null,
            active: false,
          },
        ],
      },
    };

    const html = renderToStaticMarkup(
      <PlayerStateViewer
        battleState={multiPlayerState}
        playerData={mockPlayerData as any}
        allyPlayerData={mockAllyPlayerData as any}
        localPlayerId="player-1"
      />,
    );

    // Alice is local player
    expect(html).toContain("Alice");
    expect(html).toContain("You");
    expect(html).toContain("AliceMon");
    expect(html).toContain("150/150");

    // Charlie is ally player
    expect(html).toContain("Charlie");
    expect(html).not.toContain("Ally");
    expect(html).toContain("CharlieMon");
    expect(html).toContain("220/220");
    expect(html).toContain("1/1");
  });

  it("renders all previewed mons in Pick 3 from 6 with faded unbrought styling and no bench/unrevealed cards", () => {
    const pickThreeState = {
      field: {
        sides: [
          {
            id: 0,
            name: "Side 1",
            conditions: {},
            players: {
              "player-1": {
                id: "player-1",
                name: "Trainer",
                team_size: 3,
                position: 0,
                mons: [
                  {
                    physical_appearance: { name: "Pikachu", species: "Pikachu", gender: "M" },
                    fainted: false,
                    brought: true,
                    volatile_data: { types: [] },
                    battle_appearances: [
                      {
                        inactive: {
                          health: { known: [100, 100] },
                          status: { known: "" },
                          ability: { known: "" },
                          item: { known: "" },
                          terastallization: { known: null },
                        },
                      },
                    ],
                  },
                  {
                    physical_appearance: { name: "Charmander", species: "Charmander", gender: "M" },
                    fainted: false,
                    brought: false,
                    volatile_data: { types: [] },
                    battle_appearances: [],
                  },
                  {
                    physical_appearance: { name: "Squirtle", species: "Squirtle", gender: "M" },
                    fainted: false,
                    brought: false,
                    volatile_data: { types: [] },
                    battle_appearances: [],
                  },
                  {
                    physical_appearance: { name: "Bulbasaur", species: "Bulbasaur", gender: "M" },
                    fainted: false,
                    brought: false,
                    volatile_data: { types: [] },
                    battle_appearances: [],
                  },
                  {
                    physical_appearance: { name: "Eevee", species: "Eevee", gender: "F" },
                    fainted: false,
                    brought: false,
                    volatile_data: { types: [] },
                    battle_appearances: [],
                  },
                  {
                    physical_appearance: { name: "Snorlax", species: "Snorlax", gender: "M" },
                    fainted: false,
                    brought: false,
                    volatile_data: { types: [] },
                    battle_appearances: [],
                  },
                ],
              },
            },
          },
        ],
      },
    } as unknown as BattleState;

    const html = renderToStaticMarkup(<PlayerStateViewer battleState={pickThreeState} />);

    // All 6 Mons are rendered in the roster
    expect(html).toContain("Pikachu");
    expect(html).toContain("Charmander");
    expect(html).toContain("Squirtle");
    expect(html).toContain("Bulbasaur");
    expect(html).toContain("Eevee");
    expect(html).toContain("Snorlax");

    // No dummy Unrevealed cards or Bench section
    expect(html).not.toContain("Unrevealed");
    expect(html).not.toContain("Bench");

    // Score reflects the 3 brought slots (3/3)
    expect(html).toContain("3/3");

    // Unbrought cards have summaryUnbrought
    expect(html).toContain("summaryUnbrought");
  });

  it("renders unbrought previewed Mons as disabled/faded when player has private playerData in Pick-3-from-6", () => {
    const pickThreeState = {
      field: {
        sides: [
          {
            id: 0,
            name: "Side 1",
            conditions: {},
            players: {
              ash: {
                id: "ash",
                name: "ash",
                team_size: 3,
                mons: [
                  {
                    physical_appearance: { name: "Zarude", species: "Zarude", gender: "N" },
                    fainted: false,
                    brought: true,
                    volatile_data: { types: [] },
                    battle_appearances: [
                      {
                        inactive: {
                          health: { known: [328, 328] },
                          status: { known: "" },
                          ability: { known: "" },
                          item: { known: "" },
                          terastallization: { known: null },
                        },
                      },
                    ],
                  },
                  {
                    physical_appearance: { name: "Great Tusk", species: "Great Tusk", gender: "N" },
                    fainted: false,
                    brought: false,
                    volatile_data: { types: [] },
                    battle_appearances: [],
                  },
                  {
                    physical_appearance: { name: "Cresselia", species: "Cresselia", gender: "F" },
                    fainted: false,
                    brought: false,
                    volatile_data: { types: [] },
                    battle_appearances: [],
                  },
                  {
                    physical_appearance: { name: "Walking Wake", species: "Walking Wake", gender: "N" },
                    fainted: false,
                    brought: false,
                    volatile_data: { types: [] },
                    battle_appearances: [],
                  },
                  {
                    physical_appearance: { name: "Ceruledge", species: "Ceruledge", gender: "M" },
                    fainted: false,
                    brought: false,
                    volatile_data: { types: [] },
                    battle_appearances: [],
                  },
                  {
                    physical_appearance: { name: "Ninetales", species: "Ninetales", gender: "F" },
                    fainted: false,
                    brought: false,
                    volatile_data: { types: [] },
                    battle_appearances: [],
                  },
                ],
              },
            },
          },
        ],
      },
    } as unknown as BattleState;

    const privatePlayerData = {
      name: "ash",
      id: "ash",
      mons: [
        {
          species: "Zarude",
          summary: { name: "Zarude", level: 50, gender: "N" },
          hp: 328,
          max_hp: 328,
          active: true,
          status: null,
        },
        {
          species: "Great Tusk",
          summary: { name: "Great Tusk", level: 50, gender: "N" },
          hp: 353,
          max_hp: 353,
          active: false,
          status: null,
        },
        {
          species: "Cresselia",
          summary: { name: "Cresselia", level: 50, gender: "F" },
          hp: 444,
          max_hp: 444,
          active: false,
          status: null,
        },
      ],
    };

    const html = renderToStaticMarkup(
      <PlayerStateViewer
        battleState={pickThreeState}
        playerData={privatePlayerData as any}
        localPlayerId="ash"
      />,
    );

    // All 6 Mons are rendered in the roster
    expect(html).toContain("Zarude");
    expect(html).toContain("Great Tusk");
    expect(html).toContain("Cresselia");
    expect(html).toContain("Walking Wake");
    expect(html).toContain("Ceruledge");
    expect(html).toContain("Ninetales");

    // Private HP values for brought Mons
    expect(html).toContain("328/328");
    expect(html).toContain("353/353");
    expect(html).toContain("444/444");

    // 100% placeholder HP for unbrought Mons
    expect(html).toContain("100%");

    // Score reflects the 3 brought slots (3/3)
    expect(html).toContain("3/3");

    // Unbrought cards have summaryUnbrought
    expect(html).toContain("summaryUnbrought");

    // Ash has You badge
    expect(html).toContain("You");
  });

  it("renders fainted Mon with 0% HP and FNT status even if battle appearance health is non-zero", () => {
    const mockState = {
      field: {
        sides: [
          {
            id: 0,
            name: "Side 1",
            conditions: {},
            players: {
              "player-1": {
                id: "player-1",
                name: "Alice",
                team_size: 1,
                mons: [
                  {
                    physical_appearance: { name: "Ninetales", species: "Ninetales", gender: "U" },
                    fainted: true,
                    brought: true,
                    volatile_data: { types: [] },
                    battle_appearances: [
                      {
                        inactive: {
                          health: { known: [335, 335] },
                          status: { known: "" },
                          ability: { known: "" },
                          item: { known: "" },
                          terastallization: { known: null },
                        },
                      },
                    ],
                  },
                ],
              },
            },
          },
        ],
      },
    } as unknown as BattleState;

    const html = renderToStaticMarkup(
      <PlayerStateViewer battleState={mockState} />,
    );

    expect(html).toContain("Ninetales");
    expect(html).toContain("0%");
    expect(html).toContain("FNT");
    expect(html).not.toContain("100%");
  });
});


