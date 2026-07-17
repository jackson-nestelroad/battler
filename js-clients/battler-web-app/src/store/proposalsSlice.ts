import { createSlice } from "@reduxjs/toolkit";
import type { PayloadAction } from "@reduxjs/toolkit";
import type { ProposedBattle, ProposedBattleRejection } from "battler-multiplayer-service-client";
import type { CoreBattleOptions } from "battler-types";

export interface ProposedBattleWithDetails extends ProposedBattle {
  rejection?: ProposedBattleRejection | null;
  deletionReason?: string | null;
  battle_options?: CoreBattleOptions;
}

export interface ProposalsState {
  proposals: Record<string, ProposedBattleWithDetails>;
}

const initialState: ProposalsState = {
  proposals: {},
};

const proposalsSlice = createSlice({
  name: "proposals",
  initialState,
  reducers: {
    updateProposal(state, action: PayloadAction<ProposedBattleWithDetails>) {
      state.proposals[action.payload.uuid] = action.payload;
    },
    addProposals(state, action: PayloadAction<ProposedBattle[]>) {
      for (const p of action.payload) {
        state.proposals[p.uuid] = p;
      }
    },
    removeProposal(state, action: PayloadAction<string>) {
      delete state.proposals[action.payload];
    },
    clearProposals(state) {
      state.proposals = {};
    },
  },
});

export const { updateProposal, addProposals, removeProposal, clearProposals } =
  proposalsSlice.actions;
export default proposalsSlice.reducer;
