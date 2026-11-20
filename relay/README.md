# openwgt-relay

The relay for OpenWGT's network parts: a Sender part in one player's
machine sends its input (on/off, a number or a picture) to the Receiver
parts with the same channel name in other players' machines. This is the
server in between, a Cloudflare Worker with one Durable Object per channel.

It is its own small TypeScript project, apart from the Rust workspace:
cargo doesn't see it, the flake doesn't take it in, and CI ignores pushes
that change only `relay/`.

## Protocol

[PROTOCOL.md](PROTOCOL.md) is the protocol (v1), shared with the game. In
short: a WebSocket to `wss://gustavn.se/openwgt/relay/<channel>`, JSON
text frames; the client says `hello` (role `send` or `recv`, run, version),
then `run` when GO/STOP changes and, as a Sender, `val`; the relay tells
each side the other's state (`peer`), tells Senders whether another Sender
holds the channel (`busy`), passes the active Sender's values to the
Receivers and answers `ping` with `pong`.

Where the spec leaves room, the relay does this:

- **Paths.** `/openwgt/relay/<channel>` and `/relay/<channel>` (for the
  workers.dev address), on any host. Another path is a 404; a request
  without a WebSocket upgrade a 426. A channel name is 1 to 20 of `a-z`,
  `0-9` and `-`, with no `-` at either end or two in a row (the form the
  game cleans names into, so one name is one channel). A bad one is
  accepted as a WebSocket and closed at once with 4000, so the game sees
  the code.
- **Hello.** Messages before the hello (other than `ping`) are dropped, and
  so is a second hello. A hello with `v` other than 1 closes with 4001; one
  with a bad `role` or a `run` that isn't true/false closes with 4002 (a
  code the spec doesn't name).
- **Size.** Over 256 bytes (UTF-8) closes with 1009 (message too big); a
  binary frame closes with 1003. Malformed JSON, unknown `t` and invalid
  values (wrong type for `k`, a non-finite number, a picture that isn't a
  u32) are dropped.
- **Rate.** Each connection may send 60 messages per calendar second; the
  rest of that second's are dropped. Going over in 3 seconds within 10
  closes with 4008. (`{"t":"ping"}` written exactly so is answered by the
  runtime without reaching the relay's code and doesn't count.)
- **Full.** The 33rd connection on a channel is closed with 4029 (counting
  connections that haven't said hello yet).
- **Takeover.** When the active Sender leaves, its Receivers are told
  `peer none` even if a waiting Sender takes over at once (they hear the new
  one's state right after), so the old Sender's value is dropped everywhere.
  The new active Sender is told `busy:false`; it should send its current
  value then, as the relay has none from it (a waiting Sender's values are
  dropped, not kept).
- **Order.** On join a Sender hears `busy` and then `peer`; a Receiver hears
  `peer` and then, if there is one, the last value.

### State and hibernation

The Durable Object uses the WebSocket Hibernation API, so it can leave
memory between messages while its connections stay open: a channel nobody
is talking on costs nothing, and the 20-second pings (answered by the
runtime's auto-response) don't wake it. Everything it needs is in each
socket's attachment: the role, run, join order (which Sender came first),
what that connection was last told, and, on the active Sender's socket,
its last value. So the last value survives hibernation too, and it goes
away with the Sender's connection, as the spec wants. Nothing is written to
storage, nothing is logged (Workers Logs are off in `wrangler.toml`).

Only the rate counters live in plain memory; they reset if the object
hibernates, which only happens after it has been idle anyway.

## Local development

No Cloudflare account is needed for this. With Nix:

    cd relay
    nix shell nixpkgs#nodejs nixpkgs#wrangler -c wrangler dev

serves it at `ws://127.0.0.1:8787/openwgt/relay/<channel>` (the game's dev
address: `OPENWGT_RELAY=ws://127.0.0.1:8787` natively, `?relay=` in the
browser). Without Nix, `npx wrangler dev` does the same.

Tests (Node 22 or newer, for its built-in WebSocket; no npm packages):

    nix shell nixpkgs#nodejs nixpkgs#wrangler -c ./test.sh

starts `wrangler dev` on port 8790 (`PORT=` to change it) and runs
`test/relay.test.mjs` against it: peer states for both roles, run changes,
values, the last value on join and its going with the Sender, busy and
takeover, the limits and close codes, ping. It takes about 15 seconds (the
rate test waits for whole seconds).

Type check (generates `worker-configuration.d.ts` from `wrangler.toml` and
the runtime, not committed):

    nix shell nixpkgs#nodejs nixpkgs#wrangler nixpkgs#typescript -c sh -c 'wrangler types && tsc --noEmit'

## Deploying

Needs the Cloudflare account that has the gustavn.se zone.

1. `wrangler login` (opens the browser to authorize wrangler).
2. `wrangler deploy` from `relay/`. This creates the Worker
   `openwgt-relay` and the `Channel` Durable Object class (the `v1`
   migration). It answers at
   `wss://openwgt-relay.<account subdomain>.workers.dev/relay/<channel>`
   (the subdomain is shown on deploy, and under Workers & Pages).
3. Put it on gustavn.se: uncomment `routes` in `wrangler.toml` and
   `wrangler deploy` again (or add the route
   `gustavn.se/openwgt/relay/*`, zone gustavn.se, in the dashboard under
   the Worker's Settings > Domains & Routes). The site itself stays on
   Pages; only paths under `/openwgt/relay/` go to the Worker.
4. Check: in the zone's Network settings, WebSockets must be on (it is by
   default). Then, e.g. in a browser console:
   `new WebSocket("wss://gustavn.se/openwgt/relay/test").onmessage = e => console.log(e.data)`
   and send `{"t":"hello","role":"recv","run":false,"v":1}`: the answer is
   `{"t":"peer","s":"none"}`.

`wrangler tail` shows live requests (paths, which include channel names,
and close events; never message contents, the relay doesn't log them).

Changing the Durable Object class later (a rename or a new class) needs a
new `[[migrations]]` entry with a new tag; changing its code doesn't.

## Costs and limits (Workers Free plan)

As far as known in late 2026; check
<https://developers.cloudflare.com/durable-objects/platform/pricing/> and
<https://developers.cloudflare.com/workers/platform/limits/> before relying
on it. Numbers marked (?) are uncertain.

- Durable Objects are on the Free plan, SQLite-backed ones only (hence
  `new_sqlite_classes`). Over a free limit, requests fail until the daily
  reset (00:00 UTC); nothing is billed on the Free plan.
- Workers: 100,000 requests a day. Each connection is one Worker request
  (the upgrade); messages on an open WebSocket aren't Worker requests.
- Durable Objects: 100,000 requests a day (?). A connection is one; with
  the Hibernation API incoming WebSocket messages count 20 to a request (?),
  outgoing ones are free. So about 2 million messages from players a day.
  Auto-answered pings don't reach the object and, as far as I know, don't
  count (?).
- Durable Objects duration: 13,000 GB-s a day (?), charged while an object
  is in memory (128 MB each), i.e. roughly 28 hours of an awake channel a
  day. An object hibernates about 10 seconds (?) after its last event, so a
  channel only costs while values are flowing.
- CPU: 10 ms per request on Free (?); the relay uses a fraction of a
  millisecond per message.
- A Sender sends at most 30 values a second while its input changes, so a
  busy channel with one Sender is about 1.5 requests a second, around 5,400
  an hour, plus the time it is awake. Fine for a handful of players; a
  popular channel left changing all day would hit the daily limit.
- Per channel, the relay's own limits: 32 connections, 60 messages a
  second each.
