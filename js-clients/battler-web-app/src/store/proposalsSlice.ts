import { createSlice } from "@reduxjs/toolkit";
import type { PayloadAction } from "@reduxjs/toolkit";
import type { ProposedBattle, ProposedBattleRejection } from "battler-multiplayer-service-client";
import type { CoreBattleOptions } from "battler-types";
import { formatUuid } from "../utils/uuid";

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

const normalizeId = (id: string): string => formatUuid(id);

const proposalsSlice = createSlice({
  name: "proposals",
  initialState,
  reducers: {
    updateProposal(state, action: PayloadAction<ProposedBattleWithDetails>) {
      const normalizedUuid = normalizeId(action.payload.uuid);
      const normalizedBattle = action.payload.battle
        ? normalizeId(action.payload.battle)
        : undefined;
      state.proposals[normalizedUuid] = {
        ...action.payload,
        uuid: normalizedUuid,
        battle: normalizedBattle,
      };
    },
    addProposals(state, action: PayloadAction<ProposedBattle[]>) {
      for (const p of action.payload) {
        const normalizedUuid = normalizeId(p.uuid);
        const normalizedBattle = p.battle ? normalizeId(p.battle) : undefined;
        state.proposals[normalizedUuid] = {
          ...p,
          uuid: normalizedUuid,
          battle: normalizedBattle,
        };
      }
    },
    removeProposal(state, action: PayloadAction<string>) {
      const normalizedUuid = normalizeId(action.payload);
      delete state.proposals[normalizedUuid];
    },
    clearProposals(state) {
      state.proposals = {};
    },
  },
});

export const { updateProposal, addProposals, removeProposal, clearProposals } =
  proposalsSlice.actions;
export default proposalsSlice.reducer;
