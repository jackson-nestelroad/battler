import autobahn from "autobahn";
import { EventEmitter } from "events";

export function uuidForUri(uuid: string): string {
  return uuid.replace(/-/g, "").toLowerCase();
}

interface InternalConnection extends autobahn.Connection {
  _session_close_reason?: string;
  _session_close_message?: string;
  _retry?: boolean;
  _transport?: { close: () => void };
}

export class WampSessionProvider extends EventEmitter {
  private connection: InternalConnection;
  private currentSession: autobahn.Session | null = null;
  private connectionPromise: Promise<autobahn.Session> | null = null;
  private isManualDisconnect = false;

  constructor(options: autobahn.IConnectionOptions) {
    super();
    this.connection = new autobahn.Connection(options);

    this.connection.onopen = (session) => {
      this.currentSession = session;
      session.onleave = (reason, details) => {
        this.connection._session_close_reason = reason;
        this.connection._session_close_message = details?.message || "";
        this.connection._retry = !this.isManualDisconnect;
        if (this.connection._transport) {
          this.connection._transport.close();
        }
      };
      this.emit("connect", session);
    };

    this.connection.onclose = (reason, details) => {
      this.currentSession = null;
      this.connectionPromise = null;
      this.emit("disconnect", reason, details);
      return this.isManualDisconnect;
    };
  }

  get session(): autobahn.Session | null {
    return this.currentSession;
  }

  async connect(): Promise<autobahn.Session> {
    this.isManualDisconnect = false;
    if (this.currentSession) {
      return this.currentSession;
    }
    if (this.connectionPromise) {
      return this.connectionPromise;
    }

    this.connectionPromise = new Promise<autobahn.Session>((resolve, reject) => {
      const onConnect = (session: autobahn.Session) => {
        this.off("error", onError);
        this.off("disconnect", onDisconnect);
        resolve(session);
      };
      const onError = (err: any) => {
        this.off("connect", onConnect);
        this.off("disconnect", onDisconnect);
        reject(err);
      };
      const onDisconnect = (reason: string, details?: any) => {
        if (details?.will_retry === false) {
          this.off("connect", onConnect);
          this.off("error", onError);
          this.off("disconnect", onDisconnect);
          reject(new Error(details?.message || reason || "Connection failed"));
        }
      };
      this.once("connect", onConnect);
      this.once("error", onError);
      this.on("disconnect", onDisconnect);
    });

    try {
      this.connection.open();
    } catch (err) {
      this.emit("error", err);
    }

    return this.connectionPromise;
  }

  async disconnect(): Promise<void> {
    this.isManualDisconnect = true;
    if (!this.currentSession && !this.connection.isOpen) {
      return;
    }
    const disconnectPromise = new Promise<void>((resolve) => {
      this.once("disconnect", () => resolve());
    });
    this.connection.close();
    this.currentSession = null;
    this.connectionPromise = null;
    await disconnectPromise;
  }
}

export function getWampResultString(res: unknown): string | null {
  if (res === null || res === undefined) return null;
  if (typeof res === "string") return res;
  if (typeof res === "object") {
    if ("battle_json" in res && typeof res.battle_json === "string") {
      return res.battle_json;
    }
    if ("json" in res && typeof res.json === "string") {
      return res.json;
    }
    if (Array.isArray(res)) {
      return res.length > 0 ? getWampResultString(res[0]) : null;
    }
    if ("args" in res && Array.isArray(res.args) && res.args.length > 0) {
      return getWampResultString(res.args[0]);
    }
  }
  return null;
}

export function getWampResultArray(res: unknown): unknown[] {
  if (!res) return [];
  if (Array.isArray(res)) {
    if (res.length === 1 && Array.isArray(res[0])) {
      return res[0];
    }
    return res;
  }
  if (typeof res === "object") {
    if ("args" in res && Array.isArray(res.args)) {
      return getWampResultArray(res.args[0]);
    }
  }
  return [];
}

export function getWampResultArguments(res: unknown): unknown[] {
  if (!res) return [];
  if (Array.isArray(res)) return res;
  if (typeof res === "object" && res !== null && "args" in res && Array.isArray(res.args)) {
    return res.args;
  }
  return [];
}

export function safeJsonStringify(value: unknown): string {
  return JSON.stringify(value, (_, v) => {
    if (typeof v === "bigint") {
      return Number(v);
    }
    return v;
  });
}
