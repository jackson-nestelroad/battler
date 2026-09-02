import { ChildProcess, spawn } from "child_process";
import * as assert from "node:assert";
import { after, before, describe, test } from "node:test";
import * as path from "path";
import * as readline from "readline";
import { fileURLToPath } from "url";

import { BattlerClient, ChoiceBuilder } from "battler-client";
import { BattlerMultiplayerClient } from "battler-multiplayer-client";
import { BattlerMultiplayerServiceClient } from "battler-multiplayer-service-client";
import { BattlerServiceClient } from "battler-service-client";
import { WampSessionProvider } from "battler-wamp-client";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

describe("JS/TS WAMP Clients Integration Tests", () => {
  let serverProcess: ChildProcess;
  let wampUrl: string = "";
  let p1Provider: WampSessionProvider;
  let p2Provider: WampSessionProvider;

  before(async () => {
    // Start the Rust battler-server on port 0
    const binPath = path.resolve(__dirname, "../../../target/debug/battler-server");
    const dataDir = path.resolve(__dirname, "../../../battle-data/data");

    console.log(`Spawning server binary: ${binPath} with data: ${dataDir}`);

    serverProcess = spawn(binPath, [
      "--port",
      "0",
      "--data-dir",
      dataDir,
      "--realm-name",
      "battler",
      "--realm-uri",
      "com.battler",
    ]);

    const rl = readline.createInterface({ input: serverProcess.stdout! });

    // Wait for the server to report its port
    wampUrl = await new Promise<string>((resolve, reject) => {
      const timeout = setTimeout(() => {
        reject(new Error("Timeout waiting for server to start"));
      }, 15000);

      rl.on("line", (line) => {
        console.log(`[SERVER] ${line}`);
        const match = line.match(/Server is running at (ws:\/\/127\.0\.0\.1:\d+)/);
        if (match) {
          clearTimeout(timeout);
          resolve(match[1]);
        }
      });

      serverProcess.stderr!.on("data", (data) => {
        console.error(`[SERVER ERR] ${data}`);
      });

      serverProcess.on("error", (err) => {
        clearTimeout(timeout);
        reject(err);
      });
    });

    console.log(`Server started on ${wampUrl}`);

    // Create player 1 provider
    p1Provider = new WampSessionProvider({
      url: wampUrl,
      realm: "com.battler",
      max_retries: 5,
      authmethods: ["wamp-battler-undisputed"],
      authid: "player_1",
      authextra: {
        role: "user",
      },
      onchallenge: () => "role:user",
    });
    await p1Provider.connect();

    // Create player 2 provider
    p2Provider = new WampSessionProvider({
      url: wampUrl,
      realm: "com.battler",
      max_retries: 5,
      authmethods: ["wamp-battler-undisputed"],
      authid: "player_2",
      authextra: {
        role: "user",
      },
      onchallenge: () => "role:user",
    });
    await p2Provider.connect();
  });

  after(async () => {
    if (p1Provider) await p1Provider.disconnect();
    if (p2Provider) await p2Provider.disconnect();
    if (serverProcess) {
      serverProcess.kill("SIGTERM");
    }
  });

  test("proposes, accepts, and plays a turn using raw WAMP service client references", async () => {
    // 1. Initialize Clients
    const p1ServiceClient = new BattlerServiceClient(p1Provider);
    const p2ServiceClient = new BattlerServiceClient(p2Provider);

    const p1MultiServiceClient = new BattlerMultiplayerServiceClient(p1Provider);
    const p2MultiServiceClient = new BattlerMultiplayerServiceClient(p2Provider);

    const p1MultiClient = new BattlerMultiplayerClient(
      "player_1",
      p1MultiServiceClient,
      p1ServiceClient,
    );
    const p2MultiClient = new BattlerMultiplayerClient(
      "player_2",
      p2MultiServiceClient,
      p2ServiceClient,
    );

    // 2. Propose battle as Player 1
    const p1Team = {
      members: [
        {
          name: "Bulbasaur",
          species: "Bulbasaur",
          ability: "Overgrow",
          nature: "Hardy",
          gender: "Female",
          moves: ["Tackle", "Growl"],
          level: 5,
        },
      ],
      bag: { items: {} },
    };

    const p2Team = {
      members: [
        {
          name: "Charmander",
          species: "Charmander",
          ability: "Blaze",
          nature: "Hardy",
          gender: "Female",
          moves: ["Scratch", "Growl"],
          level: 5,
        },
      ],
      bag: { items: {} },
    };

    const battleOptions = {
      seed: 0,
      format: {
        battle_type: "Singles" as const,
        rules: [],
      },
      field: {
        weather: null,
        terrain: null,
        environment: "Grass" as const,
        time: "Day" as const,
      },
      side_1: {
        name: "Side 1",
        players: [
          {
            id: "player_1",
            name: "Player 1",
            team: p1Team,
          },
        ],
      },
      side_2: {
        name: "Side 2",
        players: [
          {
            id: "player_2",
            name: "Player 2",
            team: { members: [] },
          },
        ],
      },
    };

    const proposedOptions = {
      battle_options: battleOptions,
      service_options: {
        creator: "player_1",
      },
      timeout: {
        secs: 30,
        nanos: 0,
      },
    };

    // P1 proposed battle start promise
    const p1StartPromise = p1MultiClient.proposeAndWaitForBattleStart(proposedOptions as any);

    // Give a short delay for proposal to publish
    await new Promise((r) => setTimeout(r, 100));

    // Player 2 lists proposed battles, finds the active one, and responds Accept
    const proposedList = await p2MultiClient.proposedBattles(10, 0);
    assert.strictEqual(proposedList.length, 1);
    const proposal = proposedList[0];

    // P2 starts waiting for the battle
    const p2StartPromise = p2MultiClient.waitForBattleStart(proposal.uuid).then((battleId) => {
      return p2MultiClient.createBattlerClient(battleId);
    });

    // P2 accepts the battle
    await p2MultiClient.respondToProposal(proposal.uuid, true, p2Team as any);

    // Both clients should resolve their start promises and catch up
    const p1Client = await p1StartPromise;
    const p2Client = await p2StartPromise;

    assert.ok(p1Client);
    assert.ok(p2Client);
    assert.strictEqual(p1Client.battleId, p2Client.battleId);

    // Give a small delay to make sure the start log gets published and processed
    await new Promise((r) => setTimeout(r, 150));

    // Listen to requests
    p1Client.on("request", (req) => {
      console.log(`[P1 CLIENT EVENT] request:`, req);
    });

    p2Client.on("request", (req) => {
      console.log(`[P2 CLIENT EVENT] request:`, req);
    });

    // Make choices when requests are ready
    if (!p1Client.request()) {
      await new Promise<void>((r) => {
        const handler = () => {
          p1Client.off("request", handler);
          r();
        };
        p1Client.on("request", handler);
      });
    }
    if (!p2Client.request()) {
      await new Promise<void>((r) => {
        const handler = () => {
          p2Client.off("request", handler);
          r();
        };
        p2Client.on("request", handler);
      });
    }

    await p1Client.makeChoice("move 1");
    await p2Client.makeChoice("move 1");

    // Wait until the battle state turn increases
    let turn2Resolved = false;
    for (let i = 0; i < 50; i++) {
      await new Promise((r) => setTimeout(r, 100));
      const s = p1Client.state();
      console.log(`[P1 CLIENT STATE] turn: ${s?.turn}, phase: ${s?.phase}`);
      if (s && s.turn >= 2) {
        turn2Resolved = true;
        break;
      }
    }
    assert.ok(turn2Resolved, "Turn 2 should be resolved after choices are made");

    // 5. Forfeit the battle as Player 1 to test cleanup/end flow
    let p1Ended = false;
    let p2Ended = false;

    p1Client.on("end", () => {
      console.log("[P1 CLIENT EVENT] end");
      p1Ended = true;
    });

    p2Client.on("end", () => {
      console.log("[P2 CLIENT EVENT] end");
      p2Ended = true;
    });

    await p1Client.makeChoice("forfeit");
    await p2Client.makeChoice("move 0");

    // Wait until battle resolves finished
    let finishedResolved = false;
    for (let i = 0; i < 50; i++) {
      await new Promise((r) => setTimeout(r, 100));
      const s = p1Client.state();
      console.log(`[P1 CLIENT STATE] phase: ${s?.phase}`);
      if (s && s.phase === "finished") {
        finishedResolved = true;
        break;
      }
    }
    assert.ok(finishedResolved, "Battle should finish after forfeit");
    assert.ok(p1Ended, "Player 1 client should emit 'end'");
    assert.ok(p2Ended, "Player 2 client should emit 'end'");

    // Clean up
    await p1Client.cancel();
    await p2Client.cancel();
  });

  test("proposes, accepts, and plays a battle utilizing WampSessionProvider and ChoiceBuilder", async () => {
    // 1. Establish sessions using WampSessionProvider
    const p1Provider = new WampSessionProvider({
      url: wampUrl,
      realm: "com.battler",
      max_retries: 5,
      authmethods: ["wamp-battler-undisputed"],
      authid: "player_1",
      authextra: { role: "user" },
      onchallenge: () => "role:user",
    });

    const p2Provider = new WampSessionProvider({
      url: wampUrl,
      realm: "com.battler",
      max_retries: 5,
      authmethods: ["wamp-battler-undisputed"],
      authid: "player_2",
      authextra: { role: "user" },
      onchallenge: () => "role:user",
    });

    await p1Provider.connect();
    await p2Provider.connect();

    // 2. Share provider to initialize clients
    const p1ServiceClient = new BattlerServiceClient(p1Provider);
    const p1MultiServiceClient = new BattlerMultiplayerServiceClient(p1Provider);
    const p1MultiClient = new BattlerMultiplayerClient(
      "player_1",
      p1MultiServiceClient,
      p1ServiceClient,
    );

    const p2ServiceClient = new BattlerServiceClient(p2Provider);
    const p2MultiServiceClient = new BattlerMultiplayerServiceClient(p2Provider);
    const p2MultiClient = new BattlerMultiplayerClient(
      "player_2",
      p2MultiServiceClient,
      p2ServiceClient,
    );

    // 3. Propose battle setup
    const p1Team = {
      members: [
        {
          name: "Charmander",
          species: "Charmander",
          ability: "Blaze",
          nature: "Hardy",
          gender: "Female",
          moves: ["Scratch", "Growl"],
          level: 100,
        },
      ],
      bag: { items: {} },
    };

    const p2Team = {
      members: [
        {
          name: "Bulbasaur",
          species: "Bulbasaur",
          ability: "Overgrow",
          nature: "Hardy",
          gender: "Female",
          moves: ["Tackle", "Growl"],
          level: 100,
        },
      ],
      bag: { items: {} },
    };

    const battleOptions = {
      seed: 0,
      format: {
        battle_type: "Singles" as const,
        rules: [],
      },
      field: {
        weather: null,
        terrain: null,
        environment: "Grass" as const,
        time: "Day" as const,
      },
      side_1: {
        name: "Side 1",
        players: [
          {
            id: "player_1",
            name: "Player 1",
            team: p1Team,
          },
        ],
      },
      side_2: {
        name: "Side 2",
        players: [
          {
            id: "player_2",
            name: "Player 2",
            team: { members: [] },
          },
        ],
      },
    };

    const proposedOptions = {
      battle_options: battleOptions,
      service_options: {
        creator: "player_1",
      },
      timeout: {
        secs: 30,
        nanos: 0,
      },
    };

    // P1 proposes the battle after cooldown
    await new Promise((r) => setTimeout(r, 3100));
    const proposed = await p1MultiClient.proposeBattle(proposedOptions as any);

    // P1 proposed battle start promise
    const p1StartPromise = p1MultiClient.waitForBattleStart(proposed.uuid).then((battleId) => {
      return p1MultiClient.createBattlerClient(battleId);
    });

    // P2 starts waiting for the battle
    const p2StartPromise = p2MultiClient.waitForBattleStart(proposed.uuid).then((battleId) => {
      return p2MultiClient.createBattlerClient(battleId);
    });

    // P2 accepts the battle
    await p2MultiClient.respondToProposal(proposed.uuid, true, p2Team as any);

    const [p1Client, p2Client] = await Promise.all([p1StartPromise, p2StartPromise]);
    assert.ok(p1Client, "Player 1 client should resolve and catch up");
    assert.ok(p2Client, "Player 2 client should resolve and catch up");

    // Play turn 1 using ChoiceBuilder moves
    for (let i = 0; i < 50; i++) {
      const r1 = await p1Client.fetchRequest();
      const r2 = await p2Client.fetchRequest();
      if (r1 && r2) break;
      await new Promise((r) => setTimeout(r, 100));
    }
    await Promise.all([
      p1Client.makeChoice(ChoiceBuilder.move(1).toString()),
      p2Client.makeChoice(ChoiceBuilder.move(1).toString()),
    ]);
    for (let i = 0; i < 50; i++) {
      const req = await p1Client.fetchRequest();
      if (req && (p1Client.state()?.turn ?? 0) >= 2) break;
      await new Promise((r) => setTimeout(r, 100));
    }

    let p1Ended = false;
    p1Client.on("end", () => {
      p1Ended = true;
    });

    await p1Client.makeChoice(ChoiceBuilder.forfeit().toString());
    await p2Client.makeChoice(ChoiceBuilder.move(0).toString());

    function isFinished(phase: any): boolean {
      if (!phase) return false;
      if (phase === "finished" || phase === "Finished") return true;
      if (typeof phase === "object" && ("finished" in phase || "Finished" in phase)) return true;
      return false;
    }

    // Wait until battle resolves finished
    let finishedResolved = false;
    for (let i = 0; i < 50; i++) {
      await new Promise((r) => setTimeout(r, 100));
      const s = p1Client.state();
      const phase = s?.phase;
      if (p1Ended || isFinished(phase)) {
        finishedResolved = true;
        break;
      }
    }
    assert.ok(finishedResolved, "Battle should finish after forfeit");

    // Clean up connections
    await p1Client.cancel();
    await p2Client.cancel();
    await p1Provider.disconnect();
    await p2Provider.disconnect();
  });

  test("WampSessionProvider emits lifecycle events on connection and disconnection", async () => {
    const provider = new WampSessionProvider({
      url: wampUrl,
      realm: "com.battler",
      max_retries: 5,
      authmethods: ["wamp-battler-undisputed"],
      authid: "player_1",
      authextra: { role: "user" },
      onchallenge: () => "role:user",
    });

    let connectFired = false;
    let disconnectFired = false;

    provider.on("connect", () => {
      connectFired = true;
    });

    provider.on("disconnect", () => {
      disconnectFired = true;
    });

    await provider.connect();
    assert.ok(connectFired, "connect event should be fired");
    assert.ok(provider.session, "session should be active");

    await provider.disconnect();
    assert.ok(disconnectFired, "disconnect event should be fired");
    assert.strictEqual(provider.session, null, "session should be null");
  });

  test("rejects invalid battle configurations or out-of-bounds choices with descriptive errors", async () => {
    const provider = new WampSessionProvider({
      url: wampUrl,
      realm: "com.battler",
      max_retries: 5,
      authmethods: ["wamp-battler-undisputed"],
      authid: "player_1",
      authextra: { role: "user" },
      onchallenge: () => "role:user",
    });

    await provider.connect();
    const serviceClient = new BattlerServiceClient(provider);

    // 1. Invalid battle creation parameters
    await assert.rejects(
      async () => {
        await serviceClient.create(
          { seed: 0n } as any,
          { creator: "player_1", timers: false } as any,
        );
      },
      () => {
        return true;
      },
      "Should reject invalid create options",
    );

    // 2. Making a choice on a non-existent battle
    await assert.rejects(
      async () => {
        await serviceClient.makeChoice("non-existent-uuid", "player_1", "move 0");
      },
      () => {
        return true;
      },
      "Should reject choice on non-existent battle",
    );

    await provider.disconnect();
  });

  test("queries player battles/proposals and manages matchmaking proposal updates", async () => {
    const provider = new WampSessionProvider({
      url: wampUrl,
      realm: "com.battler",
      max_retries: 5,
      authmethods: ["wamp-battler-undisputed"],
      authid: "player_1",
      authextra: { role: "user" },
      onchallenge: () => "role:user",
    });

    await provider.connect();
    const serviceClient = new BattlerServiceClient(provider);
    const multiplayerClient = new BattlerMultiplayerServiceClient(provider);

    // 2. Query battles and proposed battles filtered by player
    const emptyBattles = await serviceClient.battlesForPlayer("player_1", 10, 0);
    assert.ok(Array.isArray(emptyBattles), "battlesForPlayer should return list");

    const emptyProposals = await multiplayerClient.proposedBattlesForPlayer("player_1", 10, 0);
    assert.ok(Array.isArray(emptyProposals), "proposedBattlesForPlayer should return list");

    // 3. Propose a battle after cooldown
    await new Promise((r) => setTimeout(r, 3100));
    const proposed = await multiplayerClient.proposeBattle({
      battle_options: {
        seed: 0,
        format: {
          battle_type: "Singles",
          rules: [],
        },
        field: {
          weather: null,
          terrain: null,
          environment: "Grass",
          time: "Day",
        },
        side_1: {
          name: "Side 1",
          players: [
            { id: "player_1", name: "Player 1", team: { members: [], bag: { items: {} } } },
          ],
        },
        side_2: {
          name: "Side 2",
          players: [
            { id: "player_2", name: "Player 2", team: { members: [], bag: { items: {} } } },
          ],
        },
      } as any,
      service_options: {
        creator: "player_1",
        timers: {
          battle: null,
          player: null,
          action: null,
          team_preview: null,
        },
        log_timer_deadlines: false,
      },
      timeout: { secs: 30, nanos: 0 },
    });

    // Query it
    const queryProposed = await multiplayerClient.proposedBattle(proposed.uuid);
    assert.strictEqual(queryProposed.uuid, proposed.uuid);

    // Test multiplayer service subscription and unsubscribe
    const sub = await multiplayerClient.proposedBattleUpdates("player_1", () => {});
    await multiplayerClient.unsubscribe(sub);

    // Reject proposal to clean it up immediately
    await multiplayerClient.respondToProposedBattle(proposed.uuid, "player_1", { accept: false });

    await provider.disconnect();
  });

  test("spectator receives updates during active turn resolutions", async () => {
    const p1Provider = new WampSessionProvider({
      url: wampUrl,
      realm: "com.battler",
      max_retries: 5,
      authmethods: ["wamp-battler-undisputed"],
      authid: "player_1",
      authextra: { role: "user" },
      onchallenge: () => "role:user",
    });

    const p2Provider = new WampSessionProvider({
      url: wampUrl,
      realm: "com.battler",
      max_retries: 5,
      authmethods: ["wamp-battler-undisputed"],
      authid: "player_2",
      authextra: { role: "user" },
      onchallenge: () => "role:user",
    });

    const spectatorProvider = new WampSessionProvider({
      url: wampUrl,
      realm: "com.battler",
      max_retries: 5,
      authmethods: ["wamp-battler-undisputed"],
      authid: "spectator_1",
      authextra: { role: "user" },
      onchallenge: () => "role:user",
    });

    await Promise.all([p1Provider.connect(), p2Provider.connect(), spectatorProvider.connect()]);

    const p1MultiClient = new BattlerMultiplayerClient(
      "player_1",
      new BattlerMultiplayerServiceClient(p1Provider),
      new BattlerServiceClient(p1Provider),
    );

    const p2MultiClient = new BattlerMultiplayerClient(
      "player_2",
      new BattlerMultiplayerServiceClient(p2Provider),
      new BattlerServiceClient(p2Provider),
    );

    const spectatorServiceClient = new BattlerServiceClient(spectatorProvider);

    const p1Team = {
      members: [
        {
          name: "Bulbasaur",
          species: "Bulbasaur",
          ability: "Overgrow",
          nature: "Hardy",
          gender: "Female",
          moves: ["Tackle", "Growl"],
          level: 100,
        },
      ],
      bag: { items: {} },
    };

    const p2Team = {
      members: [
        {
          name: "Charmander",
          species: "Charmander",
          ability: "Blaze",
          nature: "Hardy",
          gender: "Female",
          moves: ["Scratch", "Growl"],
          level: 100,
        },
      ],
      bag: { items: {} },
    };

    const battleOptions = {
      seed: 123456,
      format: {
        battle_type: "Singles",
        rules: [],
      },
      field: {
        weather: null,
        terrain: null,
        environment: "Grass",
        time: "Day",
      },
      side_1: {
        name: "Player 1",
        players: [{ id: "player_1", name: "Player 1", team: p1Team }],
      },
      side_2: {
        name: "Player 2",
        players: [{ id: "player_2", name: "Player 2", team: { members: [] } }],
      },
    };

    const proposedOptions = {
      battle_options: battleOptions,
      service_options: {
        creator: "player_1",
        timers: { battle: null, player: null, action: null },
      },
      timeout: { secs: 30, nanos: 0 },
    };

    await new Promise((r) => setTimeout(r, 3100));
    const proposed = await p1MultiClient.proposeBattle(proposedOptions as any);

    // Players wait for battle start
    const p1StartPromise = p1MultiClient.waitForBattleStart(proposed.uuid).then((battleId) => {
      return p1MultiClient.createBattlerClient(battleId);
    });
    const p2StartPromise = p2MultiClient.waitForBattleStart(proposed.uuid).then((battleId) => {
      return p2MultiClient.createBattlerClient(battleId);
    });

    await p2MultiClient.respondToProposal(proposed.uuid, true, p2Team as any);

    const [p1Client, p2Client] = await Promise.all([p1StartPromise, p2StartPromise]);

    const battleId = p1Client.battleId;

    // Create Spectator client!
    const spectatorClient = await BattlerClient.create(
      battleId,
      "spectator_1",
      spectatorServiceClient,
    );

    // Track updates
    let spectatorUpdates = 0;
    spectatorClient.on("update", () => {
      spectatorUpdates++;
    });

    // Make choices
    for (let i = 0; i < 100; i++) {
      if (p1Client.request()) break;
      await new Promise((r) => setTimeout(r, 50));
    }
    for (let i = 0; i < 100; i++) {
      if (p2Client.request()) break;
      await new Promise((r) => setTimeout(r, 50));
    }

    await p1Client.makeChoice(ChoiceBuilder.move(1).toString());
    await p2Client.makeChoice(ChoiceBuilder.move(1).toString());

    // Wait until turn 2 state arrives or battle finishes
    for (let i = 0; i < 100; i++) {
      const s = p1Client.state();
      if (!s || s.turn >= 2 || s.phase === "finished") break;
      await new Promise((r) => setTimeout(r, 50));
    }

    // End battle by forfeiting
    await p1Client.makeChoice(ChoiceBuilder.forfeit().toString());
    await p2Client.makeChoice(ChoiceBuilder.move(0).toString()).catch(() => {});

    // Wait a brief period for logs to settle
    await new Promise((r) => setTimeout(r, 200));

    assert.ok(
      spectatorUpdates > 0,
      "Spectator should receive updates during active turn resolutions",
    );

    // Clean up
    await Promise.all([
      p1Client.cancel(),
      p2Client.cancel(),
      spectatorClient.cancel(),
      p1Provider.disconnect(),
      p2Provider.disconnect(),
      spectatorProvider.disconnect(),
    ]);
  });

  test("proposes, accepts, and starts a 4-player multi battle", async () => {
    const createProvider = (authid: string) =>
      new WampSessionProvider({
        url: wampUrl,
        realm: "com.battler",
        max_retries: 5,
        authmethods: ["wamp-battler-undisputed"],
        authid,
        authextra: { role: "user" },
        onchallenge: () => "role:user",
      });

    const p1Provider = createProvider("player_1");
    const p2Provider = createProvider("player_2");
    const p3Provider = createProvider("player_3");
    const p4Provider = createProvider("player_4");

    await Promise.all([
      p1Provider.connect(),
      p2Provider.connect(),
      p3Provider.connect(),
      p4Provider.connect(),
    ]);

    const createMultiClient = (provider: WampSessionProvider, player: string) =>
      new BattlerMultiplayerClient(
        player,
        new BattlerMultiplayerServiceClient(provider),
        new BattlerServiceClient(provider),
      );

    const p1Multi = createMultiClient(p1Provider, "player_1");
    const p2Multi = createMultiClient(p2Provider, "player_2");
    const p3Multi = createMultiClient(p3Provider, "player_3");
    const p4Multi = createMultiClient(p4Provider, "player_4");

    const makeMon = (name: string, species: string, move: string) => ({
      name,
      species,
      ability: "Blaze",
      nature: "Hardy",
      gender: "Female",
      moves: [move],
      level: 100,
    });

    const p1Team = {
      members: [
        {
          name: "Charmander1",
          species: "Charmander",
          ability: "Blaze",
          nature: "Hardy",
          gender: "Female",
          moves: ["Scratch"],
          level: 100,
        },
      ],
      bag: { items: {} },
    };
    const p2Team = {
      members: [
        {
          name: "Bulbasaur2",
          species: "Bulbasaur",
          ability: "Overgrow",
          nature: "Hardy",
          gender: "Female",
          moves: ["Tackle"],
          level: 100,
        },
      ],
      bag: { items: {} },
    };
    const p3Team = {
      members: [
        {
          name: "Squirtle3",
          species: "Squirtle",
          ability: "Torrent",
          nature: "Hardy",
          gender: "Female",
          moves: ["Tackle"],
          level: 100,
        },
      ],
      bag: { items: {} },
    };
    const p4Team = {
      members: [
        {
          name: "Pikachu4",
          species: "Pikachu",
          ability: "Static",
          nature: "Hardy",
          gender: "Female",
          moves: ["Quick Attack"],
          level: 100,
        },
      ],
      bag: { items: {} },
    };

    const proposedOptions = {
      battle_options: {
        seed: 123456,
        format: {
          battle_type: "Multi",
          rules: [],
        },
        field: {
          weather: null,
          terrain: null,
          environment: "Grass",
          time: "Day",
        },
        side_1: {
          name: "Side 1",
          players: [
            { id: "player_1", name: "Player 1", team: p1Team },
            { id: "player_2", name: "Player 2", team: { members: [] } },
          ],
        },
        side_2: {
          name: "Side 2",
          players: [
            { id: "player_3", name: "Player 3", team: { members: [] } },
            { id: "player_4", name: "Player 4", team: { members: [] } },
          ],
        },
      },
      service_options: {
        creator: "player_1",
        timers: { battle: null, player: null, action: null, team_preview: null },
        log_timer_deadlines: false,
      },
      timeout: { secs: 30, nanos: 0 },
    };

    await new Promise((r) => setTimeout(r, 3100));
    const proposed = await p1Multi.proposeBattle(proposedOptions as any);

    const startPromises = [
      p1Multi.waitForBattleStart(proposed.uuid).then((id) => p1Multi.createBattlerClient(id)),
      p2Multi.waitForBattleStart(proposed.uuid).then((id) => p2Multi.createBattlerClient(id)),
      p3Multi.waitForBattleStart(proposed.uuid).then((id) => p3Multi.createBattlerClient(id)),
      p4Multi.waitForBattleStart(proposed.uuid).then((id) => p4Multi.createBattlerClient(id)),
    ];

    await Promise.all([
      p2Multi.respondToProposal(proposed.uuid, true, p2Team as any),
      p3Multi.respondToProposal(proposed.uuid, true, p3Team as any),
      p4Multi.respondToProposal(proposed.uuid, true, p4Team as any),
    ]);

    const [c1, c2, c3, c4] = await Promise.all(startPromises);

    assert.ok(c1, "Client 1 should exist");
    assert.ok(c2, "Client 2 should exist");
    assert.ok(c3, "Client 3 should exist");
    assert.ok(c4, "Client 4 should exist");

    await Promise.all([c1.cancel(), c2.cancel(), c3.cancel(), c4.cancel()]);
    await new Promise((r) => setTimeout(r, 100));
    await Promise.all([
      p1Provider.disconnect(),
      p2Provider.disconnect(),
      p3Provider.disconnect(),
      p4Provider.disconnect(),
    ]);
  });

  test("proposes and plays a chaos battle against CPU", async () => {
    const p1Provider = new WampSessionProvider({
      url: wampUrl,
      realm: "com.battler",
      max_retries: 5,
      authmethods: ["wamp-battler-undisputed"],
      authid: "player_chaos",
      authextra: { role: "user" },
      onchallenge: () => "role:user",
    });

    await p1Provider.connect();

    const p1Multi = new BattlerMultiplayerClient(
      "player_chaos",
      new BattlerMultiplayerServiceClient(p1Provider),
      new BattlerServiceClient(p1Provider),
    );

    const specialOptions = {
      special_battle: {
        type: "chaos" as const,
        options: {
          mode: "singles_6v6" as const,
        },
      },
      side_1: {
        name: "Side 1",
        players: [
          {
            id: "player_chaos",
            name: "Player Chaos",
            player_type: { type: "trainer" },
            player_options: {},
            team: { members: [], bag: { items: {} } },
            dex: { species: [] },
          },
        ],
      },
      side_2: {
        name: "Side 2",
        players: [
          {
            id: "ai-random-1",
            name: "CPU",
            player_type: { type: "trainer" },
            player_options: {},
            team: { members: [], bag: { items: {} } },
            dex: { species: [] },
          },
        ],
      },
      service_options: {
        creator: "player_chaos",
        timers: { battle: null, player: null, action: null, team_preview: null },
        log_timer_deadlines: false,
      },
      timeout: { secs: 30, nanos: 0 },
    };

    const proposed = await p1Multi.proposeSpecialBattle(specialOptions);
    assert.ok(proposed.uuid);

    const battleId = await p1Multi.waitForBattleStart(proposed.uuid);
    assert.ok(battleId);

    const client = await p1Multi.createBattlerClient(battleId);
    assert.ok(client);

    if (!client.request()) {
      await new Promise<void>((r) => {
        const handler = () => {
          client.off("request", handler);
          r();
        };
        client.on("request", handler);
      });
    }

    const req = client.request();
    assert.ok(req);
    assert.strictEqual(req.type, "turn");

    await client.cancel();
    await p1Provider.disconnect();
  });
});
