// The relay for OpenWGT's network parts: a Worker that checks the request
// and hands it to one Durable Object per channel, which passes a Sender's
// value on to the Receivers on the same channel. The protocol is in
// ../PROTOCOL.md; this follows it, and the README says where it goes beyond.
//
// The Durable Object uses the WebSocket Hibernation API: between messages
// it may be evicted from memory while its sockets stay open, so a channel
// nobody is using costs nothing. Everything it knows about a connection
// lives in that socket's attachment (role, run, join order, what it was
// last told, and for the active Sender its last value), so it all survives
// hibernation; nothing is written to storage.

import { DurableObject } from "cloudflare:workers";

/** Where the relay answers: on gustavn.se and on workers.dev. */
const PREFIXES = ["/openwgt/relay/", "/relay/"];
/** A channel name: 1 to 20 of a-z, 0-9 and single hyphens between them. */
const CHANNEL = /^(?=.{1,20}$)[a-z0-9]+(-[a-z0-9]+)*$/;
const VERSION = 1;
const MAX_BYTES = 256;
const MAX_CONNECTIONS = 32;
/** Messages a connection may send per second; more are dropped. */
const RATE = 60;
/** Seconds over the rate, within `STRIKE_WINDOW` seconds, that close it. */
const STRIKES = 3;
const STRIKE_WINDOW = 10;

const PING = '{"t":"ping"}';
const PONG = '{"t":"pong"}';

/** Close codes (the 4xxx are the protocol's, the 1xxx the standard's). */
const CLOSE = {
	normal: 1000,
	notText: 1003,
	tooBig: 1009,
	badChannel: 4000,
	badVersion: 4001,
	badHello: 4002,
	abuse: 4008,
	full: 4029,
} as const;

type Role = "send" | "recv";
type Peer = "none" | "open" | "run";
type Val = { k: "b"; v: boolean } | { k: "n"; v: number } | { k: "p"; v: number };

/** What the relay keeps about a connection, in its socket's attachment. */
interface Conn {
	/** Join order: unique on the channel, larger is later. */
	order: number;
	/** Unset until the hello. */
	role?: Role;
	run: boolean;
	/** The peer state this connection was last told. */
	peer?: Peer;
	/** Senders: whether it was last told it is busy. */
	busy?: boolean;
	/** The active Sender: its last value, for Receivers that join. */
	last?: Val;
}

export default {
	async fetch(request: Request, env: Env): Promise<Response> {
		const path = new URL(request.url).pathname;
		const prefix = PREFIXES.find((p) => path.startsWith(p));
		if (prefix === undefined) {
			return new Response("Not found\n", { status: 404 });
		}
		if (request.headers.get("Upgrade")?.toLowerCase() !== "websocket") {
			return new Response("This is a WebSocket endpoint\n", {
				status: 426,
				headers: { Upgrade: "websocket" },
			});
		}
		const channel = path.slice(prefix.length);
		if (!CHANNEL.test(channel)) {
			return refuse(CLOSE.badChannel, "bad channel");
		}
		return env.CHANNEL.get(env.CHANNEL.idFromName(channel)).fetch(request);
	},
} satisfies ExportedHandler<Env>;

/** Accepts the WebSocket only to close it at once with `code`. */
function refuse(code: number, reason: string): Response {
	const [client, server] = Object.values(new WebSocketPair());
	server.accept();
	server.close(code, reason);
	return new Response(null, { status: 101, webSocket: client });
}

function conn(ws: WebSocket): Conn {
	return ws.deserializeAttachment() as Conn;
}

function send(ws: WebSocket, msg: object) {
	try {
		ws.send(JSON.stringify(msg));
	} catch {
		// Closing already; its close event tidies up.
	}
}

/** A `val` message's value if it is a valid one, else undefined. */
function parseVal(m: { k?: unknown; v?: unknown }): Val | undefined {
	const { k, v } = m;
	if (k === "b" && typeof v === "boolean") return { k, v };
	if (k === "n" && typeof v === "number" && Number.isFinite(v)) return { k, v };
	if (k === "p" && typeof v === "number" && Number.isInteger(v) && v >= 0 && v <= 0xffffffff) {
		return { k, v };
	}
	return undefined;
}

/** A connection's rate count (in memory only: it matters only while busy). */
interface Rate {
	second: number;
	count: number;
	/** The seconds in which it went over the rate. */
	over: number[];
}

export class Channel extends DurableObject<Env> {
	/** Join orders of connections that have left but may still be listed. */
	private gone = new Set<number>();
	private rates = new Map<number, Rate>();

	constructor(ctx: DurableObjectState, env: Env) {
		super(ctx, env);
		// The runtime answers the usual ping itself, without waking us.
		ctx.setWebSocketAutoResponse(new WebSocketRequestResponsePair(PING, PONG));
	}

	async fetch(request: Request): Promise<Response> {
		const all = this.ctx.getWebSockets();
		const orders = all.map((ws) => conn(ws).order);
		for (const o of this.gone) {
			if (!orders.includes(o)) this.gone.delete(o);
		}
		const open = all.filter(
			(ws) => ws.readyState === WebSocket.OPEN && !this.gone.has(conn(ws).order),
		);
		if (open.length >= MAX_CONNECTIONS) {
			return refuse(CLOSE.full, "channel full");
		}
		const [client, server] = Object.values(new WebSocketPair());
		this.ctx.acceptWebSocket(server);
		const c: Conn = { order: Math.max(0, ...orders) + 1, run: false };
		server.serializeAttachment(c);
		return new Response(null, { status: 101, webSocket: client });
	}

	async webSocketMessage(ws: WebSocket, message: string | ArrayBuffer) {
		if (typeof message !== "string") {
			return this.close(ws, CLOSE.notText, "text frames only");
		}
		if (message.length > MAX_BYTES || new TextEncoder().encode(message).length > MAX_BYTES) {
			return this.close(ws, CLOSE.tooBig, "message too big");
		}
		const c = conn(ws);
		if (!this.allow(ws, c)) return;

		let m: unknown;
		try {
			m = JSON.parse(message);
		} catch {
			return;
		}
		if (typeof m !== "object" || m === null) return;
		const msg = m as Record<string, unknown>;

		switch (msg.t) {
			case "ping":
				ws.send(PONG);
				return;
			case "hello":
				return this.hello(ws, c, msg);
			case "run":
				if (c.role === undefined || typeof msg.run !== "boolean" || msg.run === c.run) return;
				c.run = msg.run;
				ws.serializeAttachment(c);
				this.update();
				return;
			case "val": {
				if (c.role !== "send") return;
				const val = parseVal(msg);
				if (val === undefined) return;
				const { active, receivers } = this.roles();
				if (active?.ws !== ws) return;
				c.last = val;
				ws.serializeAttachment(c);
				for (const r of receivers) send(r.ws, { t: "val", ...val });
				return;
			}
		}
	}

	async webSocketClose(ws: WebSocket, code: number) {
		try {
			// Echo the close (a newer runtime does this itself).
			ws.close(code === 1005 || code === 1006 ? CLOSE.normal : code, "bye");
		} catch {
			// Closed already.
		}
		this.left(ws);
	}

	async webSocketError(ws: WebSocket) {
		this.left(ws);
	}

	private hello(ws: WebSocket, c: Conn, msg: Record<string, unknown>) {
		if (c.role !== undefined) return; // once only
		if (msg.v !== VERSION) {
			return this.close(ws, CLOSE.badVersion, "unsupported version");
		}
		if ((msg.role !== "send" && msg.role !== "recv") || typeof msg.run !== "boolean") {
			return this.close(ws, CLOSE.badHello, "bad hello");
		}
		c.role = msg.role;
		c.run = msg.run;
		ws.serializeAttachment(c);
		this.update();
		if (c.role === "recv") {
			const last = this.roles().active?.c.last;
			if (last !== undefined) send(ws, { t: "val", ...last });
		}
	}

	/** Closes from our side and drops the connection from the channel. */
	private close(ws: WebSocket, code: number, reason: string) {
		try {
			ws.close(code, reason);
		} catch {
			// Closed already.
		}
		this.left(ws);
	}

	/** A connection has left: tell the others what changed for them. */
	private left(ws: WebSocket) {
		const c = conn(ws);
		if (this.gone.has(c.order)) return;
		const before = this.roles();
		this.gone.add(c.order);
		this.rates.delete(c.order);
		if (before.active?.c.order === c.order) {
			// The Sender the Receivers listened to has gone. They hear "none"
			// even when a waiting Sender takes over at once (and then that
			// one's state), so they drop its value either way.
			for (const r of before.receivers) this.tell(r.ws, r.c, "none");
		}
		this.update();
	}

	/** Who is on the channel now (only those past their hello). */
	private roles() {
		const live = this.ctx
			.getWebSockets()
			.map((ws) => ({ ws, c: conn(ws) }))
			.filter(({ c }) => !this.gone.has(c.order));
		const senders = live.filter(({ c }) => c.role === "send").sort((a, b) => a.c.order - b.c.order);
		const receivers = live.filter(({ c }) => c.role === "recv");
		return { senders, receivers, active: senders[0] as { ws: WebSocket; c: Conn } | undefined };
	}

	/** Tells every connection its peer state and busy flag if they changed. */
	private update() {
		const { senders, receivers, active } = this.roles();
		const senderState: Peer = active === undefined ? "none" : active.c.run ? "run" : "open";
		const receiverState: Peer = receivers.some(({ c }) => c.run)
			? "run"
			: receivers.length > 0
				? "open"
				: "none";
		for (const { ws, c } of senders) {
			const busy = c.order !== active?.c.order;
			if (c.busy !== busy) {
				c.busy = busy;
				ws.serializeAttachment(c);
				send(ws, { t: "busy", busy });
			}
			this.tell(ws, c, receiverState);
		}
		for (const { ws, c } of receivers) this.tell(ws, c, senderState);
	}

	private tell(ws: WebSocket, c: Conn, peer: Peer) {
		if (c.peer === peer) return;
		c.peer = peer;
		ws.serializeAttachment(c);
		send(ws, { t: "peer", s: peer });
	}

	/** Counts a message against the rate: false drops it (and may close). */
	private allow(ws: WebSocket, c: Conn): boolean {
		const second = Math.floor(Date.now() / 1000);
		let rate = this.rates.get(c.order);
		if (rate === undefined) {
			rate = { second, count: 0, over: [] };
			this.rates.set(c.order, rate);
		}
		if (rate.second !== second) {
			rate.second = second;
			rate.count = 0;
		}
		rate.count += 1;
		if (rate.count <= RATE) return true;
		if (rate.count === RATE + 1) {
			rate.over = rate.over.filter((s) => s > second - STRIKE_WINDOW);
			rate.over.push(second);
			if (rate.over.length >= STRIKES) {
				this.close(ws, CLOSE.abuse, "too many messages");
			}
		}
		return false;
	}
}
