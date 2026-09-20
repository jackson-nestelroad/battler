import { describe, expect, it } from "vitest";
import { normalizeWebSocketUrl } from "./url";

describe("normalizeWebSocketUrl", () => {
  it("returns empty string if input is empty", () => {
    expect(normalizeWebSocketUrl("")).toBe("");
    expect(normalizeWebSocketUrl("   ")).toBe("");
  });

  it("prepends wss:// to remote domains without scheme", () => {
    expect(normalizeWebSocketUrl("ws.battler.live", false)).toBe("wss://ws.battler.live");
    expect(normalizeWebSocketUrl("example.com/ws", false)).toBe("wss://example.com/ws");
  });

  it("prepends ws:// to localhost without scheme when on http:", () => {
    expect(normalizeWebSocketUrl("localhost:8080", false)).toBe("ws://localhost:8080");
    expect(normalizeWebSocketUrl("127.0.0.1:8080", false)).toBe("ws://127.0.0.1:8080");
    expect(normalizeWebSocketUrl("0.0.0.0:8080", false)).toBe("ws://0.0.0.0:8080");
  });

  it("preserves already valid ws:// and wss:// URLs on http:", () => {
    expect(normalizeWebSocketUrl("ws://localhost:8080", false)).toBe("ws://localhost:8080");
    expect(normalizeWebSocketUrl("wss://ws.battler.live", false)).toBe("wss://ws.battler.live");
  });

  it("upgrades ws:// to wss:// when running on https:", () => {
    expect(normalizeWebSocketUrl("ws://ws.battler.live", true)).toBe("wss://ws.battler.live");
    expect(normalizeWebSocketUrl("ws.battler.live", true)).toBe("wss://ws.battler.live");
    expect(normalizeWebSocketUrl("localhost:8080", true)).toBe("wss://localhost:8080");
    expect(normalizeWebSocketUrl("ws://localhost:8080", true)).toBe("wss://localhost:8080");
  });
});
