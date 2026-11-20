# The network parts: Sender and Receiver

Two parts the original doesn't have: a **Sender** sends what is wired into
it (on/off, a number or a picture) to the **Receivers** with the same
channel name in other widgets, on other computers too, through a relay on
the Internet. They are the port's own, in `crates/engine/src/net/`, and
the host only gives them their sockets (`crates/openwgt/src/net.rs`).

## In the game

- **In the tray** while the enhancements are on (Settings ▸
  Enhancements, on at first): a last page of the Super Parts folder, tray
  indexes 65 and 66. Widgets that have them run them either way.
- **Channel names**: 1 to 20 of `a-z`, `0-9` and `-`, no `-` at either
  end or twice in a row. What the player types is cleaned into that
  (`net::clean_channel`: capitals to small letters, spaces and
  underscores to `-`, anything else dropped). A part taken from the tray
  gets a random name at once, three words of a list of 2048 short English
  words (`tools/opennet/src/words.txt`, the same in every language), such
  as `cat-sun-oak`, from the system's randomness (`getrandom`, the
  browser's `crypto.getRandomValues`). The dialog's **Random** gives
  another. The name is the part's first text setting and shows on its
  display.
- **Sender**: sends its input once on GO, then whenever it changes, at
  most 30 values a second (the latest of those that come faster). A
  sound, or nothing wired, sends nothing.
- **Receiver**: gives what came in, of the kind it came as: off and 0
  before anything comes and after its Sender leaves; a picture the game
  doesn't have counts as none.
- **The lamp**: grey, not connected (connecting, or waiting to try again
  after 1, 2, 4 ... up to 30 seconds); red, nobody on the other side;
  amber, the other side's widget is open but stopped; green, it runs. A
  Sender's other side is its best Receiver (running beats open). A second
  Sender on a channel waits and shows **BUSY**; when the first one leaves,
  the one that has waited longest takes over and sends its current value.
- **Connections**: one per part with a channel name, opened as soon as the
  widget is open (no question asked), closed when the widget is closed,
  the part deleted or its name changed. GO and STOP are told to the
  relay. A ping goes every 20 seconds; no answer within 10 counts as a
  lost link.
- **Saving**: the original's loader would make a part of every chunk it
  doesn't know and has no class for these, so they are saved, with the
  wires that reach them, inside `back` chunks right after `fver`
  (`net::wgt`); the original sees a background before the real one, the
  port puts them back where they were. A widget link carries them the
  same way.

Texts, dialogs and pictures are in English, Swedish and French
(`tools/opennet`, built into `crates/engine/opennet*`), behind the game's
own: they fill only what the CD or OpenWDATA leaves out. OpenWDATA's help
has a topic for them (Sender and Receiver); a CD's help doesn't.

## The relay

`relay/` is the server, a Cloudflare Worker with one Durable Object per
channel; [relay/PROTOCOL.md](../relay/PROTOCOL.md) is the protocol (v1)
and [relay/README.md](../relay/README.md) how it runs, what it does where
the protocol leaves room, its limits and how to deploy it. In short: a
WebSocket to `<relay>/<channel>`, JSON text frames of at most 256 bytes;
the client says `hello` (role, running, version), `run` on GO and STOP,
and as a Sender `val`; the relay answers with `peer` (the other side's
state), `busy` (to Senders), the active Sender's `val` (to Receivers) and
`pong`.

The game connects to `wss://gustavn.se/openwgt/relay` unless told
otherwise:

- `--relay=URL` on the command line (kept when Change CD starts the game
  again),
- natively, the environment variable `OPENWGT_RELAY`,
- in the browser, `?relay=URL` in the page's address (the query is the
  command line, [web.md](web.md)).

A relay address with no path gets `/openwgt/relay` (so
`ws://127.0.0.1:8787` is the local `wrangler dev`), one with a path is
taken as it is (`wss://openwgt-relay.gustavn64.workers.dev/relay`, the
Worker's own address, also works).

Natively each connection is a thread (`ewebsock` on `tungstenite`) with
`rustls` and the `ring` provider for `wss://`, and the Mozilla root
certificates built in (`webpki-roots`), so it needs nothing from the
system; in the browser it is the page's `WebSocket`. Without a transport
(the engine's tests and tools) nothing connects and the lamps stay grey.

## Trying it

Two windows on the same channel, on one computer, against a local relay:

```sh
cd relay && nix shell nixpkgs#nodejs nixpkgs#wrangler -c wrangler dev   # ws://127.0.0.1:8787
openwgt --relay=ws://127.0.0.1:8787 &
openwgt --relay=ws://127.0.0.1:8787 &
```

or against the real relay, just `openwgt` twice. In one, put a Light
Switch and a Sender on the worksheet and wire the switch to the Sender;
in the other, a Receiver wired to a Light Bulb. Double-click the Sender,
copy its channel name into the Receiver's dialog (or type the same name
into both), press GO in both and click the switch: the bulb follows. In
the browser, open the page twice, e.g. `http://localhost:8000/?open` and
`http://localhost:8000/?open&relay=ws://127.0.0.1:8787` for a local relay.

`cargo test -p openwgt -- --ignored` runs an end-to-end test of two and
three editors through a relay (`OPENWGT_RELAY`, default the local one).

## Privacy

What goes to the relay, and from it to the other players on the same
channel:

- the channel name (in the address),
- whether each part is a Sender or a Receiver, and whether its widget is
  running,
- a Sender's values: on/off, numbers, picture numbers (a picture is sent
  as the number of one of the game's pictures, never as an image).

Nothing else of the widget or the player: no names, no files, no sounds,
no other parts. The relay keeps everything in memory only while the
connections are open (the last value of a channel goes with its Sender),
writes nothing to storage and logs no contents (Workers Logs are off).
Like any web server, the relay and Cloudflare in front of it see the
players' IP addresses and the channel names in the requests' paths;
`wrangler tail` shows those live to whoever runs the relay.

Anyone who knows (or guesses) a channel name can join it, send to its
Receivers once its Sender leaves, and see its values. The random names,
one of 2048³ (about 8.6 billion), make that unlikely by chance, not
impossible on purpose: the parts are for play, not for anything secret,
and the help says so.
