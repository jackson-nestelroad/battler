import { describe, expect, it } from "vitest";
import { hasBooleanSearchParam, normalizeWebSocketUrl } from "./url";

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

describe("hasBooleanSearchParam", () => {
  it("detects boolean search parameters in various query string formats", () => {
    expect(hasBooleanSearchParam("all", "?all")).toBe(true);
    expect(hasBooleanSearchParam("all", "?all=true")).toBe(true);
    expect(hasBooleanSearchParam("all", "?all=1")).toBe(true);
    expect(hasBooleanSearchParam("all", "?foo=bar&all")).toBe(true);
    expect(hasBooleanSearchParam("all", "?all&foo=bar")).toBe(true);
    expect(hasBooleanSearchParam("debug", "?debug=yes")).toBe(true);
  });

  it("returns false when parameter is missing or explicitly disabled", () => {
    expect(hasBooleanSearchParam("all", "")).toBe(false);
    expect(hasBooleanSearchParam("all", "?")).toBe(false);
    expect(hasBooleanSearchParam("all", "?foo=bar")).toBe(false);
    expect(hasBooleanSearchParam("all", "?all=false")).toBe(false);
    expect(hasBooleanSearchParam("all", "?all=0")).toBe(false);
  });
});
