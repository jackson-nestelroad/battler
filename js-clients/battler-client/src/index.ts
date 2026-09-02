import autobahn from "autobahn";
import {
  Battle,
  BattlerServiceClient,
  LogEntry,
  PlayerBattleData,
  Request,
  TeamData,
} from "battler-service-client";
import { alterBattleState, BattleState, newBattleState } from "battler-state";
import { EventEmitter } from "events";

export type Role = { type: "spectator"; side: undefined } | { type: "player"; side: number };

export * from "./choice-builder.js";

export interface BattlerClient {
  on(event: "update", listener: () => void): this;
  on(event: "request", listener: (request: Request | null) => void): this;
  on(event: "end", listener: () => void): this;
  on(event: "deleted", listener: () => void): this;
  on(event: "error", listener: (err: unknown) => void): this;

  once(event: "update", listener: () => void): this;
  once(event: "request", listener: (request: Request | null) => void): this;
  once(event: "end", listener: () => void): this;
  once(event: "deleted", listener: () => void): this;
  once(event: "error", listener: (err: unknown) => void): this;

  off(event: "update", listener: () => void): this;
  off(event: "request", listener: (request: Request | null) => void): this;
  off(event: "end", listener: () => void): this;
  off(event: "deleted", listener: () => void): this;
  off(event: "error", listener: (err: unknown) => void): this;

  emit(event: "update"): boolean;
  emit(event: "request", request: Request | null): boolean;
  emit(event: "end"): boolean;
  emit(event: "deleted"): boolean;
  emit(event: "error", err: unknown): boolean;
}

export function getRoleForPlayer(battle: Battle, player: string): Role {
  for (let i = 0; i < battle.sides.length; i++) {
    const side = battle.sides[i];
    if (side.players.some((p) => p.id === player)) {
      return { type: "player", side: i };
    }
  }
  return { type: "spectator", side: undefined };
}

function isLogFilled(logLines: string[]): boolean {
  if (logLines.length === 0) return true;
  for (let i = 0; i < logLines.length; i++) {
    if (logLines[i] === undefined) {
      return false;
    }
  }
  return true;
}

async function backfillLog(
  logLines: string[],
  service: BattlerServiceClient,
  battleId: string,
  side?: number,
): Promise<void> {
  const fullLog = await service.fullLog(battleId, side);
  for (let i = 0; i < fullLog.length; i++) {
    logLines[i] = fullLog[i] ?? "";
  }
}

function updateBattleState(state: BattleState, logLines: string[]): BattleState {
  const lines = logLines.filter((l) => l !== undefined);
  return alterBattleState(newBattleState(), lines);
}

export class BattlerClient extends EventEmitter {
  private subscription?: autobahn.Subscription;
  private logLines: string[] = [];
  private currentBattleState: BattleState;
  private _role: Role;
  private isCanceled = false;
  private currentRequest: Request | null = null;
  private stateUpdatePromise: Promise<void> | null = null;
  private hasDoneSignal = false;
  private hasEmittedEndSignal = false;
  private hasDeletedSignal = false;
  private hasPendingRequestSignal = false;

  private constructor(
    public readonly battleId: string,
    public readonly player: string,
    private readonly service: BattlerServiceClient,
    role: Role,
    initialLogLines: string[],
  ) {
    super();
    this._role = role;
    this.logLines = initialLogLines;
    this.currentBattleState = newBattleState();
    this.currentBattleState = updateBattleState(this.currentBattleState, this.logLines);
  }

  static async create(
    battleId: string,
    player: string,
    service: BattlerServiceClient,
  ): Promise<BattlerClient> {
    const battle = await service.battle(battleId);
    const role = getRoleForPlayer(battle, player);
    const initialLogLines = await service.fullLog(battleId, role.side);

    const client = new BattlerClient(battleId, player, service, role, initialLogLines);
    await client.init();
    return client;
  }

  private async init(): Promise<void> {
    const side = this._role.side;
    this.subscription = await this.service.subscribe(this.battleId, side, (entry) => {
      this.processLogEntry(entry).catch((err) => {
        this.emit("error", err);
      });
    });
    await this.ensureCaughtUp();
    await this.checkAndEmitRequest();
  }

  private async processLogEntry(entry: LogEntry): Promise<void> {
    if (this.isCanceled) return;

    this.logLines[entry.index] = entry.content;

    if (entry?.content === "-battlerservice:request") {
      this.hasPendingRequestSignal = true;
    }

    if (entry.content === "-battlerservice:done") {
      this.hasDoneSignal = true;
    }

    if (entry.content === "-battlerservice:deleted") {
      this.hasDeletedSignal = true;
    }

    if (!this.stateUpdatePromise) {
      this.stateUpdatePromise = Promise.resolve().then(async () => {
        this.stateUpdatePromise = null;
        try {
          await this.flushStateUpdate();
        } catch (err) {
          this.emit("error", err);
        }
      });
    }
  }

  private async flushStateUpdate(): Promise<void> {
    if (this.isCanceled) return;

    if (!isLogFilled(this.logLines)) {
      await backfillLog(this.logLines, this.service, this.battleId, this._role.side);
      if (!isLogFilled(this.logLines)) {
        throw new Error("Failed to backfill missing battle log entries");
      }
      if (this.logLines.some((l) => l === "-battlerservice:request")) {
        this.hasPendingRequestSignal = true;
      }
      if (this.logLines.some((l) => l === "-battlerservice:deleted")) {
        this.hasDeletedSignal = true;
      }
    }

    this.currentBattleState = updateBattleState(this.currentBattleState, this.logLines);
    this.emit("update");
    if (this.hasPendingRequestSignal) {
      this.hasPendingRequestSignal = false;
      this.checkAndEmitRequest().catch(() => {});
    }

    if (this.hasDoneSignal && !this.hasEmittedEndSignal) {
      this.hasEmittedEndSignal = true;
      this.emit("end");
    }

    if (this.hasDeletedSignal) {
      this.emit("deleted");
      await this.cancel();
      return;
    }
  }

  private async ensureCaughtUp(): Promise<void> {
    await backfillLog(this.logLines, this.service, this.battleId, this._role.side);
    this.currentBattleState = updateBattleState(this.currentBattleState, this.logLines);
    this.emit("update");
  }

  private async checkAndEmitRequest(): Promise<void> {
    if (this._role.type === "spectator") return;
    const request = await this.fetchRequest();
    this.emit("request", request);
  }

  async sync(): Promise<void> {
    if (this.subscription) {
      try {
        await this.service.unsubscribe(this.subscription);
      } catch (e) {
        // Ignore unsubscribe errors if previous session died
      }
      this.subscription = undefined;
    }
    const side = this._role.side;
    this.subscription = await this.service.subscribe(this.battleId, side, (entry) => {
      this.processLogEntry(entry).catch((err) => {
        this.emit("error", err);
      });
    });
    await this.ensureCaughtUp();
    await this.checkAndEmitRequest();
  }

  getLogs(): string[] {
    return [...this.logLines];
  }

  async updateTeam(team: TeamData): Promise<void> {
    await this.service.updateTeam(this.battleId, this.player, team);
  }

  async start(): Promise<void> {
    await this.service.start(this.battleId);
  }

  async makeChoice(choice: string): Promise<void> {
    await this.service.makeChoice(this.battleId, this.player, choice);
    this.currentRequest = null;
  }

  async playerData(): Promise<PlayerBattleData> {
    return this.service.playerData(this.battleId, this.player);
  }

  async fetchRequest(): Promise<Request | null> {
    if (this._role.type === "spectator") return null;
    try {
      const request = await this.service.request(this.battleId, this.player);
      this.currentRequest = request;
      return request;
    } catch (err) {
      return null;
    }
  }

  request(): Request | null {
    return this.currentRequest;
  }

  state(): BattleState {
    return this.currentBattleState;
  }

  role(): Role {
    return this._role;
  }

  lastLogIndex(): number {
    return Number(this.currentBattleState.last_log_index || 0);
  }

  async cancel(): Promise<void> {
    this.isCanceled = true;
    if (this.subscription) {
      await this.service.unsubscribe(this.subscription);
      this.subscription = undefined;
    }
  }
}
