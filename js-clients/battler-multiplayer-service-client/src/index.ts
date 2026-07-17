import autobahn from "autobahn";
import {
  ProposedBattleOptions,
  ProposedBattle,
  ProposedBattleResponse,
  ProposedBattleUpdate,
} from "./bindings/index.js";

import {
  WampSessionProvider,
  uuidForUri,
  getWampResultString,
  getWampResultArray,
  safeJsonStringify,
} from "battler-wamp-client";

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
      [safeJsonStringify(options)],
    );
    const json = getWampResultString(res);
    if (!json) throw new Error("Invalid WAMP response");
    return JSON.parse(json);
  }

  async proposedBattle(proposedBattleId: string): Promise<ProposedBattle> {
    const res = await this.session.call<any>(
      `com.battler.battler_multiplayer_service.proposed_battles.${uuidForUri(proposedBattleId)}`,
    );
    const json = getWampResultString(res);
    if (!json) throw new Error("Invalid WAMP response");
    return JSON.parse(json);
  }

  async respondToProposedBattle(
    proposedBattleId: string,
    player: string,
    response: ProposedBattleResponse,
  ): Promise<ProposedBattle> {
    const res = await this.session.call<any>(
      `com.battler.battler_multiplayer_service.proposed_battles.${uuidForUri(proposedBattleId)}.respond`,
      [player, safeJsonStringify(response)],
    );
    const json = getWampResultString(res);
    if (!json) throw new Error("Invalid WAMP response");
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
    const arr = getWampResultArray(res);
    return arr
      .map((item: any) => {
        const json = getWampResultString(item);
        return json ? JSON.parse(json) : null;
      })
      .filter(Boolean);
  }

  async proposedBattleUpdates(
    player: string,
    onUpdate: (update: ProposedBattleUpdate) => void,
  ): Promise<autobahn.Subscription> {
    const topic = `com.battler.battler_multiplayer_service.proposed_battle_updates.${player}`;
    const handler = (args?: any[] | null) => {
      const json = getWampResultString(args);
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
