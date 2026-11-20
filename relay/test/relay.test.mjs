// Tests for the relay, run against `wrangler dev` by ../test.sh (which sets
// RELAY to its base URL). Uses Node's built-in WebSocket and test runner.

import { afterEach, test } from "node:test";
import assert from "node:assert/strict";

const RELAY = process.env.RELAY ?? "ws://127.0.0.1:8787";
const HTTP = RELAY.replace(/^ws/, "http");
/** How long to wait for a message that should come. */
const WAIT = 3000;
/** How long to wait to be sure a message does not come. */
const QUIET = 300;

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

/** A fresh channel name, so tests don't see each other. */
function channel() {
	const abc = "abcdefghijklmnopqrstuvwxyz0123456789";
	const word = () => {
		let s = "";
		for (let i = 0; i < 5; i++) s += abc[Math.floor(Math.random() * abc.length)];
		return s;
	};
	return `${word()}-${word()}-${word()}`;
}

/** Every client, so a failed test doesn't leave the run hanging. */
const clients = new Set();
afterEach(() => {
	for (const c of clients) c.ws.close();
	clients.clear();
});

class Client {
	static async open(ch, path = "/openwgt/relay/") {
		const c = new Client(new WebSocket(`${RELAY}${path}${ch}`));
		await c.opened;
		return c;
	}

	/** Opens and says hello. */
	static async join(ch, role, run = false) {
		const c = await Client.open(ch);
		c.send({ t: "hello", role, run, v: 1 });
		return c;
	}

	constructor(ws) {
		clients.add(this);
		this.ws = ws;
		this.inbox = [];
		this.waiting = null;
		this.opened = new Promise((resolve, reject) => {
			ws.onopen = resolve;
			ws.onerror = reject;
		});
		this.closed = new Promise((resolve) => {
			ws.onclose = (e) => {
				this.wake();
				resolve({ code: e.code, reason: e.reason });
			};
		});
		ws.onmessage = (e) => {
			this.inbox.push(JSON.parse(e.data));
			this.wake();
		};
	}

	wake() {
		const w = this.waiting;
		this.waiting = null;
		w?.();
	}

	send(msg) {
		this.ws.send(typeof msg === "string" ? msg : JSON.stringify(msg));
	}

	/** The next message, or fails after `ms`. */
	async next(ms = WAIT) {
		const until = Date.now() + ms;
		while (this.inbox.length === 0) {
			const left = until - Date.now();
			if (left <= 0 || this.ws.readyState >= WebSocket.CLOSING) {
				assert.fail(`no message within ${ms} ms`);
			}
			await new Promise((r) => {
				this.waiting = r;
				setTimeout(r, left);
			});
		}
		return this.inbox.shift();
	}

	/** Expects exactly these messages next, in any order. */
	async expect(...msgs) {
		const got = [];
		for (let i = 0; i < msgs.length; i++) got.push(await this.next());
		const key = (m) => JSON.stringify(m);
		assert.deepEqual(got.map(key).sort(), msgs.map(key).sort());
	}

	/** Expects these messages next, in this order. */
	async expectInOrder(...msgs) {
		for (const m of msgs) assert.deepEqual(await this.next(), m);
	}

	/** Expects nothing more for a while. */
	async quiet(ms = QUIET) {
		await sleep(ms);
		assert.deepEqual(this.inbox, [], "expected no messages");
	}

	async closedWith(code) {
		const c = await Promise.race([this.closed, sleep(WAIT).then(() => null)]);
		assert.ok(c, "not closed");
		assert.equal(c.code, code, `closed with ${c.code} (${c.reason})`);
	}

	async close() {
		this.ws.close(1000);
		await this.closed;
	}
}

const peer = (s) => ({ t: "peer", s });
const busy = (b) => ({ t: "busy", busy: b });
const val = (k, v) => ({ t: "val", k, v });

test("a receiver alone hears peer none", async () => {
	const r = await Client.join(channel(), "recv");
	await r.expect(peer("none"));
	await r.quiet();
	await r.close();
});

test("a sender alone hears busy false and peer none", async () => {
	const s = await Client.join(channel(), "send", true);
	await s.expect(busy(false), peer("none"));
	await s.quiet();
	await s.close();
});

test("peer states follow joins, runs and leaves on both sides", async () => {
	const ch = channel();
	const r1 = await Client.join(ch, "recv");
	await r1.expect(peer("none"));

	const s = await Client.join(ch, "send");
	await s.expect(busy(false), peer("open"));
	await r1.expect(peer("open"));

	r1.send({ t: "run", run: true });
	await s.expect(peer("run"));
	s.send({ t: "run", run: true });
	await r1.expect(peer("run"));
	s.send({ t: "run", run: true }); // no change, no message
	await r1.quiet();
	s.send({ t: "run", run: false });
	await r1.expect(peer("open"));

	// The sender sees the best of the receivers: run > open > none.
	const r2 = await Client.join(ch, "recv");
	await r2.expect(peer("open"));
	await s.quiet();
	await r1.close();
	await s.expect(peer("open"));
	const r3 = await Client.join(ch, "recv", true);
	await r3.expect(peer("open"));
	await s.expect(peer("run"));
	await r3.close();
	await s.expect(peer("open"));
	await r2.close();
	await s.expect(peer("none"));

	s.send({ t: "run", run: true });
	const r4 = await Client.join(ch, "recv");
	await r4.expect(peer("run"));
	await s.close();
	await r4.expect(peer("none"));
	await r4.close();
});

test("values go from the sender to every receiver, and invalid ones are dropped", async () => {
	const ch = channel();
	const s = await Client.join(ch, "send", true);
	await s.expect(busy(false), peer("none"));
	const r1 = await Client.join(ch, "recv", true);
	const r2 = await Client.join(ch, "recv");
	await r1.expect(peer("run"));
	await r2.expect(peer("run"));
	await s.expect(peer("run"));

	for (const [k, v] of [["b", true], ["b", false], ["n", -2.5], ["n", 1e300], ["p", 0], ["p", 4294967295]]) {
		s.send({ t: "val", k, v });
		await r1.expect(val(k, v));
		await r2.expect(val(k, v));
	}
	for (const bad of [
		{ t: "val", k: "b", v: 1 },
		{ t: "val", k: "n", v: "3" },
		{ t: "val", k: "n", v: null },
		{ t: "val", k: "p", v: -1 },
		{ t: "val", k: "p", v: 1.5 },
		{ t: "val", k: "p", v: 4294967296 },
		{ t: "val", k: "z", v: 1 },
		{ t: "val", v: 1 },
		{ t: "nonsense" },
		[1, 2],
		"not json",
		"42",
	]) {
		s.send(typeof bad === "string" ? bad : JSON.stringify(bad));
	}
	// A receiver's value goes nowhere.
	r1.send({ t: "val", k: "n", v: 7 });
	await r1.quiet();
	await r2.quiet();
	await s.quiet();
	// Still working.
	s.send({ t: "val", k: "n", v: 3 });
	await r1.expect(val("n", 3));
	await r2.expect(val("n", 3));
	await Promise.all([s.close(), r1.close(), r2.close()]);
});

test("a joining receiver gets the last value, but not after the sender has left", async () => {
	const ch = channel();
	const s = await Client.join(ch, "send", true);
	await s.expect(busy(false), peer("none"));
	// Sent with nobody listening: still kept.
	s.send({ t: "val", k: "n", v: 1 });
	s.send({ t: "val", k: "n", v: 3.5 });
	await sleep(100);
	const r1 = await Client.join(ch, "recv");
	await r1.expectInOrder(peer("run"), val("n", 3.5));
	await r1.quiet();

	await s.close();
	await r1.expect(peer("none"));
	const r2 = await Client.join(ch, "recv");
	await r2.expect(peer("none"));
	await r2.quiet();

	// A new sender starts with nothing to pass on.
	const s2 = await Client.join(ch, "send");
	await s2.expect(busy(false), peer("open"));
	await r1.expect(peer("open"));
	const r3 = await Client.join(ch, "recv");
	await r3.expect(peer("open"));
	await r3.quiet();
	await Promise.all([s2.close(), r1.close(), r2.close(), r3.close()]);
});

test("a second sender is busy until the first leaves, then takes over", async () => {
	const ch = channel();
	const r = await Client.join(ch, "recv", true);
	await r.expect(peer("none"));
	const s1 = await Client.join(ch, "send", true);
	await s1.expect(busy(false), peer("run"));
	await r.expect(peer("run"));
	s1.send({ t: "val", k: "b", v: true });
	await r.expect(val("b", true));

	const s2 = await Client.join(ch, "send");
	await s2.expect(busy(true), peer("run"));
	const s3 = await Client.join(ch, "send", true);
	await s3.expect(busy(true), peer("run"));
	// Waiting senders' values and runs don't reach the receivers.
	s2.send({ t: "val", k: "n", v: 9 });
	s3.send({ t: "run", run: false });
	await r.quiet();
	await s1.quiet();

	// The first leaves: the longest waiting (s2) takes over. The receiver
	// hears none (its value is gone) and then the new sender's state.
	await s1.close();
	await s2.expect(busy(false));
	await r.expectInOrder(peer("none"), peer("open"));
	await s3.quiet();
	// Its earlier value was dropped, so a new receiver gets nothing yet.
	const r2 = await Client.join(ch, "recv");
	await r2.expect(peer("open"));
	await r2.quiet();
	s2.send({ t: "val", k: "n", v: 10 });
	await r.expect(val("n", 10));
	await r2.expect(val("n", 10));

	// A waiting sender that leaves changes nothing for the others.
	await s3.close();
	await r.quiet();
	await s2.quiet();

	// The receivers leaving: the running one, then the last.
	await r.close();
	await s2.expect(peer("open"));
	await r2.close();
	await s2.expect(peer("none"));
	await s2.close();
});

test("ping gets pong, before and after hello", async () => {
	const ch = channel();
	const c = await Client.open(ch);
	c.send({ t: "ping" });
	await c.expect({ t: "pong" });
	c.send('{ "t": "ping" }'); // not the exact text: answered by the object
	await c.expect({ t: "pong" });
	c.send({ t: "hello", role: "recv", run: false, v: 1 });
	await c.expect(peer("none"));
	c.send({ t: "ping" });
	await c.expect({ t: "pong" });
	await c.close();
});

test("the workers.dev path works too", async () => {
	const c = await Client.open(channel(), "/relay/");
	c.send({ t: "hello", role: "recv", run: false, v: 1 });
	await c.expect(peer("none"));
	await c.close();
});

test("bad channel names are closed with 4000", async () => {
	for (const ch of ["", "ABC", "a_b", "-ab", "ab-", "a--b", "abcdefghij-klmnopqrst", "a/b", "%C3%A5", "a%20b"]) {
		const c = await Client.open(ch);
		await c.closedWith(4000);
	}
});

test("channel names of up to 20 with hyphens are fine", async () => {
	for (const ch of ["a", "cat-sun-oak", "abcdefghij-klmnopqrs", "kiwi42"]) {
		const c = await Client.open(ch);
		c.send({ t: "hello", role: "recv", run: false, v: 1 });
		await c.expect(peer("none"));
		await c.close();
	}
});

test("an unsupported version is closed with 4001, a bad hello with 4002", async () => {
	for (const v of [2, 0, "1", undefined]) {
		const c = await Client.open(channel());
		c.send({ t: "hello", role: "recv", run: false, v });
		await c.closedWith(4001);
	}
	for (const hello of [
		{ t: "hello", role: "both", run: false, v: 1 },
		{ t: "hello", role: "send", v: 1 },
	]) {
		const c = await Client.open(channel());
		c.send(hello);
		await c.closedWith(4002);
	}
});

test("messages before hello are dropped, and a second hello is ignored", async () => {
	const ch = channel();
	const s = await Client.join(ch, "send");
	await s.expect(busy(false), peer("none"));
	const r = await Client.open(ch);
	r.send({ t: "run", run: true });
	await s.quiet();
	r.send({ t: "hello", role: "recv", run: false, v: 1 });
	await r.expect(peer("open"));
	await s.expect(peer("open"));
	r.send({ t: "hello", role: "send", run: true, v: 1 });
	await r.quiet();
	await s.quiet();
	await Promise.all([s.close(), r.close()]);
});

test("an oversize message is closed with 1009, binary with 1003", async () => {
	const c = await Client.join(channel(), "send");
	await c.expect(busy(false), peer("none"));
	c.send({ t: "val", k: "n", v: 1, pad: "x".repeat(256) });
	await c.closedWith(1009);

	// 256 bytes is still fine, counted in bytes, not characters.
	const ch = channel();
	const s = await Client.join(ch, "send");
	const r = await Client.join(ch, "recv");
	await r.expect(peer("open"));
	const base = JSON.stringify({ t: "val", k: "b", v: true, pad: "" });
	s.send(JSON.stringify({ t: "val", k: "b", v: true, pad: "x".repeat(256 - base.length) }));
	await r.expect(val("b", true));
	// Under 256 characters, but "å" is two bytes each: over.
	s.send(JSON.stringify({ t: "val", k: "b", v: true, pad: "å".repeat(256 - base.length - 80) }));
	await s.closedWith(1009);

	const b = await Client.open(channel());
	b.ws.send(new Uint8Array([1, 2, 3]));
	await b.closedWith(1003);
	await r.close();
});

test("a full channel closes the 33rd connection with 4029", async () => {
	const ch = channel();
	const conns = [];
	for (let i = 0; i < 32; i++) conns.push(await Client.open(ch));
	const extra = await Client.open(ch);
	await extra.closedWith(4029);
	// One leaves: there's room again.
	await conns.pop().close();
	const again = await Client.open(ch);
	again.send({ t: "hello", role: "recv", run: false, v: 1 });
	await again.expect(peer("none"));
	await Promise.all([again, ...conns].map((c) => c.close()));
});

test("more than 60 messages a second are dropped, and keeping on closes with 4008", async () => {
	const ch = channel();
	const s = await Client.join(ch, "send");
	const r = await Client.join(ch, "recv");
	await r.expect(peer("open"));
	await s.expect(busy(false), peer("none"), peer("open"));

	// Start a burst early in a second, so it stays inside one.
	const burst = async (n, from) => {
		await sleep(1000 - (Date.now() % 1000) + 20);
		for (let i = 0; i < n; i++) s.send({ t: "val", k: "n", v: from + i });
	};
	await burst(100, 0);
	await sleep(1100);
	const got = r.inbox.splice(0);
	assert.equal(got.length, 60, `got ${got.length} of 100`);
	assert.deepEqual(got.at(-1), val("n", 59));

	// Over the rate in three seconds out of ten: closed.
	await burst(100, 100);
	await burst(100, 200);
	await s.closedWith(4008);
	await r.close();
});

test("plain HTTP gets 426, other paths 404", async () => {
	let res = await fetch(`${HTTP}/openwgt/relay/abc`);
	assert.equal(res.status, 426);
	res = await fetch(`${HTTP}/openwgt/other`);
	assert.equal(res.status, 404);
	res = await fetch(`${HTTP}/`);
	assert.equal(res.status, 404);
});
