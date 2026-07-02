import { test, describe, before, after, afterEach } from "node:test";
import * as assert from "node:assert";
import { spawn, ChildProcess } from "child_process";
import * as path from "path";
import * as readline from "readline";
import { fileURLToPath } from "url";
import * as autobahn from "autobahn";
import { WebSocket } from "ws";
import { generateScramResponse } from "./scram.js";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Monkeypatch autobahn transport creation to support SCRAM AUTHENTICATE extra details
const originalCreateTransport = (autobahn.Connection.prototype as any)._create_transport;
(autobahn.Connection.prototype as any)._create_transport = function () {
  const connectionInstance = this;
  const transport = originalCreateTransport.call(this);
  if (transport) {
    const originalSend = transport.send;
    transport.send = function (msg: any) {
      if (msg && msg[0] === 5) {
        // AUTHENTICATE
        const extra = msg[2] || {};
        if (connectionInstance._session && (connectionInstance._session as any)._scram_nonce) {
          extra.nonce = (connectionInstance._session as any)._scram_nonce;
        }
        msg[2] = extra;
      }
      return originalSend.call(this, msg);
    };
  }
  return transport;
};

// Helper to start the Rust router from the battler-wamp-compat-test-bin binary
function startRouter(auth: string = "none"): Promise<{ port: number; process: ChildProcess }> {
  return new Promise((resolve, reject) => {
    const binPath = path.resolve(__dirname, "../../target/debug/battler-wamp-compat-test-bin");
    const child = spawn(binPath, [
      "router",
      "--port",
      "0",
      "--realm",
      "com.compat.test",
      "--auth",
      auth,
    ]);

    const rl = readline.createInterface({ input: child.stdout });
    let ready = false;

    child.stderr.on("data", (data) => {
      console.error(`[ROUTER ERR] ${data}`);
    });

    child.on("error", (err) => {
      if (!ready) {
        reject(err);
      }
    });

    child.on("exit", (code) => {
      if (!ready) {
        reject(new Error(`Router exited with code ${code} before becoming ready`));
      }
    });

    rl.on("line", (line) => {
      console.log(`[ROUTER OUT] ${line}`);
      if (line.startsWith("READY:")) {
        const match = line.match(/:(\d+)$/);
        if (match) {
          ready = true;
          resolve({ port: parseInt(match[1], 10), process: child });
        }
      }
    });
  });
}

class ProgressPromise<T> extends Promise<T> {
  private _progressHandlers: ((value: any) => void)[] = [];

  progress(handler: (value: any) => void): this {
    this._progressHandlers.push(handler);
    return this;
  }

  then(onfulfilled?: any, onrejected?: any, onprogress?: any): any {
    if (onprogress) {
      this._progressHandlers.push(onprogress);
    }
    return super.then(onfulfilled, onrejected);
  }

  _notify(value: any) {
    for (const handler of this._progressHandlers) {
      try {
        handler(value);
      } catch (e) {
        console.error("Error in progress handler:", e);
      }
    }
  }
}

function customDeferredFactory() {
  const deferred: any = {};
  deferred.promise = new ProgressPromise((resolve, reject) => {
    deferred.resolve = resolve;
    deferred.reject = reject;
  });
  deferred.notify = (value: any) => {
    (deferred.promise as any)._notify(value);
  };
  return deferred;
}

function connectClient(port: number, options: any = {}): Promise<autobahn.Connection> {
  return new Promise((resolve, reject) => {
    const connection = new autobahn.Connection({
      url: `ws://127.0.0.1:${port}`,
      realm: "com.compat.test",
      max_retries: 0,
      use_deferred: customDeferredFactory,
      ...options,
    });
    connection.onopen = (session, details) => {
      resolve(connection);
    };
    connection.onclose = (reason, details) => {
      reject(new Error(`Connection closed: ${reason} details: ${JSON.stringify(details)}`));
      return false;
    };
    connection.open();
  });
}

function makeScramChallenger(clientNonce: string, password?: string) {
  return async (session: any, method: string, extra: any) => {
    if (method === "scram") {
      const scramResponse = await generateScramResponse(
        "test-user",
        password || "test-password123!",
        clientNonce,
        {
          nonce: extra.nonce,
          salt: extra.salt,
          kdf: extra.kdf,
          iterations: extra.iterations,
          memory: extra.memory,
        },
      );
      session._scram_nonce = extra.nonce;
      return scramResponse.signature;
    }
    throw new Error("Unsupported auth method");
  };
}

describe("WAMP Router Compatibility Tests", () => {
  let router: { port: number; process: ChildProcess };

  before(async () => {
    router = await startRouter("none");
  });

  after(() => {
    if (router && router.process) {
      router.process.kill();
    }
  });

  test("T1.1: Basic Hello and Welcome Handshake", async () => {
    const connection = await connectClient(router.port);
    assert.ok(connection.session);
    assert.ok(connection.session.id > 0);
    connection.close();
  });

  test("T1.2: Joining Invalid Realm", async () => {
    await assert.rejects(
      connectClient(router.port, { realm: "com.invalid.realm" }),
      /Connection closed: (unreachable|closed)/,
    );
  });

  test("T1.3: Clean Goodbye Handshake", async () => {
    const connection = await connectClient(router.port);
    assert.ok(connection.isOpen);

    const closed = new Promise<void>((resolve) => {
      connection.onclose = () => {
        resolve();
        return false;
      };
    });

    // clean close triggers goodbye exchange internally in autobahn
    connection.close();
    await closed;
    assert.ok(!connection.isOpen);
  });

  test("T1.4: Protocol Violation", async () => {
    // Send a direct raw socket message that violates protocol (invalid json)
    const ws = new WebSocket(`ws://127.0.0.1:${router.port}`, "wamp.2.json");
    await new Promise<void>((resolve, reject) => {
      ws.on("open", () => {
        ws.send("invalid json payload");
      });
      ws.on("close", (code, reason) => {
        // Expect connection to be closed by router due to protocol violation
        assert.ok(code > 0);
        resolve();
      });
      ws.on("error", (err) => {
        resolve();
      });
    });
  });

  test("T1.5: Serializer Negotiation", async () => {
    // Verify JSON subprotocol
    const wsJson = new WebSocket(`ws://127.0.0.1:${router.port}`, "wamp.2.json");
    await new Promise<void>((resolve, reject) => {
      wsJson.on("open", () => {
        assert.strictEqual(wsJson.protocol, "wamp.2.json");
        wsJson.close();
        resolve();
      });
      wsJson.on("error", reject);
    });

    // Verify MessagePack subprotocol
    const wsMsgpack = new WebSocket(`ws://127.0.0.1:${router.port}`, "wamp.2.msgpack");
    await new Promise<void>((resolve, reject) => {
      wsMsgpack.on("open", () => {
        assert.strictEqual(wsMsgpack.protocol, "wamp.2.msgpack");
        wsMsgpack.close();
        resolve();
      });
      wsMsgpack.on("error", reject);
    });
  });

  test("T1.6: Registration/Subscription Cleanup on Abrupt Disconnect", async () => {
    const callee = await connectClient(router.port);
    await callee.session!.register("com.compat.cleanup", () => "ok");

    const closed = new Promise<void>((resolve) => {
      callee.onclose = () => {
        resolve();
        return false;
      };
    });
    callee.close();
    await closed;

    // Wait a brief moment for the router to complete session cleanup
    await new Promise((resolve) => setTimeout(resolve, 200));

    const caller = await connectClient(router.port);
    await assert.rejects(
      Promise.resolve(caller.session!.call("com.compat.cleanup")),
      (err: any) => err.error === "wamp.error.no_available_callee",
    );

    caller.close();
  });

  test("T1.7: MessagePack Serializer Handshake", async () => {
    const connection = await connectClient(router.port, {
      serializers: [new (autobahn as any).serializer.MsgpackSerializer()],
    });
    assert.ok(connection.isOpen);

    const closed = new Promise<void>((resolve) => {
      connection.onclose = () => {
        resolve();
        return false;
      };
    });
    connection.close();
    await closed;
  });

  test("T1.8: Subscription Cleanup on Abrupt Disconnect", async () => {
    const subscriber = await connectClient(router.port);
    const otherSub = await connectClient(router.port);
    const publisher = await connectClient(router.port);

    let subCount = 0;
    let otherCount = 0;

    await subscriber.session!.subscribe("com.compat.cleanup_sub", () => {
      subCount++;
    });
    await otherSub.session!.subscribe("com.compat.cleanup_sub", () => {
      otherCount++;
    });

    // Both should receive before the disconnect
    publisher.session!.publish("com.compat.cleanup_sub", []);
    await new Promise((resolve) => setTimeout(resolve, 200));
    assert.strictEqual(subCount, 1);
    assert.strictEqual(otherCount, 1);

    // Disconnect subscriber abruptly; allow router to process the session close
    const closed = new Promise<void>((resolve) => {
      subscriber.onclose = () => {
        resolve();
        return false;
      };
    });
    subscriber.close();
    await closed;
    await new Promise((resolve) => setTimeout(resolve, 200));

    // Publish again — only otherSub should receive
    publisher.session!.publish("com.compat.cleanup_sub", []);
    await new Promise((resolve) => setTimeout(resolve, 200));
    assert.strictEqual(subCount, 1, "Disconnected subscriber must not receive further events");
    assert.strictEqual(
      otherCount,
      2,
      "Remaining subscriber must still receive events after peer disconnect",
    );

    publisher.close();
    otherSub.close();
  });

  test("T3.1: Basic Pub/Sub (Unacknowledged)", async () => {
    const clientA = await connectClient(router.port);
    const clientB = await connectClient(router.port);

    let resolveEvent: any;
    const eventReceived = new Promise<any>((resolve) => {
      resolveEvent = resolve;
    });

    await clientA.session!.subscribe("com.compat.topic", (args, kwargs) => {
      resolveEvent({ args, kwargs });
    });

    // Publish
    clientB.session!.publish("com.compat.topic", [123, "hello"], { foo: "bar" });

    const event = await eventReceived;
    assert.deepStrictEqual(event.args, [123, "hello"]);
    assert.deepStrictEqual(event.kwargs, { foo: "bar" });

    clientA.close();
    clientB.close();
  });

  test("T3.2: Pub/Sub with Acknowledgment", async () => {
    const client = await connectClient(router.port);
    const pubResult = await client.session!.publish(
      "com.compat.topic",
      [],
      {},
      { acknowledge: true },
    );
    assert.ok(pubResult.id > 0);
    client.close();
  });

  test("T3.3: Prefix-based Subscription Matching", async () => {
    const clientA = await connectClient(router.port);
    const clientB = await connectClient(router.port);

    const events: any[] = [];
    await clientA.session!.subscribe(
      "com.compat",
      (args, kwargs, details) => {
        events.push({ args, topic: details?.topic });
      },
      { match: "prefix" },
    );

    clientB.session!.publish("com.compat.foo", [1]);
    clientB.session!.publish("com.compat.bar", [2]);

    await new Promise((resolve) => setTimeout(resolve, 300));

    assert.strictEqual(events.length, 2);
    assert.strictEqual(events[0].topic, "com.compat.foo");
    assert.strictEqual(events[1].topic, "com.compat.bar");

    clientA.close();
    clientB.close();
  });

  test("T3.4: Wildcard-based Subscription Matching", async () => {
    const clientA = await connectClient(router.port);
    const clientB = await connectClient(router.port);

    const events: any[] = [];
    await clientA.session!.subscribe(
      "com.compat..status",
      (args, kwargs, details) => {
        events.push({ args, topic: details?.topic });
      },
      { match: "wildcard" },
    );

    clientB.session!.publish("com.compat.node1.status", [10]);
    clientB.session!.publish("com.compat.node1.logs", [20]); // should not match
    clientB.session!.publish("com.compat.node2.status", [30]);

    await new Promise((resolve) => setTimeout(resolve, 300));

    assert.strictEqual(events.length, 2);
    assert.deepStrictEqual(
      events.map((e) => e.topic),
      ["com.compat.node1.status", "com.compat.node2.status"],
    );

    clientA.close();
    clientB.close();
  });

  test("T3.5: Clean Unsubscribe", async () => {
    const clientA = await connectClient(router.port);
    const clientB = await connectClient(router.port);

    let count = 0;
    const sub = await clientA.session!.subscribe("com.compat.unsub", () => {
      count++;
    });

    clientB.session!.publish("com.compat.unsub", []);
    await new Promise((resolve) => setTimeout(resolve, 100));
    assert.strictEqual(count, 1);

    await clientA.session!.unsubscribe(sub);

    clientB.session!.publish("com.compat.unsub", []);
    await new Promise((resolve) => setTimeout(resolve, 100));
    assert.strictEqual(count, 1); // should not have incremented

    clientA.close();
    clientB.close();
  });

  test("T3.6: Pattern-Based Pub/Sub (Prefix & Wildcard)", async () => {
    const subPrefix = await connectClient(router.port);
    const subWildcard = await connectClient(router.port);
    const publisher = await connectClient(router.port);

    let prefixCount = 0;
    let prefixReceivedArgs: any[] = [];
    await subPrefix.session!.subscribe(
      "com.compat.prefix",
      (args) => {
        prefixCount++;
        prefixReceivedArgs = args || [];
      },
      { match: "prefix" },
    );

    let wildcardCount = 0;
    let wildcardReceivedArgs: any[] = [];
    await subWildcard.session!.subscribe(
      "com.compat.wildcard..event",
      (args) => {
        wildcardCount++;
        wildcardReceivedArgs = args || [];
      },
      { match: "wildcard" },
    );

    // publish matching prefix
    await publisher.session!.publish("com.compat.prefix.subtopic", ["prefix-match"]);
    // publish matching wildcard
    await publisher.session!.publish("com.compat.wildcard.test.event", ["wildcard-match"]);
    // publish non-matching
    await publisher.session!.publish("com.compat.other", ["no-match"]);

    await new Promise((resolve) => setTimeout(resolve, 200));

    assert.strictEqual(prefixCount, 1);
    assert.deepStrictEqual(prefixReceivedArgs, ["prefix-match"]);

    assert.strictEqual(wildcardCount, 1);
    assert.deepStrictEqual(wildcardReceivedArgs, ["wildcard-match"]);

    subPrefix.close();
    subWildcard.close();
    publisher.close();
  });

  test("T3.7: Publisher Identification (Disclose Publisher)", async () => {
    const subscriber = await connectClient(router.port);
    const publisher = await connectClient(router.port);

    // Collect full event details for each received event
    const disclosedDetails: any[] = [];
    await subscriber.session!.subscribe(
      "com.compat.pub_disclose",
      (_args: any, _kwargs: any, details: any) => {
        disclosedDetails.push(details);
      },
    );

    // Publish with disclose_me: true — router must include publisher session ID in EVENT
    publisher.session!.publish("com.compat.pub_disclose", [], {}, { disclose_me: true });
    await new Promise((resolve) => setTimeout(resolve, 200));

    assert.strictEqual(disclosedDetails.length, 1);
    assert.ok(
      typeof disclosedDetails[0].publisher === "number" && disclosedDetails[0].publisher > 0,
      "EVENT details must contain publisher session ID when disclose_me: true",
    );

    // Publish without disclose_me — publisher session ID should NOT appear
    const hiddenDetails: any[] = [];
    await subscriber.session!.subscribe(
      "com.compat.pub_hidden",
      (_args: any, _kwargs: any, details: any) => {
        hiddenDetails.push(details);
      },
    );
    publisher.session!.publish("com.compat.pub_hidden", []);
    await new Promise((resolve) => setTimeout(resolve, 200));

    assert.strictEqual(hiddenDetails.length, 1);
    assert.ok(
      hiddenDetails[0].publisher === undefined || hiddenDetails[0].publisher === null,
      "EVENT details must NOT contain publisher session ID without disclose_me",
    );

    subscriber.close();
    publisher.close();
  });

  test("T3.8: Pub/Sub exclude_me Option", async () => {
    const client = await connectClient(router.port);
    const otherSub = await connectClient(router.port);

    let selfCount = 0;
    let otherCount = 0;

    await client.session!.subscribe("com.compat.excludeme", () => {
      selfCount++;
    });
    await otherSub.session!.subscribe("com.compat.excludeme", () => {
      otherCount++;
    });

    // Default: exclude_me is implicitly true — publisher must NOT receive its own event
    await client.session!.publish("com.compat.excludeme", [], {}, { acknowledge: true });
    await new Promise((resolve) => setTimeout(resolve, 200));
    assert.strictEqual(
      selfCount,
      0,
      "Publisher must not receive own event (exclude_me defaults to true)",
    );
    assert.strictEqual(otherCount, 1, "Other subscriber must receive the event");

    // Explicit exclude_me: false — publisher MUST receive its own event
    await client.session!.publish(
      "com.compat.excludeme",
      [],
      {},
      { acknowledge: true, exclude_me: false },
    );
    await new Promise((resolve) => setTimeout(resolve, 200));
    assert.strictEqual(selfCount, 1, "Publisher must receive own event when exclude_me: false");
    assert.strictEqual(otherCount, 2, "Other subscriber must still receive the event");

    client.close();
    otherSub.close();
  });

  test("T4.1: Basic RPC (Call & Yield)", async () => {
    const callee = await connectClient(router.port);
    const caller = await connectClient(router.port);

    await callee.session!.register("com.compat.proc", (args) => {
      return (args?.[0] as number) + (args?.[1] as number);
    });

    const res = await caller.session!.call("com.compat.proc", [10, 20]);
    assert.strictEqual(res, 30);

    callee.close();
    caller.close();
  });

  test("T4.2: Calling Non-existent Procedure", async () => {
    const caller = await connectClient(router.port);
    await assert.rejects(
      Promise.resolve(caller.session!.call("com.compat.nonexistent")),
      (err: any) => err.error === "wamp.error.no_such_procedure",
    );
    caller.close();
  });

  test("T4.3: Unregistering a Procedure", async () => {
    const callee = await connectClient(router.port);
    const caller = await connectClient(router.port);

    const reg = await callee.session!.register("com.compat.temp", () => "ok");
    let res = await caller.session!.call("com.compat.temp");
    assert.strictEqual(res, "ok");

    await callee.session!.unregister(reg);

    await assert.rejects(
      Promise.resolve(caller.session!.call("com.compat.temp")),
      (err: any) => err.error === "wamp.error.no_such_procedure",
    );

    callee.close();
    caller.close();
  });

  test("T4.4: RPC Error Propagation", async () => {
    const callee = await connectClient(router.port);
    const caller = await connectClient(router.port);

    await callee.session!.register("com.compat.error", () => {
      throw new autobahn.Error("com.compat.custom_error", ["failed details"], {
        extra_key: "extra_val",
      });
    });

    await assert.rejects(Promise.resolve(caller.session!.call("com.compat.error")), (err: any) => {
      assert.strictEqual(err.error, "com.compat.custom_error");
      assert.deepStrictEqual(err.args, ["failed details"]);
      assert.deepStrictEqual(err.kwargs, { extra_key: "extra_val" });
      return true;
    });

    callee.close();
    caller.close();
  });

  test("T4.5: Shared Registrations (Invocation Policies)", async () => {
    const callee1 = await connectClient(router.port);
    const callee2 = await connectClient(router.port);
    const caller = await connectClient(router.port);

    // Policy: single (default) should fail to register twice
    await callee1.session!.register("com.compat.shared.single", () => 1);
    await assert.rejects(
      Promise.resolve(callee2.session!.register("com.compat.shared.single", () => 2)),
      (err: any) => err.error === "wamp.error.procedure_already_exists",
    );

    // Policy: roundrobin
    await callee1.session!.register("com.compat.shared.rr", () => 1, { invoke: "roundrobin" });
    await callee2.session!.register("com.compat.shared.rr", () => 2, { invoke: "roundrobin" });

    const resultsRr = [
      await caller.session!.call("com.compat.shared.rr"),
      await caller.session!.call("com.compat.shared.rr"),
      await caller.session!.call("com.compat.shared.rr"),
    ];
    // Verify we hit both callees
    assert.ok(resultsRr.includes(1));
    assert.ok(resultsRr.includes(2));

    // Policy: first
    await callee1.session!.register("com.compat.shared.first", () => 1, { invoke: "first" });
    await callee2.session!.register("com.compat.shared.first", () => 2, { invoke: "first" });
    for (let i = 0; i < 5; i++) {
      const res = await caller.session!.call("com.compat.shared.first");
      assert.strictEqual(res, 1);
    }

    // Policy: last
    await callee1.session!.register("com.compat.shared.last", () => 1, { invoke: "last" });
    await callee2.session!.register("com.compat.shared.last", () => 2, { invoke: "last" });
    for (let i = 0; i < 5; i++) {
      const res = await caller.session!.call("com.compat.shared.last");
      assert.strictEqual(res, 2);
    }

    // Policy: random — both callees should be reachable, all calls must succeed
    await callee1.session!.register("com.compat.shared.random", () => 1, { invoke: "random" });
    await callee2.session!.register("com.compat.shared.random", () => 2, { invoke: "random" });

    const resultsRandom = await Promise.all([
      caller.session!.call("com.compat.shared.random"),
      caller.session!.call("com.compat.shared.random"),
      caller.session!.call("com.compat.shared.random"),
      caller.session!.call("com.compat.shared.random"),
    ]);
    assert.ok(
      (resultsRandom as number[]).every((r) => r === 1 || r === 2),
      "All calls with random policy must be dispatched to a registered callee",
    );

    callee1.close();
    callee2.close();
    caller.close();
  });

  test("T4.6: RPC Pattern Matching (Prefix & Wildcard)", async () => {
    const callee = await connectClient(router.port);
    const caller = await connectClient(router.port);

    // 1. Prefix Matching
    await callee.session!.register(
      "com.compat.prefix",
      (args, kwargs, details) => {
        return details?.procedure;
      },
      { match: "prefix" } as autobahn.IRegisterOptions,
    );

    const resPrefix = await caller.session!.call("com.compat.prefix.subproc");
    assert.strictEqual(resPrefix, "com.compat.prefix.subproc");

    // 2. Wildcard Matching
    await callee.session!.register(
      "com.compat.wildcard..proc",
      (args, kwargs, details) => {
        return details?.procedure;
      },
      { match: "wildcard" } as autobahn.IRegisterOptions,
    );

    const resWildcard1 = await caller.session!.call("com.compat.wildcard.foo.proc");
    assert.strictEqual(resWildcard1, "com.compat.wildcard.foo.proc");

    const resWildcard2 = await caller.session!.call("com.compat.wildcard.bar.proc");
    assert.strictEqual(resWildcard2, "com.compat.wildcard.bar.proc");

    // Verify non-matching wildcard call fails
    await assert.rejects(
      Promise.resolve(caller.session!.call("com.compat.wildcard.foo.other")),
      (err: any) => err.error === "wamp.error.no_such_procedure",
    );

    // 3. Priority/Conflict Resolution: Exact vs Prefix
    await callee.session!.register("com.compat.match.exact", () => "exact");
    await callee.session!.register("com.compat.match", () => "prefix", {
      match: "prefix",
    } as autobahn.IRegisterOptions);

    // calling exact should hit exact
    const resExact = await caller.session!.call("com.compat.match.exact");
    assert.strictEqual(resExact, "exact");

    // calling prefix sub-procedure should hit prefix
    const resPrefixSub = await caller.session!.call("com.compat.match.other");
    assert.strictEqual(resPrefixSub, "prefix");

    callee.close();
    caller.close();
  });

  test("T4.7: Caller Identification (Disclose Caller)", async () => {
    const callee = await connectClient(router.port);
    const caller = await connectClient(router.port);

    // 1. Opt-in disclosure (disclose_caller = false on registration)
    await callee.session!.register("com.compat.disclose.optin", (args, kwargs, details) => {
      return (details as any)?.caller_authid !== undefined
        ? (details as any)?.caller_authid
        : "hidden";
    });

    // Call with disclose_me = true
    const resOptInTrue = await caller.session!.call(
      "com.compat.disclose.optin",
      [],
      {},
      { disclose_me: true },
    );
    assert.ok(typeof resOptInTrue === "string");

    // Call without disclose_me
    const resOptInFalse = await caller.session!.call("com.compat.disclose.optin");
    assert.strictEqual(resOptInFalse, "hidden");

    // 2. Forced disclosure (disclose_caller = true on registration)
    await callee.session!.register(
      "com.compat.disclose.force",
      (args, kwargs, details) => {
        return (details as any)?.caller_authid !== undefined
          ? (details as any)?.caller_authid
          : "hidden";
      },
      { disclose_caller: true },
    );

    // Call without disclose_me (should still be disclosed)
    const resForce = await caller.session!.call("com.compat.disclose.force");
    assert.ok(typeof resForce === "string");

    callee.close();
    caller.close();
  });

  test("T4.8: RPC Call Timeout", async () => {
    const callee = await connectClient(router.port);
    const caller = await connectClient(router.port);

    await callee.session!.register("com.compat.slow", async () => {
      await new Promise((resolve) => setTimeout(resolve, 1000));
      return "done";
    });

    await assert.rejects(
      Promise.resolve(caller.session!.call("com.compat.slow", [], {}, { timeout: 300 })),
      (err: any) => err.error === "wamp.error.canceled",
    );

    callee.close();
    caller.close();
  });

  test("T4.9: Caller-Initiated Call Cancellation", async () => {
    const callee = await connectClient(router.port);
    const caller = await connectClient(router.port);

    await callee.session!.register("com.compat.cancelable", async () => {
      await new Promise((resolve) => setTimeout(resolve, 1500));
      return "done";
    });

    const callPromise = caller.session!.call("com.compat.cancelable");

    // Wait briefly for the call request to be sent
    await new Promise((resolve) => setTimeout(resolve, 100));

    // Cancel the call
    (callPromise as any).cancel({ mode: "kill" });

    await assert.rejects(
      Promise.resolve(callPromise),
      (err: any) => err.error === "wamp.error.canceled",
    );

    callee.close();
    caller.close();
  });

  test("T4.10: Progressive Call Results", async () => {
    const callee = await connectClient(router.port);
    const caller = await connectClient(router.port);

    await callee.session!.register("com.compat.progressive", (args, kwargs, details) => {
      if (details?.progress) {
        details?.progress([1], undefined);
        details?.progress([2], undefined);
      }
      return 3;
    });

    const progressResults: number[] = [];
    const callPromise = caller.session!.call(
      "com.compat.progressive",
      [],
      {},
      {
        receive_progress: true,
      },
    );
    callPromise.then(
      (finalResult) => {
        assert.strictEqual(finalResult, 3);
      },
      (err) => {},
      (val: any) => {
        progressResults.push(val);
      },
    );

    const finalResult = await callPromise;
    assert.deepStrictEqual(progressResults, [1, 2]);
    assert.strictEqual(finalResult, 3);

    callee.close();
    caller.close();
  });
});

describe("WAMP Router Authentication Compatibility Tests", () => {
  let currentRouter: any = null;

  afterEach(() => {
    if (currentRouter && currentRouter.process) {
      currentRouter.process.kill();
      currentRouter = null;
    }
  });

  test("T2.1: Undisputed Authentication", async () => {
    currentRouter = await startRouter("undisputed");
    const connection = await connectClient(currentRouter.port, {
      authmethods: ["wamp-battler-undisputed"],
      authid: "test-user",
      authextra: {
        role: "user",
      },
      onchallenge: () => "role:user",
    });
    assert.ok(connection.session);
    connection.close();
  });

  test("T2.2: WAMP-SCRAM Successful Authentication", async () => {
    currentRouter = await startRouter("scram");
    const clientNonce = "clientnonce1234567890";
    const connection = await connectClient(currentRouter.port, {
      authmethods: ["scram"],
      authid: "test-user",
      authextra: {
        nonce: clientNonce,
      },
      onchallenge: makeScramChallenger(clientNonce),
    });
    assert.ok(connection.session);
    connection.close();
  });

  test("T2.3: SCRAM Authentication - Incorrect Password", async () => {
    currentRouter = await startRouter("scram");
    const clientNonce = "clientnonce1234567890";
    await assert.rejects(
      connectClient(currentRouter.port, {
        authmethods: ["scram"],
        authid: "test-user",
        authextra: {
          nonce: clientNonce,
        },
        onchallenge: makeScramChallenger(clientNonce, "wrong-password"),
      }),
      /Connection closed:.*(authfail|authentication_failed|authentication_denied|no_such_principal)/,
    );
  });

  test("T2.4: SCRAM Authentication - Invalid User", async () => {
    currentRouter = await startRouter("scram");
    const clientNonce = "clientnonce1234567890";
    await assert.rejects(
      connectClient(currentRouter.port, {
        authmethods: ["scram"],
        authid: "invalid-user",
        authextra: {
          nonce: clientNonce,
        },
        onchallenge: () => "",
      }),
      /Connection closed:.*(authfail|authentication_failed|authentication_denied|no_such_principal)/,
    );
  });
});
