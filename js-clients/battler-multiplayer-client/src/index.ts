import autobahn from "autobahn";
import {
  BattlerMultiplayerServiceClient,
  ProposedBattle,
  ProposedBattleOptions,
  ProposedBattleUpdate,
} from "battler-multiplayer-service-client";
import { BattlerServiceClient } from "battler-service-client";
import { BattlerClient } from "battler-client";

export class BattlerMultiplayerClient {
  constructor(
    public readonly player: string,
    private readonly multiplayerService: BattlerMultiplayerServiceClient,
    private readonly service: BattlerServiceClient,
  ) {}

  async proposeBattle(options: ProposedBattleOptions): Promise<ProposedBattle> {
    return this.multiplayerService.proposeBattle(options);
  }

  async respondToProposal(proposedBattleId: string, accept: boolean): Promise<ProposedBattle> {
    return this.multiplayerService.respondToProposedBattle(proposedBattleId, this.player, {
      accept,
    });
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
    return new Promise<string>(async (resolve, reject) => {
      let subscription: autobahn.Subscription | undefined;
      try {
        subscription = await this.proposedBattleUpdates((update) => {
          if (update.proposed_battle.uuid === proposedBattleId) {
            if (update.proposed_battle.battle) {
              if (subscription) {
                this.multiplayerService.unsubscribe(subscription).catch(() => {});
              }
              resolve(update.proposed_battle.battle);
            } else if (update.rejection || update.deletion_reason) {
              if (subscription) {
                this.multiplayerService.unsubscribe(subscription).catch(() => {});
              }
              reject(
                new Error(
                  update.deletion_reason || "proposed battle proposal was rejected or cancelled",
                ),
              );
            }
          }
        });
      } catch (err) {
        reject(err);
      }
    });
  }

  async createBattlerClient(battleId: string): Promise<BattlerClient> {
    return BattlerClient.create(battleId, this.player, this.service);
  }

  async proposeAndWaitForBattleStart(options: ProposedBattleOptions): Promise<BattlerClient> {
    const proposed = await this.proposeBattle(options);
    const startPromise = this.waitForBattleStart(proposed.uuid);
    const battleId = await startPromise;
    return this.createBattlerClient(battleId);
  }
}

export { ProposedBattle, ProposedBattleOptions, ProposedBattleUpdate };
