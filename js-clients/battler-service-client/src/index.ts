import autobahn from "autobahn";
import { Battle, BattlePreview, BattleServiceOptions, LogEntry } from "./bindings/index.js";

import {
  WampSessionProvider,
  getWampResultArguments,
  getWampResultArray,
  getWampResultString,
  safeJsonStringify,
  uuidForUri,
} from "battler-wamp-client";

export * from "battler-types";
export * from "./bindings/index.js";

import { CoreBattleOptions, PlayerBattleData, Request, TeamData } from "battler-types";
export type RequestType = Request["type"];

export class ValidationError extends Error {
  constructor(
    message: string,
    public problems: string[],
  ) {
    super(message);
    this.name = "ValidationError";
  }
}

export class BattlerServiceClient {
  constructor(private provider: WampSessionProvider) {}

  private get session(): autobahn.Session {
    const s = this.provider.session;
    if (!s) throw new Error("WAMP session is not connected");
    return s;
  }

  async create(options: CoreBattleOptions, serviceOptions: BattleServiceOptions): Promise<Battle> {
    const res = await this.session.call<unknown>("com.battler.battler_service.battles.create", [
      safeJsonStringify(options),
      safeJsonStringify(serviceOptions),
    ]);
    const json = getWampResultString(res);
    if (!json) throw new Error("Failed to get create response string");
    return JSON.parse(json);
  }

  async battles(count: number, offset: number): Promise<BattlePreview[]> {
    const res = await this.session.call<unknown>("com.battler.battler_service.battles", [
      count,
      offset,
    ]);
    const arr = getWampResultArray(res);
    return arr
      .map((item: unknown) => {
        const json = getWampResultString(item);
        return json ? JSON.parse(json) : null;
      })
      .filter(Boolean);
  }

  async battlesForPlayer(player: string, count: number, offset: number): Promise<BattlePreview[]> {
    const res = await this.session.call<unknown>("com.battler.battler_service.battles_for_player", [
      player,
      count,
      offset,
    ]);
    const arr = getWampResultArray(res);
    return arr
      .map((item: unknown) => {
        const json = getWampResultString(item);
        return json ? JSON.parse(json) : null;
      })
      .filter(Boolean);
  }

  async battle(battleId: string): Promise<Battle> {
    const res = await this.session.call<unknown>(
      `com.battler.battler_service.battles.${uuidForUri(battleId)}`,
    );
    const json = getWampResultString(res);
    if (!json) throw new Error("Failed to get battle response string");
    return JSON.parse(json);
  }

  async updateTeam(battleId: string, player: string, team: TeamData): Promise<void> {
    try {
      await this.session.call(
        `com.battler.battler_service.battles.${uuidForUri(battleId)}.update_team`,
        [player, safeJsonStringify(team)],
      );
    } catch (err: any) {
      if (err?.error === "com.battler.battler_service.error.validation_failed") {
        const problems = err.args?.map((arg: any) => String(arg)) || [];
        throw new ValidationError(err.message || "Validation failed", problems);
      }
      throw err;
    }
  }

  async start(battleId: string): Promise<void> {
    await this.session.call(`com.battler.battler_service.battles.${uuidForUri(battleId)}.start`);
  }

  async playerData(battleId: string, player: string): Promise<PlayerBattleData> {
    const res = await this.session.call<unknown>(
      `com.battler.battler_service.battles.${uuidForUri(battleId)}.player_data`,
      [player],
    );
    const json = getWampResultString(res);
    if (!json) throw new Error("Failed to get player data response string");
    return JSON.parse(json);
  }

  async request(battleId: string, player: string): Promise<Request | null> {
    const res = await this.session.call<unknown>(
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
    const res = await this.session.call<unknown>(
      `com.battler.battler_service.battles.${uuidForUri(battleId)}.full_log`,
      [side !== undefined ? side : null],
    );
    return getWampResultArray(res).map(String);
  }

  async lastLogEntry(battleId: string, side?: number): Promise<[number, string] | null> {
    const res = await this.session.call<unknown>(
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
    const handler = (args?: unknown[] | null) => {
      const arr = getWampResultArguments(args);
      if (arr.length >= 2) {
        onLogEntry({
          index: Number(arr[0]),
          content: String(arr[1]),
        });
      }
    };

    const selector = side !== undefined ? `${side}` : "public";
    const topic = `com.battler.battler_service.battles.${uuidForUri(battleId)}.log.${selector}`;

    return this.session.subscribe(topic, handler);
  }

  async unsubscribe(subscription: autobahn.Subscription): Promise<void> {
    if (!this.session || !("isOpen" in this.session && this.session.isOpen)) {
      return;
    }
    await this.session.unsubscribe(subscription).catch(() => {});
  }
}
