import autobahn from "autobahn";
import { BattlerClient } from "battler-client";
import {
  BattlerMultiplayerServiceClient,
  ProposedBattle,
  ProposedBattleOptions,
  ProposedBattleUpdate,
} from "battler-multiplayer-service-client";
import { BattlerServiceClient, TeamData } from "battler-service-client";

export class BattlerMultiplayerClient {
  constructor(
    public readonly player: string,
    private readonly multiplayerService: BattlerMultiplayerServiceClient,
    private readonly service: BattlerServiceClient,
  ) {}

  async proposeBattle(options: ProposedBattleOptions): Promise<ProposedBattle> {
    return this.multiplayerService.proposeBattle(options);
  }

  async proposedBattle(proposedBattleId: string): Promise<ProposedBattle> {
    return this.multiplayerService.proposedBattle(proposedBattleId);
  }

  async respondToProposal(
    proposedBattleId: string,
    accept: boolean,
    team?: TeamData,
  ): Promise<ProposedBattle> {
    const result = await this.multiplayerService.respondToProposedBattle(
      proposedBattleId,
      this.player,
      {
        accept,
      },
    );
    if (accept && team) {
      const battleId = result.battle ?? (await this.waitForBattleStart(proposedBattleId));
      await this.service.updateTeam(battleId, this.player, team);
    }
    return result;
  }

  async proposedBattles(count: number, offset: number): Promise<ProposedBattle[]> {
    return this.multiplayerService.proposedBattlesForPlayer(this.player, count, offset);
  }

  async proposedBattleUpdates(
    onUpdate: (update: ProposedBattleUpdate) => void,
  ): Promise<autobahn.Subscription> {
    return this.multiplayerService.proposedBattleUpdates(this.player, onUpdate);
  }

  async waitForBattleStart(proposedBattleId: string): Promise<string> {
    const initial = await this.multiplayerService
      .proposedBattle(proposedBattleId)
      .catch(() => null);
    if (initial && initial.battle) {
      return initial.battle;
    }

    return new Promise<string>(async (resolve, reject) => {
      let sub: autobahn.Subscription | undefined;
      let resolved = false;
      const cleanup = () => {
        if (sub) {
          this.multiplayerService.unsubscribe(sub).catch(() => {});
        }
      };

      try {
        sub = await this.proposedBattleUpdates((update) => {
          if (resolved) return;
          if (update.proposed_battle.uuid === proposedBattleId) {
            if (update.proposed_battle.battle) {
              resolved = true;
              cleanup();
              resolve(update.proposed_battle.battle);
            } else if (update.deletion_reason === "fulfilled" && update.proposed_battle.battle) {
              resolved = true;
              cleanup();
              resolve(update.proposed_battle.battle);
            } else if (
              update.rejection ||
              (update.deletion_reason && update.deletion_reason !== "fulfilled")
            ) {
              resolved = true;
              cleanup();
              reject(
                new Error(
                  update.deletion_reason || "proposed battle proposal was rejected or cancelled",
                ),
              );
            }
          }
        });

        // Re-check proposed battle after subscription setup to ensure updates during setup aren't missed
        const check = await this.multiplayerService
          .proposedBattle(proposedBattleId)
          .catch(() => null);
        if (check && check.battle && !resolved) {
          resolved = true;
          cleanup();
          resolve(check.battle);
        }
      } catch (err) {
        if (!resolved) {
          cleanup();
          reject(err);
        }
      }
    });
  }

  async createBattlerClient(battleId: string): Promise<BattlerClient> {
    return BattlerClient.create(battleId, this.player, this.service);
  }

  async proposeAndWaitForBattleStart(options: ProposedBattleOptions): Promise<BattlerClient> {
    const proposed = await this.proposeBattle(options);
    if (proposed.battle) {
      return this.createBattlerClient(proposed.battle);
    }
    const battleId = await this.waitForBattleStart(proposed.uuid);
    return this.createBattlerClient(battleId);
  }
}

export { ProposedBattle, ProposedBattleOptions, ProposedBattleUpdate };
