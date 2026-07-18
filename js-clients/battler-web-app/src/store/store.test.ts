import { describe, it, expect, vi, beforeEach } from "vitest";
import { store, hydrateStore } from "./store";
import { saveTeam } from "./teamsSlice";
import type { MonData } from "battler-types";

describe("teams persistence", () => {
  let localStorageMock: Record<string, string> = {};

  beforeEach(() => {
    localStorageMock = {};
    vi.stubGlobal("window", {
      localStorage: {
        getItem: vi.fn((key: string) => localStorageMock[key] || null),
        setItem: vi.fn((key: string, value: string) => {
          localStorageMock[key] = value;
        }),
        removeItem: vi.fn((key: string) => {
          delete localStorageMock[key];
        }),
      },
    });
  });

  it("should not save default teams to storage on startup", async () => {
    expect(localStorageMock["battler_teams"]).toBeUndefined();
  });

  it("should hydrate teams from storage and save updates", async () => {
    const testTeams = {
      "My Test Team": [
        {
          name: "Pikachu",
          species: "Pikachu",
          ability: "Static",
          moves: ["Thunderbolt"],
          level: 50,
        },
      ],
    };
    localStorageMock["battler_teams"] = JSON.stringify(testTeams);
    localStorageMock["battler_default_team"] = JSON.stringify("My Test Team");
    localStorageMock["battler_team_order"] = JSON.stringify(["My Test Team"]);

    await store.dispatch(hydrateStore());

    const state = store.getState();
    expect(state.teams.teams).toEqual(testTeams);
    expect(state.teams.defaultTeam).toBe("My Test Team");

    const updatedTeam = [
      {
        name: "Raichu",
        species: "Raichu",
        ability: "Lightning Rod",
        moves: ["Thunderbolt"],
        level: 50,
      },
    ];
    store.dispatch(
      saveTeam({ name: "My Test Team", members: updatedTeam as unknown as MonData[] }),
    );

    // Wait a brief moment for promises in middleware to resolve
    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(JSON.parse(localStorageMock["battler_teams"])).toEqual({
      "My Test Team": updatedTeam,
    });
  });
});
