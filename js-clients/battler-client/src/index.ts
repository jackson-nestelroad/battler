import { EventEmitter } from "events";
import autobahn from "autobahn";
import { newBattleState, alterBattleState, BattleState } from "battler-state";
import {
  BattlerServiceClient,
  Battle,
  Request,
  LogEntry,
  PlayerValidation,
  TeamData,
  PlayerBattleData,
} from "battler-service-client";

export type Role = { type: "spectator"; side: undefined } | { type: "player"; side: number };

export * from "./choice-builder.js";

export interface BattlerClient {
  on(event: "update", listener: () => void): this;
  on(event: "request", listener: (request: Request | null) => void): this;
  on(event: "end", listener: () => void): this;
  on(event: "error", listener: (err: any) => void): this;

  once(event: "update", listener: () => void): this;
  once(event: "request", listener: (request: Request | null) => void): this;
  once(event: "end", listener: () => void): this;
  once(event: "error", listener: (err: any) => void): this;

  off(event: "update", listener: () => void): this;
  off(event: "request", listener: (request: Request | null) => void): this;
  off(event: "end", listener: () => void): this;
  off(event: "error", listener: (err: any) => void): this;
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
  const lines: string[] = [];
  for (let i = 0; i < logLines.length; i++) {
    lines.push(logLines[i] ?? "");
  }
  return alterBattleState(state, lines);
}

export class BattlerClient extends EventEmitter {
  private subscription?: autobahn.Subscription;
  private logLines: string[] = [];
  private currentBattleState: BattleState;
  private _role: Role;
  private isCanceled = false;
  private lastEmittedRequest: string | null = null;
  private currentRequest: Request | null = null;
  private stateUpdatePromise: Promise<void> | null = null;
  private hasDoneSignal = false;
  private safetyEndTimeout: any = null;

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

    if (entry.content === "-battlerservice:done") {
      this.hasDoneSignal = true;
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
    }

    this.currentBattleState = updateBattleState(this.currentBattleState, this.logLines);
    this.emit("update");

    if (this.hasDoneSignal) {
      this.emit("end");
      await this.cancel();
      return;
    }

    if (this.currentBattleState.phase === "finished" && !this.safetyEndTimeout) {
      this.safetyEndTimeout = setTimeout(() => {
        if (!this.isCanceled) {
          console.warn("[BattlerClient] Safety timeout triggered: forcing end unsubscription");
          this.emit("end");
          this.cancel().catch((err) => this.emit("error", err));
        }
      }, 5000);
    }

    const lastLogIndex = this.lastLogIndex();
    const lastEntry = await this.service.lastLogEntry(this.battleId, this._role.side);
    const caughtUp = lastLogIndex === (lastEntry ? lastEntry[0] : 0);
    if (caughtUp && lastLogIndex > 0) {
      await this.checkAndEmitRequest();
    }
  }

  private async ensureCaughtUp(): Promise<void> {
    await backfillLog(this.logLines, this.service, this.battleId, this._role.side);
    this.currentBattleState = updateBattleState(this.currentBattleState, this.logLines);
    this.emit("update");
  }

  private async checkAndEmitRequest(): Promise<void> {
    if (this._role.type === "spectator") return;
    const request = await this.service.request(this.battleId, this.player);
    this.currentRequest = request;
    const requestStr = JSON.stringify(request);
    if (requestStr !== this.lastEmittedRequest) {
      this.lastEmittedRequest = requestStr;
      this.emit("request", request);
    }
  }

  async sync(): Promise<void> {
    await this.ensureCaughtUp();
    this.lastEmittedRequest = null;
    await this.checkAndEmitRequest();
  }

  getLogs(): string[] {
    return [...this.logLines];
  }

  async readyForBattle(): Promise<PlayerValidation> {
    return this.service.validatePlayer(this.battleId, this.player);
  }

  async updateTeam(team: TeamData): Promise<void> {
    await this.service.updateTeam(this.battleId, this.player, team);
  }

  async start(): Promise<void> {
    await this.service.start(this.battleId);
  }

  async makeChoice(choice: string): Promise<void> {
    await this.service.makeChoice(this.battleId, this.player, choice);
    this.lastEmittedRequest = null;
  }

  async playerData(): Promise<PlayerBattleData> {
    return this.service.playerData(this.battleId, this.player);
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
    if (this.safetyEndTimeout) {
      clearTimeout(this.safetyEndTimeout);
      this.safetyEndTimeout = null;
    }
    if (this.subscription) {
      await this.service.unsubscribe(this.subscription);
      this.subscription = undefined;
    }
  }
}
