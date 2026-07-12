import autobahn from "autobahn";
import {
  Battle,
  BattlePreview,
  PlayerValidation,
  LogEntry,
  BattleServiceOptions,
} from "./bindings/index.js";

import { WampSessionProvider, uuidForUri } from "battler-wamp-client";

export * from "battler-types";
export * from "./bindings/index.js";

import { Request, CoreBattleOptions, TeamData, PlayerBattleData } from "battler-types";
export type RequestType = Request["type"];

export class BattlerServiceClient {
  constructor(private provider: WampSessionProvider) {}

  private get session(): autobahn.Session {
    const s = this.provider.session;
    if (!s) throw new Error("WAMP session is not connected");
    return s;
  }

  async create(options: CoreBattleOptions, serviceOptions: BattleServiceOptions): Promise<Battle> {
    const res = await this.session.call<any>("com.battler.battler_service.battles.create", [
      JSON.stringify(options),
      JSON.stringify(serviceOptions),
    ]);
    const json = typeof res === "string" ? res : res[0];
    return JSON.parse(json);
  }

  async battles(count: number, offset: number): Promise<BattlePreview[]> {
    const res = await this.session.call<any>("com.battler.battler_service.battles", [
      count,
      offset,
    ]);
    const arr = Array.isArray(res) ? (Array.isArray(res[0]) ? res[0] : res) : [];
    return arr.map((item: any) => {
      const json = Array.isArray(item) ? item[0] : item;
      return JSON.parse(json);
    });
  }

  async battlesForPlayer(player: string, count: number, offset: number): Promise<BattlePreview[]> {
    const res = await this.session.call<any>("com.battler.battler_service.battles_for_player", [
      player,
      count,
      offset,
    ]);
    const arr = Array.isArray(res) ? (Array.isArray(res[0]) ? res[0] : res) : [];
    return arr.map((item: any) => {
      const json = Array.isArray(item) ? item[0] : item;
      return JSON.parse(json);
    });
  }

  async battle(battleId: string): Promise<Battle> {
    const res = await this.session.call<any>(
      `com.battler.battler_service.battles.${uuidForUri(battleId)}`,
    );
    const json = typeof res === "string" ? res : res[0];
    return JSON.parse(json);
  }

  async updateTeam(battleId: string, player: string, team: TeamData): Promise<void> {
    await this.session.call(
      `com.battler.battler_service.battles.${uuidForUri(battleId)}.update_team`,
      [player, JSON.stringify(team)],
    );
  }

  async validatePlayer(battleId: string, player: string): Promise<PlayerValidation> {
    const res = await this.session.call<any>(
      `com.battler.battler_service.battles.${uuidForUri(battleId)}.validate_player`,
      [player],
    );
    const problems = Array.isArray(res) ? (Array.isArray(res[0]) ? res[0] : res) : [];
    return { problems };
  }

  async start(battleId: string): Promise<void> {
    await this.session.call(`com.battler.battler_service.battles.${uuidForUri(battleId)}.start`);
  }

  async playerData(battleId: string, player: string): Promise<PlayerBattleData> {
    const res = await this.session.call<any>(
      `com.battler.battler_service.battles.${uuidForUri(battleId)}.player_data`,
      [player],
    );
    const json = typeof res === "string" ? res : res[0];
    return JSON.parse(json);
  }

  async request(battleId: string, player: string): Promise<Request | null> {
    const res = await this.session.call<any>(
      `com.battler.battler_service.battles.${uuidForUri(battleId)}.request`,
      [player],
    );
    const json = typeof res === "string" ? res : res && res[0] ? res[0] : null;
    return json ? JSON.parse(json) : null;
  }

  async makeChoice(battleId: string, player: string, choice: string): Promise<void> {
    await this.session.call(
      `com.battler.battler_service.battles.${uuidForUri(battleId)}.make_choice`,
      [player, choice],
    );
  }

  async delete(battleId: string): Promise<void> {
    await this.session.call(`com.battler.battler_service.battles.${uuidForUri(battleId)}.delete`);
  }

  async fullLog(battleId: string, side?: number): Promise<string[]> {
    const res = await this.session.call<any>(
      `com.battler.battler_service.battles.${uuidForUri(battleId)}.full_log`,
      [side !== undefined ? side : null],
    );
    const log = Array.isArray(res) ? (Array.isArray(res[0]) ? res[0] : res) : [];
    return log;
  }

  async lastLogEntry(battleId: string, side?: number): Promise<[number, string] | null> {
    const res = await this.session.call<any>(
      `com.battler.battler_service.battles.${uuidForUri(battleId)}.last_log_entry`,
      [side !== undefined ? side : null],
    );
    if (!res) return null;
    if (Array.isArray(res)) {
      if (Array.isArray(res[0])) {
        if (res[0].length < 2) return null;
        return [Number(res[0][0]), String(res[0][1])];
      }
      if (res.length < 2) return null;
      return [Number(res[0]), String(res[1])];
    }
    return null;
  }

  async subscribe(
    battleId: string,
    side: number | undefined,
    onLogEntry: (entry: LogEntry) => void,
  ): Promise<autobahn.Subscription> {
    const selector = side !== undefined ? side.toString() : "public";
    const topic = `com.battler.battler_service.battles.${uuidForUri(battleId)}.log.${selector}`;

    const handler = (args?: any[] | null) => {
      if (!args || args.length === 0) return;
      if (Array.isArray(args[0]) && args[0].length >= 2) {
        onLogEntry({
          index: Number(args[0][0]),
          content: String(args[0][1]),
        });
      } else if (args.length >= 2) {
        onLogEntry({
          index: Number(args[0]),
          content: String(args[1]),
        });
      }
    };

    return this.session.subscribe(topic, handler);
  }

  async unsubscribe(subscription: autobahn.Subscription): Promise<void> {
    await this.session.unsubscribe(subscription);
  }
}
