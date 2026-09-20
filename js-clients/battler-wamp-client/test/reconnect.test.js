import assert from "node:assert";
import { test } from "node:test";
import { WampSessionProvider } from "../dist/index.js";

test("WampSessionProvider fast first retry sets delay to 0 and primes backoff to initial_retry_delay (1.5s)", () => {
  const provider = new WampSessionProvider({
    url: "ws://localhost:9999",
    realm: "com.battler",
  });

  const conn = provider.connection;
  assert.ok(typeof conn._autoreconnect_advance === "function");
  assert.strictEqual(conn._initial_retry_delay, 1.5);

  conn._retry = true;

  // Attempt 1: instant (delay: 0)
  const retry1 = conn._autoreconnect_advance();
  assert.strictEqual(retry1.count, 1);
  assert.strictEqual(retry1.delay, 0);
  assert.strictEqual(retry1.will_retry, true);

  // Verify internal fallback state: _retry_delay MUST be primed to 1.5s (not 0)
  // to avoid the Autobahn jitter bug and exponential backoff collapse
  assert.strictEqual(conn._retry_delay, 1.5);

  // Attempt 2: Autobahn's native math runs with healthy mean=1.5 and sd=0.15
  const retry2 = conn._autoreconnect_advance();
  assert.strictEqual(retry2.count, 2);
  assert.ok(
    retry2.delay >= 1.0 && retry2.delay <= 2.0,
    `Second retry delay (${retry2.delay}) should be centered around 1.5s with jitter`,
  );
  assert.strictEqual(retry2.will_retry, true);

  // Attempt 3: Exponential backoff increases (~2.25s)
  const retry3 = conn._autoreconnect_advance();
  assert.strictEqual(retry3.count, 3);
  assert.ok(
    retry3.delay > 1.8 && retry3.delay < 3.0,
    `Third retry delay (${retry3.delay}) should reflect exponential backoff (~2.25s)`,
  );
  assert.strictEqual(retry3.will_retry, true);

  // After reconnect reset, first retry is instant again and primes 1.5s again
  conn._autoreconnect_reset();
  conn._retry = true;
  const resetRetry1 = conn._autoreconnect_advance();
  assert.strictEqual(resetRetry1.count, 1);
  assert.strictEqual(resetRetry1.delay, 0);
  assert.strictEqual(conn._retry_delay, 1.5);
});

test("WampSessionProvider preserves custom initial_retry_delay when priming fallback", () => {
  const provider = new WampSessionProvider({
    url: "ws://localhost:9999",
    realm: "com.battler",
    initial_retry_delay: 3.0,
  });

  const conn = provider.connection;
  conn._retry = true;

  // Attempt 1 is instant
  const retry1 = conn._autoreconnect_advance();
  assert.strictEqual(retry1.delay, 0);

  // _retry_delay should be primed to custom 3.0s, not hardcoded 1.5s
  assert.strictEqual(conn._retry_delay, 3.0);

  // Attempt 2 should be centered around 3.0s
  const retry2 = conn._autoreconnect_advance();
  assert.ok(
    retry2.delay >= 2.2 && retry2.delay <= 3.8,
    `Second retry delay (${retry2.delay}) should be centered around custom initial delay 3.0s`,
  );
});

test("WampSessionProvider respects fastFirstRetry: false option", () => {
  const provider = new WampSessionProvider({
    url: "ws://localhost:9999",
    realm: "com.battler",
    fastFirstRetry: false,
  });

  const conn = provider.connection;
  conn._retry = true;
  const retry1 = conn._autoreconnect_advance();
  assert.strictEqual(retry1.count, 1);
  assert.ok(retry1.delay > 0, "When fastFirstRetry is false, first retry should have standard delay");
});

