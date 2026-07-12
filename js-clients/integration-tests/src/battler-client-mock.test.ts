import { test, describe } from "node:test";
import * as assert from "node:assert";
import { BattlerClient } from "battler-client";
import { LogEntry } from "battler-service-client";

class MockBattlerServiceClient {
  public subscribeCallback?: (entry: LogEntry) => void;
  public logsReturned: (string | null)[] = [];
  public currentRequest: any = null;

  async battle(battleId: string) {
    return {
      uuid: battleId,
      sides: [
        { name: "Side 1", players: [{ id: "player-1", name: "Player 1" }] },
        { name: "Side 2", players: [{ id: "player-2", name: "Player 2" }] },
      ],
    };
  }

  async fullLog(battleId: string, side?: number) {
    return this.logsReturned.filter((x): x is string => x !== null);
  }

  async subscribe(battleId: string, side: number | undefined, callback: (entry: LogEntry) => void) {
    this.subscribeCallback = callback;
    return { id: 1 } as any;
  }

  async unsubscribe(subscription: any) {}

  async request(battleId: string, player: string) {
    return this.currentRequest;
  }

  async lastLogEntry(battleId: string, side?: number) {
    return [this.logsReturned.length - 1, ""] as [number, string];
  }
}

const startLogs = [
  "info|battletype:singles",
  "side|id:0|name:Side 1",
  "side|id:1|name:Side 2",
  "maxsidelength|length:1",
  "player|id:player-1|name:Player 1|side:0|position:0",
  "player|id:player-2|name:Player 2|side:1|position:0",
  "teamsize|player:player-1|size:3",
  "teamsize|player:player-2|size:3",
  "battlestart",
  "turn|turn:1",
];

describe("BattlerClient Mock Tests", () => {
  test("recovers cleanly when log packets arrive out of order (gap detection)", async () => {
    const mockService = new MockBattlerServiceClient();
    mockService.logsReturned = [...startLogs];

    const client = await BattlerClient.create("test-battle-uuid", "player-1", mockService as any);

    assert.strictEqual(client.state().turn, 1);

    // Mock fullLog to return the caught up logs including index startLogs.length and index startLogs.length + 1
    mockService.logsReturned = [...startLogs, "turn|turn:2", "turn|turn:2"];

    let errorEmitted = false;
    client.on("error", (err) => {
      errorEmitted = true;
      console.error("Client error event emitted in test 1:", err);
    });

    // Simulate gap: push index startLogs.length + 1, skipping index startLogs.length
    // We wait briefly for the asynchronous processLogEntry to complete after we call the callback
    await mockService.subscribeCallback!({
      index: startLogs.length + 1,
      content: "turn|turn:2",
    });

    // Give a short delay for asynchronous log processing
    await new Promise((r) => setTimeout(r, 100));

    assert.strictEqual(errorEmitted, false, "No errors should be emitted");
    assert.strictEqual(
      client.state().turn,
      2,
      "State should catch up to turn 2 after gap recovery",
    );
  });

  test("emits error events when log processing or subscriber fails", async () => {
    const mockService = new MockBattlerServiceClient();
    mockService.logsReturned = [...startLogs];

    const client = await BattlerClient.create("test-battle-uuid", "player-1", mockService as any);

    const errorPromise = new Promise<any>((resolve) => {
      client.on("error", (err) => {
        resolve(err);
      });
    });

    // Cause gap recovery to fail by returning empty logs (meaning index startLogs.length is undefined)
    mockService.logsReturned = [];

    mockService.subscribeCallback!({
      index: startLogs.length + 1,
      content: "turn|turn:2",
    });

    const errorFired = await errorPromise;
    assert.ok(errorFired, "error event should have been emitted on processing failure");
  });
});
