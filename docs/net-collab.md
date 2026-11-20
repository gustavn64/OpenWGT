# Building together: a proposal

FORK-ONLY, LOCAL-ONLY, like the rest of the network work: nothing of this
goes to GitHub until decided otherwise.

A proposal for several players building the same widget at once over the
relay the Sender and Receiver use (docs/net-parts.md, relay/PROTOCOL.md).
Nothing here is built yet; it's the design to decide on first. Open
questions are marked **Q**.

## What it should feel like

- One player picks **File ▸ Build Together…** and gets a room name, three
  words like a channel (`cat-sun-oak`), to read out over the phone; the
  others pick the same item and type it. No accounts, no lobby.
- Everyone sees the same worksheet. Each player has a colour; the others'
  pointers show as small coloured arrows with their initial, and a part
  someone is dragging or has a dialog open on gets an outline in their
  colour (and can't be grabbed by another meanwhile).
- Parts, wires, settings, deleting, moving, the background: all shared.
  Edit ▸ Undo undoes your own last change.
- GO and STOP: see "Running" below.
- Anyone can save the widget to their own computer at any time; leaving
  the room keeps your copy as an ordinary unsaved widget.

## The model: the relay orders, the clients apply

The widget is a `Document`; every change to it is an **operation**. A
client never changes the shared document directly: it sends the operation
to the room, and the room's Durable Object stamps it with the next
sequence number and sends it to everyone, the sender included. Every
client applies the operations in sequence order, so all end with the same
document, without merging rules. (A central sequencer is the simplest
thing that is correct; the relay already is one Durable Object per name.)

So that one's own changes don't feel laggy, a drag is drawn locally while
it lasts and only the drop is sent; a dialog's OK sends the settings. The
echo comes back in tens of milliseconds; until then the change shows
"pending" (as drawn), and if the room refuses it (the part was deleted
meanwhile) it's dropped with a quiet sound.

### Operations

All in terms of part ids (`Core::id`, already stable and saved), never
list indexes (`raise_gadget` reorders the list on every click):

| Op | Fields |
|---|---|
| `add` | id, tag, x, y, settings block (the saved bytes) |
| `move` | id, x, y |
| `settings` | id, settings block |
| `delete` | id (its wires go with it) |
| `wire` | from (id, knot), to (id, knot), bend points |
| `unwire` | from (id, knot), to (id, knot) |
| `raise` | id (drawing order; a click raises a part in the original) |
| `background` | sheet |
| `lock` / `unlock` | id (dragging, dialog open), with the player |
| `pointer` | x, y (not sequenced, not stored; at most 10 a second) |

Each op is validated by every client the same way before applying (the
part exists, the knot is free, the tag is a real class, a puzzle's tray
limits...); an invalid op is skipped by all, so they stay equal.

**Ids**: each client makes ids from its own range (the room gives each
joiner a number n; its ids are `n << 20 | k`), so two clients adding parts
at the same time never collide. A widget saved from a room keeps them;
the original game only needs them unique.

### Joining

A joiner needs the document as it is now. The room keeps, in memory
only, the latest snapshot (the `.WGT` bytes, sent by the first player
when the room is made, and refreshed by any client every 200 operations)
plus the operations since; it sends both to the joiner. The `.WGT` is
tens of kilobytes, more than relay messages carry (256 bytes): the room
sends it in chunks, and its limits for rooms are separate from channels'
(e.g. 64 KB per message, 8 players, 30 operations a second per player).

**Q**: when the last player leaves, the room forgets everything (as
channels do) — or keeps the snapshot a day so the group can come back?
The proposal: forget; whoever wants it keeps a saved copy.

### Undo

The original's Undo is one level, a toggle between a snapshot and now.
Shared: each client keeps the inverse of its own last operation(s) (a
`delete` inverts to an `add` with the saved bytes and the wires; a `move`
to the old place) and Undo sends the inverse as a new operation. If a
later operation by someone else makes the inverse invalid (the part was
deleted), Undo does nothing and says so. **Q**: one level as the
original, or several?

## Running

The hard part: a running widget changes every tick and reads the mouse,
the keys, the clock, the microphone, random numbers and its Receivers.

1. **v1: editing shared, running each on their own.** GO runs the
   widget on your computer only; the others may run it too, or keep
   editing. While you run, the others' edits wait and apply at your STOP
   (the original stops before editing anyway). Simple, and enough to
   build together.
2. **v2: running together, in lockstep.** Everyone's simulation runs
   the same ticks from the same start, fed the same inputs: each tick's
   inputs (clicks on parts, keys, the mouse over the worksheet, which
   player) go through the room like operations, and every client applies
   tick `t` only once it has all inputs for `t` (a fixed delay of a few
   ticks hides the network's). The random seed is shared at GO, the
   clock is the tick count, the microphone and Receivers are inputs of
   the player they belong to. This is the same input log the debugging
   tools' rewind needs (scripting plan §6), so the two share their code;
   the rewind's determinism test is what makes lockstep trustworthy. A
   checksum of the parts' state every second detects a divergence (then
   the leader's state is sent to all).

**Q**: whose mouse does a running widget see in v2? Proposal: each
player's own pointer drives the parts they click; parts that follow the
mouse (Mouse Distance, the Win/Lose cheat) follow the player who last
pressed GO.

## Puzzles

A puzzle has a goal and tray limits. Building one together works the same
(the limits are checked per operation, for the whole room); "solved"
counts for everyone in the room. **Q**: allow it, or free building only?
Proposal: allow; it's a nice thing to do together.

## Privacy and safety

As channels: no accounts, nothing stored but in memory while the room
lives, nothing logged with content. The room name keeps strangers out (8.6
billion names); a room is capped at 8 players; a player can be told to
leave by whoever made the room. Text in labels travels as part of the
widget, so the same care as with a widget link applies. Children: the
room's three words are the only thing shared.

## Phases

1. Operations in the engine: the editor's changes expressed as ops
   (`editor/collab.rs`), applied through one function, with a test that
   replaying the ops of a session rebuilds the document byte for byte.
   Useful alone: it's also a better undo history and a macro recorder for
   scripts.
2. The room in the relay: sequencing, snapshot + ops for joiners, ids,
   locks, pointers, limits; a protocol document beside PROTOCOL.md; tests
   in `relay/test`.
3. The game: Build Together dialog, applying remote ops, pending local
   ones, pointers and locks drawn, undo of own ops; the end-to-end test of
   two and three editors (as the Sender's).
4. Running v1.
5. Running v2 (lockstep), after the rewind work.
