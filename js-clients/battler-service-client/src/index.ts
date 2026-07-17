import autobahn from "autobahn";
import {
  Battle,
  BattlePreview,
  PlayerValidation,
  LogEntry,
  BattleServiceOptions,
} from "./bindings/index.js";

import {
  WampSessionProvider,
  uuidForUri,
  getWampResultString,
  getWampResultArray,
  getWampResultArguments,
  safeJsonStringify,
} from "battler-wamp-client";

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
      safeJsonStringify(options),
      safeJsonStringify(serviceOptions),
    ]);
    const json = getWampResultString(res);
    if (!json) throw new Error("Failed to get create response string");
    return JSON.parse(json);
  }

  async battles(count: number, offset: number): Promise<BattlePreview[]> {
    const res = await this.session.call<any>("com.battler.battler_service.battles", [
      count,
      offset,
    ]);
    const arr = getWampResultArray(res);
    return arr
      .map((item: any) => {
        const json = getWampResultString(item);
        return json ? JSON.parse(json) : null;
      })
      .filter(Boolean);
  }

  async battlesForPlayer(player: string, count: number, offset: number): Promise<BattlePreview[]> {
    const res = await this.session.call<any>("com.battler.battler_service.battles_for_player", [
      player,
      count,
      offset,
    ]);
    const arr = getWampResultArray(res);
    return arr
      .map((item: any) => {
        const json = getWampResultString(item);
        return json ? JSON.parse(json) : null;
      })
      .filter(Boolean);
  }

  async battle(battleId: string): Promise<Battle> {
    const res = await this.session.call<any>(
      `com.battler.battler_service.battles.${uuidForUri(battleId)}`,
    );
    const json = getWampResultString(res);
    if (!json) throw new Error("Failed to get battle response string");
    return JSON.parse(json);
  }

  async updateTeam(battleId: string, player: string, team: TeamData): Promise<void> {
    await this.session.call(
      `com.battler.battler_service.battles.${uuidForUri(battleId)}.update_team`,
      [player, safeJsonStringify(team)],
    );
  }

  async validatePlayer(battleId: string, player: string): Promise<PlayerValidation> {
    const res = await this.session.call<any>(
      `com.battler.battler_service.battles.${uuidForUri(battleId)}.validate_player`,
      [player],
    );
    const problems = getWampResultArray(res);
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
    const json = getWampResultString(res);
    if (!json) throw new Error("Failed to get player data response string");
    return JSON.parse(json);
  }

  async request(battleId: string, player: string): Promise<Request | null> {
    const res = await this.session.call<any>(
      `com.battler.battler_service.battles.${uuidForUri(battleId)}.request`,
      [player],
    );
    const json = getWampResultString(res);
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
    return getWampResultArray(res);
  }

  async lastLogEntry(battleId: string, side?: number): Promise<[number, string] | null> {
    const res = await this.session.call<any>(
      `com.battler.battler_service.battles.${uuidForUri(battleId)}.last_log_entry`,
      [side !== undefined ? side : null],
    );
    const arr = getWampResultArguments(res);
    if (arr.length < 2 || arr[0] === null || arr[0] === undefined) return null;
    return [Number(arr[0]), String(arr[1])];
  }

  async subscribe(
    battleId: string,
    side: number | undefined,
    onLogEntry: (entry: LogEntry) => void,
  ): Promise<autobahn.Subscription> {
    const selector = side !== undefined ? side.toString() : "public";
    const topic = `com.battler.battler_service.battles.${uuidForUri(battleId)}.log.${selector}`;

    const handler = (args?: any[] | null) => {
      const arr = getWampResultArray(args);
      if (arr.length >= 2) {
        onLogEntry({
          index: Number(arr[0]),
          content: String(arr[1]),
        });
      }
    };

    return this.session.subscribe(topic, handler);
  }

  async unsubscribe(subscription: autobahn.Subscription): Promise<void> {
    await this.session.unsubscribe(subscription);
  }
}
