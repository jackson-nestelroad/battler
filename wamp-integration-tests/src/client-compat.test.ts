import { test, describe, before, after, afterEach } from 'node:test';
import * as assert from 'node:assert';
import { spawn, execSync, ChildProcess } from 'child_process';
import * as path from 'path';
import * as readline from 'readline';
import { fileURLToPath } from 'url';
import * as autobahn from 'autobahn';
import * as fs from 'fs';
import * as net from 'net';

// Monkeypatch Autobahn's Session prototype to negotiate call_timeout and call_canceling features.
// This is necessary because Autobahn JS disables these features by default in its static roles.
const originalSendWamp = (autobahn as any).Session.prototype._send_wamp;
(autobahn as any).Session.prototype._send_wamp = function(msg: any) {
    if (msg && msg[0] === 1) { // HELLO
        const details = msg[2] || {};
        if (details.roles) {
            if (details.roles.callee) {
                if (!details.roles.callee.features) details.roles.callee.features = {};
                details.roles.callee.features.call_timeout = true;
                details.roles.callee.features.call_canceling = true;
            }
            if (details.roles.caller) {
                if (!details.roles.caller.features) details.roles.caller.features = {};
                details.roles.caller.features.call_timeout = true;
                details.roles.caller.features.call_canceling = true;
            }
        }
    }
    return originalSendWamp.call(this, msg);
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
            socket.on('connect', () => {
                socket.destroy();
                resolve();
            });
            socket.on('error', () => {
                socket.destroy();
                if (Date.now() - startTime > timeoutMs) {
                    reject(new Error(`Timed out waiting for port ${port}`));
                } else {
                    setTimeout(check, 100);
                }
            });
            socket.on('timeout', () => {
                socket.destroy();
                if (Date.now() - startTime > timeoutMs) {
                    reject(new Error(`Timed out waiting for port ${port}`));
                } else {
                    setTimeout(check, 100);
                }
            });
            socket.connect(port, '127.0.0.1');
        };
        check();
    });
}

// Helper to spawn the Rust client scenario
function runRustClient(
    port: number,
    scenario: string,
    extraArgs: string[] = []
): { process: ChildProcess; lines: Promise<string[]>; waitForLine: (pat: RegExp | string) => Promise<string> } {
    const binPath = path.resolve(__dirname, '../../target/debug/battler-wamp-compat-test-bin');
    const child = spawn(binPath, [
        'client',
        '--url', `ws://127.0.0.1:${port}`,
        '--realm', 'com.compat.test',
        '--scenario', scenario,
        ...extraArgs
    ]);

    const rl = readline.createInterface({ input: child.stdout });
    const collected: string[] = [];
    const lineCallbacks: Array<{ pat: RegExp | string; resolve: (val: string) => void }> = [];

    rl.on('line', (line) => {
        console.log(`[CLIENT-${scenario}] ${line}`);
        collected.push(line);
        // Check callbacks
        for (let i = lineCallbacks.length - 1; i >= 0; i--) {
            const cb = lineCallbacks[i];
            let match = false;
            if (cb.pat instanceof RegExp) {
                match = cb.pat.test(line);
            } else {
                match = line.includes(cb.pat);
            }
            if (match) {
                cb.resolve(line);
                lineCallbacks.splice(i, 1);
            }
        }
    });

    child.stderr.on('data', (data) => {
        console.error(`[CLIENT-${scenario} ERR] ${data}`);
    });

    const linesPromise = new Promise<string[]>((resolve) => {
        child.on('close', () => resolve(collected));
    });

    const waitForLine = (pat: RegExp | string): Promise<string> => {
        // First check if already collected
        for (const line of collected) {
            let match = false;
            if (pat instanceof RegExp) {
                match = pat.test(line);
            } else {
                match = line.includes(pat);
            }
            if (match) return Promise.resolve(line);
        }
        return new Promise<string>((resolve) => {
            lineCallbacks.push({ pat, resolve });
        });
    };

    return { process: child, lines: linesPromise, waitForLine };
}

describe('WAMP Client Compatibility Tests', () => {
    let server: any;
    let controlClient: autobahn.Connection;
    let serverPort: number;
    let controlSessionDetails: any = null;
    const controlRegistrations: any[] = [];
 
    before(async () => {
        serverPort = 9001;
        
        // Start Crossbar locally
        const crossbarBin = path.resolve(__dirname, '../venv/bin/crossbar');
        const cbDir = path.resolve(__dirname, '../.crossbar');
        const configPath = path.resolve(__dirname, '../.crossbar/config.json');
        server = spawn(crossbarBin, [
            'start',
            '--cbdir', cbDir,
            '--config', configPath
        ]);

        server.stdout.on('data', (data: any) => {
            console.log(`[CROSSBAR] ${data.toString().trim()}`);
        });

        server.stderr.on('data', (data: any) => {
            console.error(`[CROSSBAR ERR] ${data.toString().trim()}`);
        });

        // Wait for port 9001 to open
        await waitForPort(serverPort);

        // Connect control client
        controlClient = new autobahn.Connection({
            url: `ws://127.0.0.1:${serverPort}`,
            realm: 'com.compat.test',
            max_retries: 0,
            authid: 'test-user'
        });

        await new Promise<void>((resolve, reject) => {
            controlClient.onopen = (session, details) => {
                controlSessionDetails = details;
                resolve();
            };
            controlClient.onclose = (reason) => reject(new Error(`Control client closed: ${reason}`));
            controlClient.open();
        });
    });

    after(async () => {
        if (controlClient) {
            controlClient.close();
        }
        if (server) {
            server.kill();
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
        await new Promise(r => setTimeout(r, 50));
    });

    test('T1.2: Joining Invalid Realm (Client Check)', async () => {
        const client = runRustClient(serverPort, 'client-invalid-realm');
        const lines = await client.lines;
        assert.ok(lines.some(l => l.includes('SUCCESS: rejected with error')));
    });

    test('T3.1: Pub/Sub Basic (Client Scenario)', async () => {
        const client = runRustClient(serverPort, 'pubsub');
        const lines = await client.lines;
        assert.ok(lines.includes('SUBSCRIBED'));
        assert.ok(lines.includes('PUBLISHED'));
        assert.ok(lines.includes('UNSUBSCRIBED'));
    });

    test('T3.2: Pub/Sub Basic (TS Subscriber, Rust Publisher)', async () => {
        // Subscribe from the TS control client before the Rust client runs
        const receivedArgs: any[] = [];
        const receivedKwargs: any[] = [];
        const sub = await controlClient.session!.subscribe(
            'com.compat.topic',
            (args: any[], kwargs: any) => {
                receivedArgs.push(...args);
                receivedKwargs.push(kwargs);
            }
        );

        // Rust pubsub scenario publishes [123, "test"] with kwargs {foo: "bar"}
        const client = runRustClient(serverPort, 'pubsub');
        await client.waitForLine('PUBLISHED');

        // Allow the event to propagate
        await new Promise(r => setTimeout(r, 200));

        assert.strictEqual(receivedArgs[0], 123);
        assert.strictEqual(receivedArgs[1], 'test');
        assert.strictEqual(receivedKwargs[0]?.foo, 'bar');

        await controlClient.session!.unsubscribe(sub);
        await client.lines;
    });

    test('T3.3: Prefix-based Subscription Matching (Client Scenario)', async () => {
        const client = runRustClient(serverPort, 'pubsub-prefix');
        await client.waitForLine('SUBSCRIBED');

        // Control client publishes to subtopic
        controlClient.session!.publish('com.compat.prefix_subtopic', [42]);

        const lines = await client.lines;
        assert.ok(lines.some(l => l.includes('EVENT:') && l.includes('42') && l.includes('com.compat.prefix_subtopic')));
    });

    test('T3.4: Wildcard-based Subscription Matching (Client Scenario)', async () => {
        const client = runRustClient(serverPort, 'pubsub-wildcard');
        await client.waitForLine('SUBSCRIBED');

        // Control client publishes to wildcard topic
        controlClient.session!.publish('com.compat.foo.status', [100]);

        const lines = await client.lines;
        assert.ok(lines.some(l => l.includes('EVENT:') && l.includes('100') && l.includes('com.compat.foo.status')));
    });

    test('T4.1: Basic RPC (Callee Client Scenario)', async () => {
        const client = runRustClient(serverPort, 'rpc-callee');
        await client.waitForLine('REGISTERED');

        // Control client calls the Rust procedure
        const res = await controlClient.session!.call('com.compat.proc', [12]);
        assert.strictEqual(res, 24); // Callee scenario doubles the value

        const lines = await client.lines;
        assert.ok(lines.includes('RESPONDED'));
        assert.ok(lines.includes('UNREGISTERED'));
    });

    test('T4.1: Basic RPC (Caller Client Scenario)', async () => {
        // Control client registers the procedure
        const reg = await controlClient.session!.register('com.compat.proc', (args) => {
            return (args[0] as number) * 3;
        });
        controlRegistrations.push(reg);

        const client = runRustClient(serverPort, 'rpc-caller');
        const lines = await client.lines;
        assert.ok(lines.some(l => l.includes('RESULT:') && l.includes('30'))); // 10 * 3

        await controlClient.session!.unregister(reg);
    });

    test('T4.2: RPC Error Propagation (TS Callee, Rust Caller)', async () => {
        // Control client registers a procedure that throws a WAMP application error
        const reg = await controlClient.session!.register(
            'com.compat.error_proc',
            () => { throw new (autobahn as any).ApplicationError('com.compat.error.custom', ['ts error payload']); }
        );
        controlRegistrations.push(reg);

        const client = runRustClient(serverPort, 'rpc-caller-error');
        const lines = await client.lines;
        assert.ok(lines.some(l => l.includes('CALL_ERROR')),
            'Rust caller must receive the WAMP error thrown by the TS callee');
    });

    test('T4.3: Unregistering a Procedure (Client Scenario)', async () => {
        const client = runRustClient(serverPort, 'rpc-unregister');
        const lines = await client.lines;
        assert.ok(lines.includes('REGISTERED'));
        assert.ok(lines.includes('UNREGISTERED'));
    });

    test('T4.4: RPC Error Propagation (Callee Client Scenario)', async () => {
        const client = runRustClient(serverPort, 'rpc-error');
        await client.waitForLine('REGISTERED');

        await assert.rejects(
            controlClient.session!.call('com.compat.error_proc'),
            (err: any) => {
                console.error("T4.4 ERROR DETAILS:", {
                    error: err.error,
                    message: err.message,
                    args: err.args,
                    kwargs: err.kwargs,
                    details: err.details
                });
                assert.strictEqual(err.error, 'com.compat.error.custom');
                assert.strictEqual(err.args[0], 'custom callee error payload');
                return true;
            }
        );

        await client.lines;
    });

    test('T4.5: Shared Registration (Client Scenario)', async () => {
        // --- Single policy: only one callee may register ---

        // Client A registers com.compat.shared with the default single policy
        const singleA = runRustClient(serverPort, 'rpc-shared');
        await singleA.waitForLine('REGISTERED_SHARED');

        // Client B attempts to register the same URI under single policy - should fail
        const singleB = runRustClient(serverPort, 'rpc-shared', ['--auth-secret', 'policy:single']);
        const linesSingleB = await singleB.lines;
        assert.ok(linesSingleB.some(l => l.includes('REGISTER_FAILED')),
            'Second single-policy registration should be rejected');

        // Control client calls the procedure - client A should respond with its client name
        const singleResult = await controlClient.session!.call('com.compat.shared') as string;
        assert.strictEqual(singleResult, 'rust-client-rpc-shared');

        await singleA.lines;

        // --- Round-robin policy: multiple callees may register ---

        // Both clients register com.compat.shared with roundrobin policy
        const rrA = runRustClient(serverPort, 'rpc-shared', ['--auth-secret', 'policy:roundrobin']);
        const rrB = runRustClient(serverPort, 'rpc-shared', ['--auth-secret', 'policy:roundrobin']);

        // Wait for both to be registered before calling
        await Promise.all([
            rrA.waitForLine('REGISTERED_SHARED'),
            rrB.waitForLine('REGISTERED_SHARED'),
        ]);

        // First call: race to see which client prints RESPONDED first
        const rrResult1 = await controlClient.session!.call('com.compat.shared') as string;
        assert.strictEqual(rrResult1, 'rust-client-rpc-shared');

        const firstResponded = await Promise.race([
            rrA.waitForLine('RESPONDED').then(() => 'A' as const),
            rrB.waitForLine('RESPONDED').then(() => 'B' as const),
        ]);

        // Second call: the other client must respond (round-robin)
        const rrResult2 = await controlClient.session!.call('com.compat.shared') as string;
        assert.strictEqual(rrResult2, 'rust-client-rpc-shared');

        // Only wait on the client that hasn't responded yet
        const secondResponded = firstResponded === 'A'
            ? await rrB.waitForLine('RESPONDED').then(() => 'B' as const)
            : await rrA.waitForLine('RESPONDED').then(() => 'A' as const);

        assert.notStrictEqual(firstResponded, secondResponded,
            'Round-robin must dispatch each call to a different callee');

        await Promise.all([rrA.lines, rrB.lines]);
    });

    test('T4.6: Progressive Call Results (TS Caller, Rust Callee)', async () => {
        const client = runRustClient(serverPort, 'rpc-callee-progressive');
        await client.waitForLine('REGISTERED');

        // Call with receive_progress so the Rust callee's progressive yields reach us.
        // Autobahn uses when.js deferred notify for progress; for single positional
        // args it passes the scalar directly (not a Result object).
        const progressResults: number[] = [];
        const finalResult = await new Promise<number>((resolve, reject) => {
            const d = (controlClient.session as any).call(
                'com.compat.callee_progress',
                [],
                {},
                { receive_progress: true }
            );
            d.then(
                (res: any) => resolve(res?.args ? res.args[0] : res),
                (err: any) => reject(err),
                (p: any) => { progressResults.push(p?.args ? p.args[0] : p); }
            );
        });

        assert.deepStrictEqual(progressResults, [10, 20]);
        assert.strictEqual(finalResult, 30);

        const lines = await client.lines;
        assert.ok(lines.includes('PROGRESS_SENT: 10'));
        assert.ok(lines.includes('PROGRESS_SENT: 20'));
        assert.ok(lines.includes('RESPONDED'));
    });

    test('T4.7: Caller Identification (Callee Client Scenario)', async () => {
        const client = runRustClient(serverPort, 'rpc-disclose-caller');
        await client.waitForLine('REGISTERED');

        // Control client calls procedure disclosing caller identity
        const res = await controlClient.session!.call('com.compat.disclose', [], {}, { disclose_me: true });
        assert.strictEqual(res, controlSessionDetails.authid);

        await client.lines;
    });

    test('T4.8: RPC Call Timeout (Caller Client Scenario)', async () => {
        // Control client registers a slow procedure
        const reg = await controlClient.session!.register('com.compat.slow_proc', async () => {
            await new Promise(r => setTimeout(r, 1000));
            return 'late';
        });
        controlRegistrations.push(reg);

        const client = runRustClient(serverPort, 'rpc-timeout');
        const lines = await client.lines;
        assert.ok(lines.some(l => l.includes('TIMEOUT_SUCCESS')));

        await controlClient.session!.unregister(reg);
    });

    test('T4.9: Caller-Initiated Call Cancellation (Callee Client Scenario)', async () => {
        const client = runRustClient(serverPort, 'rpc-cancel');
        await client.waitForLine('REGISTERED');

        // Call the slow procedure
        const callPromise = controlClient.session!.call('com.compat.slow_proc');

        await client.waitForLine('INVOCATION_RECEIVED');

        // Cancel the call
        callPromise.cancel({ mode: 'kill' });

        await assert.rejects(callPromise, (err: any) => {
            assert.strictEqual(err.error, 'wamp.error.canceled');
            return true;
        });

        const lines = await client.lines;
        assert.ok(lines.includes('CANCEL_RECEIVED'));
        assert.ok(lines.includes('CANCEL_RESPONDED'));
    });

    test('T4.10: Progressive Call Results (Caller Client Scenario)', async () => {
        // Control client registers progressive procedure
        const reg = await controlClient.session!.register('com.compat.progress', (args, kwargs, details) => {
            if (details.progress) {
                details.progress([10]);
                details.progress([20]);
            }
            return 30;
        });
        controlRegistrations.push(reg);

        const client = runRustClient(serverPort, 'rpc-progressive');
        const lines = await client.lines;
        assert.ok(lines.some(l => l.includes('PROGRESS_RESULT: [Integer(10)] progress: true')));
        assert.ok(lines.some(l => l.includes('PROGRESS_RESULT: [Integer(20)] progress: true')));
        assert.ok(lines.some(l => l.includes('PROGRESS_RESULT: [Integer(30)] progress: false')));

        await controlClient.session!.unregister(reg);
    });
});
