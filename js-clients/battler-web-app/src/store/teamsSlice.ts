import { createSlice } from "@reduxjs/toolkit";
import type { PayloadAction } from "@reduxjs/toolkit";
import type { MonData } from "battler-types";

export interface TeamsState {
  teams: Record<string, MonData[]>;
  defaultTeam: string | null;
  teamOrder: string[];
}

const DEFAULT_TEAM: Partial<MonData>[] = [
  {
    name: "Bulbasaur",
    species: "Bulbasaur",
    ability: "Overgrow",
    moves: ["Tackle", "Vine Whip", "Growl"],
    level: 50,
  },
  {
    name: "Charmander",
    species: "Charmander",
    ability: "Blaze",
    moves: ["Scratch", "Ember", "Growl"],
    level: 50,
  },
  {
    name: "Squirtle",
    species: "Squirtle",
    ability: "Torrent",
    moves: ["Tackle", "Water Gun", "Tail Whip"],
    level: 50,
  },
  {
    name: "Pikachu",
    species: "Pikachu",
    ability: "Static",
    moves: ["Thunder Shock", "Thunderbolt", "Quick Attack", "Growl"],
    level: 50,
  },
];

const initialState: TeamsState = {
  teams: {
    "Kanto Starters": DEFAULT_TEAM as MonData[],
  },
  defaultTeam: "Kanto Starters",
  teamOrder: ["Kanto Starters"],
};

const teamsSlice = createSlice({
  name: "teams",
  initialState,
  reducers: {
    teamsLoaded(
      state,
      action: PayloadAction<{
        teams: Record<string, MonData[]>;
        defaultTeam: string | null;
        teamOrder: string[];
      }>,
    ) {
      state.teams = action.payload.teams;
      state.defaultTeam = action.payload.defaultTeam;
      state.teamOrder = action.payload.teamOrder;
    },
    saveTeam(state, action: PayloadAction<{ name: string; members: MonData[] }>) {
      const { name, members } = action.payload;
      state.teams[name] = members;
      if (!state.teamOrder.includes(name)) {
        state.teamOrder.push(name);
      }
    },
    deleteTeam(state, action: PayloadAction<string>) {
      delete state.teams[action.payload];
      state.teamOrder = state.teamOrder.filter((name) => name !== action.payload);
      if (state.defaultTeam === action.payload) {
        state.defaultTeam = null;
      }
    },
    setDefaultTeam(state, action: PayloadAction<string | null>) {
      state.defaultTeam = action.payload;
    },
    moveTeamUp(state, action: PayloadAction<string>) {
      const index = state.teamOrder.indexOf(action.payload);
      if (index > 0) {
        state.teamOrder[index] = state.teamOrder[index - 1];
        state.teamOrder[index - 1] = action.payload;
      }
    },
    moveTeamDown(state, action: PayloadAction<string>) {
      const index = state.teamOrder.indexOf(action.payload);
      if (index >= 0 && index < state.teamOrder.length - 1) {
        state.teamOrder[index] = state.teamOrder[index + 1];
        state.teamOrder[index + 1] = action.payload;
      }
    },
  },
});

export const { teamsLoaded, saveTeam, deleteTeam, setDefaultTeam, moveTeamUp, moveTeamDown } =
  teamsSlice.actions;
export default teamsSlice.reducer;
