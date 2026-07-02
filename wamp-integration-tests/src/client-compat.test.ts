import { test, describe, before, after, afterEach } from "node:test";
import * as assert from "node:assert";
import { spawn, execSync, ChildProcess } from "child_process";
import * as path from "path";
import * as readline from "readline";
import { fileURLToPath } from "url";
import * as autobahn from "autobahn";
import * as fs from "fs";
import * as net from "net";

// Monkeypatch Autobahn's Session prototype to negotiate call_timeout and call_canceling caller features.
const originalJoin = (autobahn as any).Session.prototype.join;
(autobahn as any).Session.prototype.join = function (
  realm: any,
  authmethods: any,
  authid: any,
  authextra: any,
) {
  const self = this;
  const originalSendWamp = self._send_wamp;
  self._send_wamp = function (msg: any) {
    if (msg && msg[0] === 1) {
      // HELLO
      const details = msg[2] || {};
      if (details.roles) {
        if (details.roles.caller) {
          if (!details.roles.caller.features) details.roles.caller.features = {};
          details.roles.caller.features.call_timeout = true;
          details.roles.caller.features.call_canceling = true;
        }
      }
    }
    return originalSendWamp.call(this, msg);
  };
  return originalJoin.call(this, realm, authmethods, authid, authextra);
};

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Helper to wait for a port to be ready
function waitForPort(port: number, timeoutMs = 15000): Promise<void> {
  const startTime = Date.now();
  return new Promise((resolve, reject) => {
    const check = () => {
      const socket = new net.Socket();
      socket.setTimeout(500);
      socket.on("connect", () => {
        socket.destroy();
        resolve();
      });
      socket.on("error", () => {
        socket.destroy();
        if (Date.now() - startTime > timeoutMs) {
          reject(new Error(`Timed out waiting for port ${port}`));
        } else {
          setTimeout(check, 100);
        }
      });
      socket.on("timeout", () => {
        socket.destroy();
        if (Date.now() - startTime > timeoutMs) {
          reject(new Error(`Timed out waiting for port ${port}`));
        } else {
          setTimeout(check, 100);
        }
      });
      socket.connect(port, "127.0.0.1");
    };
    check();
  });
}

// Helper to spawn the Rust client scenario
function runRustClient(
  port: number,
  scenario: string,
  extraArgs: string[] = [],
): {
  process: ChildProcess;
  lines: Promise<string[]>;
  waitForLine: (pat: RegExp | string) => Promise<string>;
} {
  const binPath = path.resolve(__dirname, "../../target/debug/battler-wamp-compat-test-bin");
  const child = spawn(binPath, [
    "client",
    "--url",
    `ws://127.0.0.1:${port}`,
    "--realm",
    "com.compat.test",
    "--scenario",
    scenario,
    ...extraArgs,
  ]);

  const rl = readline.createInterface({ input: child.stdout });
  const collected: string[] = [];
  const lineCallbacks: Array<{ pat: RegExp | string; resolve: (val: string) => void }> = [];

  const matchLine = (pat: RegExp | string, line: string): boolean => {
    if (pat instanceof RegExp) {
      return pat.test(line);
    } else {
      return line.includes(pat);
    }
  };

  rl.on("line", (line) => {
    console.log(`[CLIENT-${scenario}] ${line}`);
    collected.push(line);
    // Check callbacks
    for (let i = lineCallbacks.length - 1; i >= 0; i--) {
      const cb = lineCallbacks[i];
      if (matchLine(cb.pat, line)) {
        cb.resolve(line);
        lineCallbacks.splice(i, 1);
      }
    }
  });

  child.stderr.on("data", (data) => {
    console.error(`[CLIENT-${scenario} ERR] ${data}`);
  });

  const linesPromise = new Promise<string[]>((resolve) => {
    child.on("close", () => resolve(collected));
  });

  const waitForLine = (pat: RegExp | string): Promise<string> => {
    // First check if already collected
    for (const line of collected) {
      if (matchLine(pat, line)) return Promise.resolve(line);
    }
    return new Promise<string>((resolve) => {
      lineCallbacks.push({ pat, resolve });
    });
  };

  return { process: child, lines: linesPromise, waitForLine };
}

describe("WAMP Client Compatibility Tests", () => {
  let server: any;
  let controlClient: autobahn.Connection;
  let serverPort: number;
  let controlSessionDetails: any = null;
  const controlRegistrations: any[] = [];

  before(async () => {
    serverPort = 9001;

    // Start Crossbar locally
    const crossbarBin = path.resolve(__dirname, "../venv/bin/crossbar");
    const cbDir = path.resolve(__dirname, "../.crossbar");
    const configPath = path.resolve(__dirname, "../.crossbar/config.json");
    server = spawn(crossbarBin, ["start", "--cbdir", cbDir, "--config", configPath], {
      detached: true,
    });

    server.stdout.on("data", (data: any) => {
      console.log(`[CROSSBAR] ${data.toString().trim()}`);
    });

    server.stderr.on("data", (data: any) => {
      console.error(`[CROSSBAR ERR] ${data.toString().trim()}`);
    });

    // Wait for port 9001 to open
    await waitForPort(serverPort);

    // Connect control client
    controlClient = new autobahn.Connection({
      url: `ws://127.0.0.1:${serverPort}`,
      realm: "com.compat.test",
      max_retries: 5,
      authid: "test-user",
    });

    await new Promise<void>((resolve, reject) => {
      controlClient.onopen = (session, details) => {
        controlSessionDetails = details;
        resolve();
      };
      controlClient.onclose = (reason) => {
        reject(new Error(`Control client closed: ${reason}`));
        return false;
      };
      controlClient.open();
    });
  });

  after(async () => {
    if (controlClient) {
      controlClient.close();
    }
    if (server) {
      const exitPromise = new Promise<void>((resolve) => {
        server.on("exit", () => resolve());
      });
      try {
        process.kill(-server.pid!, "SIGTERM");
      } catch (e) {
        server.kill("SIGTERM");
      }
      const timeout = setTimeout(() => {
        try {
          process.kill(-server.pid!, "SIGKILL");
        } catch (e) {
          server.kill("SIGKILL");
        }
      }, 1500);
      await exitPromise;
      clearTimeout(timeout);
    }
  });

  afterEach(async () => {
    if (controlClient && controlClient.session) {
      while (controlRegistrations.length > 0) {
        const reg = controlRegistrations.pop();
        try {
          await controlClient.session.unregister(reg);
        } catch (e) {}
      }
    }
    await new Promise((r) => setTimeout(r, 50));
  });

  test("T1.2: Joining Invalid Realm (Client Check)", async () => {
    const client = runRustClient(serverPort, "client-invalid-realm");
    const lines = await client.lines;
    assert.ok(lines.some((l) => l.includes("SUCCESS: rejected with error")));
  });

  test("T1.7: MessagePack Serializer Handshake (Client Scenario)", async () => {
    const client = runRustClient(serverPort, "pubsub", ["--serializer", "msgpack"]);
    const lines = await client.lines;
    assert.ok(lines.includes("SUBSCRIBED"));
  });

  test("T2.2: WAMP-SCRAM Successful Authentication (Client Scenario)", async () => {
    const client = runRustClient(serverPort, "pubsub", [
      "--auth-id",
      "test-user",
      "--auth-secret",
      "password123!",
    ]);
    const lines = await client.lines;
    assert.ok(lines.includes("SUBSCRIBED"));
    assert.ok(lines.includes("PUBLISHED"));
  });

  test("T3.1: Pub/Sub Basic (Client Scenario)", async () => {
    const client = runRustClient(serverPort, "pubsub");
    const lines = await client.lines;
    assert.ok(lines.includes("SUBSCRIBED"));
    assert.ok(lines.includes("PUBLISHED"));
    assert.ok(lines.includes("UNSUBSCRIBED"));
  });

  test("T3.2: Pub/Sub Basic (TS Subscriber, Rust Publisher)", async () => {
    // Subscribe from the TS control client before the Rust client runs
    const receivedArgs: any[] = [];
    const receivedKwargs: any[] = [];
    const sub = await controlClient.session!.subscribe("com.compat.topic", (args, kwargs) => {
      receivedArgs.push(...(args ?? []));
      receivedKwargs.push(kwargs ?? {});
    });

    // Rust pubsub scenario publishes [123, "test"] with kwargs {foo: "bar"}
    const client = runRustClient(serverPort, "pubsub");
    await client.waitForLine("PUBLISHED");

    // Allow the event to propagate
    await new Promise((r) => setTimeout(r, 200));

    assert.strictEqual(receivedArgs[0], 123);
    assert.strictEqual(receivedArgs[1], "test");
    assert.strictEqual(receivedKwargs[0]?.foo, "bar");

    await controlClient.session!.unsubscribe(sub);
    await client.lines;
  });

  test("T3.3: Prefix-based Subscription Matching (Client Scenario)", async () => {
    const client = runRustClient(serverPort, "pubsub-prefix");
    await client.waitForLine("SUBSCRIBED");

    // Control client publishes to subtopic
    controlClient.session!.publish("com.compat.prefix_subtopic", [42]);

    const lines = await client.lines;
    assert.ok(
      lines.some(
        (l) => l.includes("EVENT:") && l.includes("42") && l.includes("com.compat.prefix_subtopic"),
      ),
    );
  });

  test("T3.4: Wildcard-based Subscription Matching (Client Scenario)", async () => {
    const client = runRustClient(serverPort, "pubsub-wildcard");
    await client.waitForLine("SUBSCRIBED");

    // Control client publishes to wildcard topic
    controlClient.session!.publish("com.compat.foo.status", [100]);

    const lines = await client.lines;
    assert.ok(
      lines.some(
        (l) => l.includes("EVENT:") && l.includes("100") && l.includes("com.compat.foo.status"),
      ),
    );
  });

  test("T3.6: Pattern-Based Pub/Sub (Subscriber Client Scenario)", async () => {
    const client = runRustClient(serverPort, "pubsub-subscriber-pattern");
    await client.waitForLine("SUBSCRIBED_WILDCARD");
    await client.waitForLine("SUBSCRIBED_PREFIX");

    // Publish to wildcard
    await controlClient.session!.publish("com.compat.pattern.test.topic", ["wildcard-payload"]);
    // Publish to prefix
    await controlClient.session!.publish("com.compat.pattern.prefix.sub", ["prefix-payload"]);

    const lines = await client.lines;
    assert.ok(
      lines.some(
        (l) =>
          l.includes("WILDCARD_EVENT: com.compat.pattern.test.topic") &&
          l.includes("wildcard-payload"),
      ),
    );
    assert.ok(
      lines.some(
        (l) =>
          l.includes("PREFIX_EVENT: com.compat.pattern.prefix.sub") && l.includes("prefix-payload"),
      ),
    );
  });

  test("T3.7: Publisher Identification (Subscriber Client Scenario)", async () => {
    const client = runRustClient(serverPort, "pubsub-subscriber-disclose");
    await client.waitForLine("SUBSCRIBED");

    // Publish with disclose_me: true
    await controlClient.session!.publish("com.compat.pub_disclose", [], {}, { disclose_me: true });

    const lines = await client.lines;
    const line = lines.find((l) => l.includes("EVENT_RECEIVED:"));
    assert.ok(line);
    assert.match(line, /publisher: Integer\(\d+\)/);
    assert.match(line, new RegExp('authid: String\\("' + controlSessionDetails?.authid + '"\\)'));
  });

  test("T4.1: Basic RPC (Callee Client Scenario)", async () => {
    const calleeA = runRustClient(serverPort, "rpc-callee");
    await calleeA.waitForLine("REGISTERED");

    // Control client calls the Rust procedure
    const res = await controlClient.session!.call("com.compat.proc", [12]);
    assert.strictEqual(res, 24); // Callee scenario doubles the value

    const linesA = await calleeA.lines;
    assert.ok(linesA.includes("RESPONDED"));
    assert.ok(linesA.includes("UNREGISTERED"));

    // Control client registers the procedure
    const reg = await controlClient.session!.register("com.compat.proc", (args) => {
      return (args?.[0] as number) * 3;
    });
    controlRegistrations.push(reg);

    const callerB = runRustClient(serverPort, "rpc-caller");
    const linesB = await callerB.lines;
    assert.ok(linesB.some((l) => l.includes("RESULT:") && l.includes("30"))); // 10 * 3

    await controlClient.session!.unregister(reg);
  });

  test("T4.2: RPC Error Propagation (TS Callee, Rust Caller)", async () => {
    // Control client registers a procedure that throws a WAMP application error
    const reg = await controlClient.session!.register("com.compat.error_proc", () => {
      throw new (autobahn as any).Error("com.compat.error.custom", ["ts error payload"]);
    });
    controlRegistrations.push(reg);

    const client = runRustClient(serverPort, "rpc-caller-error");
    const lines = await client.lines;
    assert.ok(
      lines.some((l) => l.includes("CALL_ERROR")),
      "Rust caller must receive the WAMP error thrown by the TS callee",
    );
  });

  test("T4.3: Unregistering a Procedure (Client Scenario)", async () => {
    const client = runRustClient(serverPort, "rpc-unregister");
    const lines = await client.lines;
    assert.ok(lines.includes("REGISTERED"));
    assert.ok(lines.includes("UNREGISTERED"));
  });

  test("T4.4: RPC Error Propagation (Callee Client Scenario)", async () => {
    const client = runRustClient(serverPort, "rpc-error");
    await client.waitForLine("REGISTERED");

    await assert.rejects(
      Promise.resolve(controlClient.session!.call("com.compat.error_proc")),
      (err: any) => {
        assert.strictEqual(err.error, "com.compat.error.custom");
        assert.strictEqual(err.args[0], "custom callee error payload");
        return true;
      },
    );

    await client.lines;
  });

  test("T4.5: Shared Registration (Client Scenario)", async () => {
    // --- Single policy: only one callee may register ---

    // Client A registers com.compat.shared with the default single policy
    const singleA = runRustClient(serverPort, "rpc-shared");
    await singleA.waitForLine("REGISTERED_SHARED");

    // Client B attempts to register the same URI under single policy - should fail
    const singleB = runRustClient(serverPort, "rpc-shared", ["--auth-secret", "policy:single"]);
    const linesSingleB = await singleB.lines;
    assert.ok(
      linesSingleB.some((l) => l.includes("REGISTER_FAILED")),
      "Second single-policy registration should be rejected",
    );

    // Control client calls the procedure - client A should respond with its client name
    const singleResult = (await controlClient.session!.call("com.compat.shared")) as string;
    assert.strictEqual(singleResult, "rust-client-rpc-shared");

    await singleA.lines;

    // --- Round-robin policy: multiple callees may register ---

    // Both clients register com.compat.shared with roundrobin policy
    const rrA = runRustClient(serverPort, "rpc-shared", ["--auth-secret", "policy:roundrobin"]);
    const rrB = runRustClient(serverPort, "rpc-shared", ["--auth-secret", "policy:roundrobin"]);

    // Wait for both to be registered before calling
    await Promise.all([rrA.waitForLine("REGISTERED_SHARED"), rrB.waitForLine("REGISTERED_SHARED")]);

    // First call: race to see which client prints RESPONDED first
    const rrResult1 = (await controlClient.session!.call("com.compat.shared")) as string;
    assert.strictEqual(rrResult1, "rust-client-rpc-shared");

    const firstResponded = await Promise.race([
      rrA.waitForLine("RESPONDED").then(() => "A" as const),
      rrB.waitForLine("RESPONDED").then(() => "B" as const),
    ]);

    // Second call: the other client must respond (round-robin)
    const rrResult2 = (await controlClient.session!.call("com.compat.shared")) as string;
    assert.strictEqual(rrResult2, "rust-client-rpc-shared");

    // Only wait on the client that hasn't responded yet
    const secondResponded =
      firstResponded === "A"
        ? await rrB.waitForLine("RESPONDED").then(() => "B" as const)
        : await rrA.waitForLine("RESPONDED").then(() => "A" as const);

    assert.notStrictEqual(
      firstResponded,
      secondResponded,
      "Round-robin must dispatch each call to a different callee",
    );

    await Promise.all([rrA.lines, rrB.lines]);
  });

  test("T4.6: RPC Pattern Matching (Callee Client Scenario)", async () => {
    const client = runRustClient(serverPort, "rpc-callee-pattern");
    await client.waitForLine("REGISTERED_WILDCARD");
    await client.waitForLine("REGISTERED_PREFIX");

    // Call wildcard matching procedure
    const resWildcard = await controlClient.session!.call("com.compat.pattern.foo.match");
    assert.strictEqual(resWildcard, "com.compat.pattern.foo.match");

    // Call prefix matching procedure
    const resPrefix = await controlClient.session!.call("com.compat.pattern.prefix.bar");
    assert.strictEqual(resPrefix, "com.compat.pattern.prefix.bar");

    const lines = await client.lines;
    assert.ok(lines.includes("WILDCARD_INVOCATED: com.compat.pattern.foo.match"));
    assert.ok(lines.includes("PREFIX_INVOCATED: com.compat.pattern.prefix.bar"));
  });

  test("T4.7: Caller Identification", async (t) => {
    await t.test("Callee Client Scenario", async () => {
      const client = runRustClient(serverPort, "rpc-disclose-caller");
      await client.waitForLine("REGISTERED");

      // Control client calls procedure disclosing caller identity
      const res = await controlClient.session!.call(
        "com.compat.disclose",
        [],
        {},
        { disclose_me: true },
      );
      assert.strictEqual(res, controlSessionDetails.authid);

      await client.lines;
    });

    await t.test("Caller Client Scenario", async () => {
      let callerAuthid: string | null = null;
      let callerSession: number | null = null;
      const reg = await controlClient.session!.register(
        "com.compat.caller_disclose",
        (args, kwargs, details) => {
          callerAuthid = (details as any)?.caller_authid || null;
          callerSession = (details as any)?.caller || null;
          return "ok";
        },
      );
      controlRegistrations.push(reg);

      const client = runRustClient(serverPort, "rpc-caller-disclose");
      const authidLine = await client.waitForLine("RUST_CLIENT_AUTHID:");
      const match = authidLine.match(/RUST_CLIENT_AUTHID: String\("([^"]+)"\)/);
      const expectedAuthid = match ? match[1] : null;

      await client.waitForLine("DISCLOSE_RESULT");

      assert.ok(expectedAuthid);
      assert.strictEqual(callerAuthid, expectedAuthid);
      assert.ok(typeof callerSession === "number" && callerSession > 0);

      await controlClient.session!.unregister(reg);
    });
  });

  test("T4.8: RPC Call Timeout (Caller Client Scenario)", async () => {
    // Control client registers a slow procedure
    const reg = await controlClient.session!.register("com.compat.slow_proc", async () => {
      await new Promise((r) => setTimeout(r, 1000));
      return "late";
    });
    controlRegistrations.push(reg);

    const client = runRustClient(serverPort, "rpc-timeout");
    const lines = await client.lines;
    assert.ok(lines.some((l) => l.includes("TIMEOUT_SUCCESS")));

    await controlClient.session!.unregister(reg);
  });

  test("T4.9: Caller-Initiated Call Cancellation (Callee Client Scenario)", async () => {
    const client = runRustClient(serverPort, "rpc-cancel");
    await client.waitForLine("REGISTERED");

    // Call the slow procedure
    const callPromise = controlClient.session!.call("com.compat.slow_proc");

    await client.waitForLine("INVOCATION_RECEIVED");

    // Cancel the call
    (callPromise as any).cancel({ mode: "kill" });

    await assert.rejects(Promise.resolve(callPromise), (err: any) => {
      assert.strictEqual(err.error, "wamp.error.canceled");
      return true;
    });

    const lines = await client.lines;
    assert.ok(lines.includes("CANCEL_RECEIVED"));
    assert.ok(lines.includes("CANCEL_RESPONDED"));
  });

  test("T4.10: Progressive Call Results", async (t) => {
    await t.test("TS Caller, Rust Callee", async () => {
      const client = runRustClient(serverPort, "rpc-callee-progressive");
      await client.waitForLine("REGISTERED");

      const progressResults: number[] = [];
      const finalResult = await new Promise<number>((resolve, reject) => {
        const d = (controlClient.session as any).call(
          "com.compat.callee_progress",
          [],
          {},
          { receive_progress: true },
        );
        d.then(
          (res: any) => resolve(res?.args ? res.args[0] : res),
          (err: any) => reject(err),
          (p: any) => {
            progressResults.push(p?.args ? p.args[0] : p);
          },
        );
      });

      assert.deepStrictEqual(progressResults, [10, 20]);
      assert.strictEqual(finalResult, 30);

      const lines = await client.lines;
      assert.ok(lines.includes("PROGRESS_SENT: 10"));
      assert.ok(lines.includes("PROGRESS_SENT: 20"));
      assert.ok(lines.includes("RESPONDED"));
    });

    await t.test("Caller Client Scenario", async () => {
      // Control client registers progressive procedure
      const reg = await controlClient.session!.register(
        "com.compat.progress",
        (args, kwargs, details) => {
          if (details?.progress) {
            details?.progress([10], undefined);
            details?.progress([20], undefined);
          }
          return 30;
        },
      );
      controlRegistrations.push(reg);

      const client = runRustClient(serverPort, "rpc-progressive");
      const lines = await client.lines;
      assert.ok(lines.some((l) => l.includes("PROGRESS_RESULT: [Integer(10)] progress: true")));
      assert.ok(lines.some((l) => l.includes("PROGRESS_RESULT: [Integer(20)] progress: true")));
      assert.ok(lines.some((l) => l.includes("PROGRESS_RESULT: [Integer(30)] progress: false")));

      await controlClient.session!.unregister(reg);
    });
  });
});
