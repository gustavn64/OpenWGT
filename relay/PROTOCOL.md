# OpenWGT relay protocol (v1) — shared spec for the relay and the game

FORK-ONLY, LOCAL-ONLY: nothing of this is ever pushed to GitHub (no push to any remote, no issues/PRs mentioning it).

## Transport
WebSocket to `wss://gustavn.se/openwgt/relay/<channel>` (dev: `ws://127.0.0.1:8787/openwgt/relay/<channel>`; the game takes the base URL from a constant, overridable by env `OPENWGT_RELAY` natively and `?relay=` in the browser). Text frames, one JSON object each, at most 256 bytes.

Channel name: 1–20 chars of `[a-z0-9-]`, no `-` at either end or twice in a row (the game lowercases what the player types, turns spaces and underscores into `-`, drops other characters, collapses repeated `-` and trims them from the ends; the relay rejects anything else with close code 4000). Random names are three short words from one English list of 2048, joined with hyphens, e.g. `cat-sun-oak` (the same list in every language), easy to say over the phone; typed names may have digits too. One Durable Object per channel name.

## Client → relay
- `{"t":"hello","role":"send"|"recv","run":false|true,"v":1}` — first message; `v` = protocol version (relay closes with 4001 if unsupported).
- `{"t":"run","run":true|false}` — the widget started (GO) or stopped.
- `{"t":"val","k":"b"|"n"|"p","v":<bool|number|int>}` — a Sender's value: b = on/off, n = number (finite f64), p = picture (sheet number, u32). Only from the active sender; others' are dropped. Sent when it changes; the client sends at most 30 per second (coalescing to the latest).
- `{"t":"ping"}` every 20 s (relay answers `{"t":"pong"}`); also lets the client detect a dead link.

## Relay → client
- `{"t":"peer","s":"none"|"open"|"run"}` — the other side's state, sent on join and whenever it changes. For a receiver: the active sender's state ("none" if no sender connected). For a sender: the best state among all receivers (run > open > none). A busy (waiting) sender gets the receivers' state too.
- `{"t":"busy","busy":true|false}` — to senders only: whether this sender is waiting because another sender holds the channel. First come (by connection time) is active; when it leaves, the longest-waiting sender becomes active and is told busy:false.
- `{"t":"val","k":...,"v":...}` — to receivers: the active sender's value; also the last value on join (kept in memory only while the active sender is connected; dropped when it leaves, so a new receiver gets nothing stale).
- `{"t":"pong"}`.

## Limits (relay)
- ≤ 256 bytes per message, ≤ 60 messages/s per connection (excess dropped; sustained abuse → close 4008), ≤ 32 connections per channel (more → close 4029).
- No storage beyond memory; nothing logged with content.

## Lamp (game)
grey = not connected to the relay (connecting/retrying with backoff 1,2,4..30 s); red = peer none; amber = peer open; green = peer run. A busy sender's display shows BUSY (lamp still shows the receivers' state).

## Game semantics
- Parts connect when the widget is open (loaded in the editor) and disconnect when it closes / the part is deleted / the channel is changed (reconnect to the new one). `run` follows GO/STOP.
- Receiver output: the last value received while the peer is connected; 0/off/no picture before anything arrives and after the sender leaves.
- Sender input: whatever is wired in (on/off, number or picture), sent on change while running (and once on GO).
