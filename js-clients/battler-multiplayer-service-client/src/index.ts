import autobahn from "autobahn";
import {
  ProposedBattleOptions,
  ProposedBattle,
  ProposedBattleResponse,
  ProposedBattleUpdate,
} from "./bindings/index.js";

import { WampSessionProvider, uuidForUri } from "battler-wamp-client";

export * from "battler-types";
export * from "./bindings/index.js";

export class BattlerMultiplayerServiceClient {
  constructor(private provider: WampSessionProvider) {}

  private get session(): autobahn.Session {
    const s = this.provider.session;
    if (!s) throw new Error("WAMP session is not connected");
    return s;
  }

  async proposeBattle(options: ProposedBattleOptions): Promise<ProposedBattle> {
    const res = await this.session.call<any>(
      "com.battler.battler_multiplayer_service.proposed_battles.create",
      [JSON.stringify(options)],
    );
    const json = typeof res === "string" ? res : res[0];
    return JSON.parse(json);
  }

  async proposedBattle(proposedBattleId: string): Promise<ProposedBattle> {
    const res = await this.session.call<any>(
      `com.battler.battler_multiplayer_service.proposed_battles.${uuidForUri(proposedBattleId)}`,
    );
    const json = typeof res === "string" ? res : res[0];
    return JSON.parse(json);
  }

  async respondToProposedBattle(
    proposedBattleId: string,
    player: string,
    response: ProposedBattleResponse,
  ): Promise<ProposedBattle> {
    const res = await this.session.call<any>(
      `com.battler.battler_multiplayer_service.proposed_battles.${uuidForUri(proposedBattleId)}.respond`,
      [player, JSON.stringify(response)],
    );
    const json = typeof res === "string" ? res : res[0];
    return JSON.parse(json);
  }

  async proposedBattlesForPlayer(
    player: string,
    count: number,
    offset: number,
  ): Promise<ProposedBattle[]> {
    const res = await this.session.call<any>(
      "com.battler.battler_multiplayer_service.proposed_battles_for_player",
      [player, count, offset],
    );
    const arr = Array.isArray(res) ? (Array.isArray(res[0]) ? res[0] : res) : [];
    return arr.map((item: any) => {
      const json = Array.isArray(item) ? item[0] : item;
      return JSON.parse(json);
    });
  }

  async proposedBattleUpdates(
    player: string,
    onUpdate: (update: ProposedBattleUpdate) => void,
  ): Promise<autobahn.Subscription> {
    const topic = `com.battler.battler_multiplayer_service.proposed_battle_updates.${player}`;
    const handler = (args?: any[] | null) => {
      if (!args || args.length === 0) return;
      const json = Array.isArray(args[0]) ? args[0][0] : args[0];
      if (json) {
        onUpdate(JSON.parse(json));
      }
    };
    return this.session.subscribe(topic, handler);
  }

  async unsubscribe(subscription: autobahn.Subscription): Promise<void> {
    await this.session.unsubscribe(subscription);
  }
}
