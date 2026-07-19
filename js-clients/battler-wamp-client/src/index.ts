import autobahn from "autobahn";
import { EventEmitter } from "events";

export function uuidForUri(uuid: string): string {
  return uuid.replace(/-/g, "").toLowerCase();
}

export class WampSessionProvider extends EventEmitter {
  private connection: autobahn.Connection;
  private currentSession: autobahn.Session | null = null;
  private connectionPromise: Promise<autobahn.Session> | null = null;
  private isManualDisconnect = false;

  constructor(options: autobahn.IConnectionOptions) {
    super();
    this.connection = new autobahn.Connection(options);

    this.connection.onopen = (session) => {
      this.currentSession = session;
      session.onleave = (reason, details) => {
        (this.connection as any)._session_close_reason = reason;
        (this.connection as any)._session_close_message = details?.message || "";
        (this.connection as any)._retry = !this.isManualDisconnect;
        if ((this.connection as any)._transport) {
          (this.connection as any)._transport.close();
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
        resolve(session);
      };
      const onError = (err: any) => {
        this.off("connect", onConnect);
        reject(err);
      };
      this.once("connect", onConnect);
      this.once("error", onError);
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
    this.connection.close();
    this.currentSession = null;
    this.connectionPromise = null;
  }
}

export function getWampResultString(res: any): string | null {
  if (res === null || res === undefined) return null;
  if (typeof res === "string") return res;
  if (typeof res === "object") {
    if (res.battle_json && typeof res.battle_json === "string") {
      return res.battle_json;
    }
    if (res.json && typeof res.json === "string") {
      return res.json;
    }
    if (Array.isArray(res)) {
      return res.length > 0 ? getWampResultString(res[0]) : null;
    }
    if (res.args && Array.isArray(res.args) && res.args.length > 0) {
      return getWampResultString(res.args[0]);
    }
  }
  return null;
}

export function getWampResultArray(res: any): any[] {
  if (!res) return [];
  if (Array.isArray(res)) {
    if (res.length === 1 && Array.isArray(res[0])) {
      return res[0];
    }
    return res;
  }
  if (typeof res === "object") {
    if (res.args && Array.isArray(res.args)) {
      return getWampResultArray(res.args[0]);
    }
  }
  return [];
}

export function getWampResultArguments(res: any): any[] {
  if (!res) return [];
  if (Array.isArray(res)) return res;
  if (typeof res === "object" && res.args && Array.isArray(res.args)) {
    return res.args;
  }
  return [];
}

export function safeJsonStringify(value: any): string {
  return JSON.stringify(value, (_, v) => {
    if (typeof v === "bigint") {
      return Number(v);
    }
    return v;
  });
}
