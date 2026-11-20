//! The hub between the network parts and the relay: one connection per
//! part with a channel name, opened while the widget is open, through a
//! [`Transport`] the host gives (the engine itself does no input or
//! output, as with the microphone, `crate::sim::Mic`). Without one nothing
//! connects and the lamps stay grey.
//!
//! [`Hub::sync`], once a frame on the editor's clock, matches the parts
//! with their connections, reads what the relay said, tells it GO and STOP
//! and the Senders' values, and gives the parts their state and the
//! Receivers what came in. The timers (retries, pings) run on the same
//! clock, so the whole is deterministic for a given run of events.

use std::collections::{BTreeMap, BTreeSet};

use crate::doc::Document;

use super::parts::{self, Receiver, Sender};
use super::proto::{self, ClientMsg, RelayMsg, Role};
use super::{NetValue, Peer, Status};

/// Opens connections to the relay: the host's web sockets.
pub trait Transport {
    /// Starts connecting to `url`. The socket says [`SocketEvent::Open`]
    /// once connected (or [`SocketEvent::Closed`] if it can't).
    fn connect(&mut self, url: &str) -> Box<dyn Socket>;
}

/// One connection. Dropping it closes it.
pub trait Socket {
    /// Sends a text frame; `false` if it can't now (not open, or gone).
    fn send(&mut self, text: &str) -> bool;

    /// What happened since the last call, in order. Nothing comes after
    /// [`SocketEvent::Closed`].
    fn poll(&mut self) -> Vec<SocketEvent>;
}

/// What a [`Socket`] reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SocketEvent {
    /// Connected: messages can go.
    Open,
    /// A text frame from the relay.
    Text(String),
    /// Closed, by either side, or never connected.
    Closed,
}

/// The path the relay serves channels under, added to a relay address
/// that has none (`ws://127.0.0.1:8787`).
pub const RELAY_PATH: &str = "/openwgt/relay";
/// Every 20 s a ping; no pong 10 s after it, and the link counts as dead.
pub const PING_MS: u64 = 20_000;
pub const PONG_MS: u64 = 10_000;
/// The waits before connecting again: 1, 2, 4, ... seconds, up to 30.
pub const RETRY_FIRST_MS: u64 = 1_000;
pub const RETRY_MAX_MS: u64 = 30_000;
/// At most 30 values a second from a Sender: one each 33⅓ ms (counted in
/// thirds of a millisecond), with a few milliseconds' slack for frames
/// that don't fall on the 33⅓.
const SEND_EVERY_3: u64 = 100;
const SEND_SLACK_3: u64 = 30;

/// The address of `channel` on the relay at `base`
/// (`wss://gustavn.se/openwgt/relay` gives
/// `wss://gustavn.se/openwgt/relay/cat-sun-oak`); a base with no path
/// gets [`RELAY_PATH`].
#[must_use]
pub fn url(base: &str, channel: &str) -> String {
    let base = base.trim_end_matches('/');
    let host_only = base.split_once("://").is_some_and(|(_, rest)| !rest.contains('/'));
    let path = if host_only { RELAY_PATH } else { "" };
    format!("{base}{path}/{channel}")
}

/// The wait before try `n` (from 0) to connect again.
fn retry_ms(failures: u32) -> u64 {
    RETRY_FIRST_MS
        .checked_shl(failures.min(16))
        .map_or(RETRY_MAX_MS, |ms| ms.min(RETRY_MAX_MS))
}

/// One part's connection.
struct Endpoint {
    role: Role,
    channel: String,
    socket: Option<Box<dyn Socket>>,
    /// Open, and the hello said.
    open: bool,
    /// The GO/STOP the relay was last told.
    told_run: bool,
    peer: Peer,
    /// A Sender's turn, as the relay last said: waiting (`true`), holding
    /// the channel (`false`), not said yet (`None`).
    busy: Option<bool>,
    /// A Sender's value waiting for its turn (the latest of those that
    /// came too fast).
    pending: Option<NetValue>,
    /// When the next value may go (thirds of a millisecond).
    send_at3: u64,
    /// What a Receiver gives: the last value while its Sender is there.
    value: Option<NetValue>,
    /// Tries in a row that came to nothing, and when to try again.
    failures: u32,
    retry_at: u64,
    /// When to ping next, and the pong awaited.
    ping_at: u64,
    pong_by: Option<u64>,
}

impl Endpoint {
    fn new(role: Role, channel: String) -> Endpoint {
        Endpoint {
            role,
            channel,
            socket: None,
            open: false,
            told_run: false,
            peer: Peer::None,
            busy: None,
            pending: None,
            send_at3: 0,
            value: None,
            failures: 0,
            retry_at: 0,
            ping_at: 0,
            pong_by: None,
        }
    }

    fn status(&self) -> Status {
        Status {
            online: self.open,
            peer: if self.open { self.peer } else { Peer::None },
            busy: self.open && self.busy == Some(true),
        }
    }

    fn send(&mut self, msg: ClientMsg) {
        if let (Some(socket), Some(text)) = (&mut self.socket, msg.encode()) {
            socket.send(&text);
        }
    }

    /// Lets the connection go, to try again after the wait.
    fn drop_link(&mut self, now: u64) {
        self.socket = None;
        self.open = false;
        self.peer = Peer::None;
        self.busy = None;
        self.pong_by = None;
        // The Sender is gone as far as this part knows.
        self.value = None;
        self.retry_at = now + retry_ms(self.failures);
        self.failures = self.failures.saturating_add(1);
    }

    /// A Sender's value sent now, and the next one's turn put back.
    fn send_value(&mut self, v: NetValue, now: u64) {
        self.send(ClientMsg::Val(v));
        self.send_at3 = self.send_at3.max(now * 3) + SEND_EVERY_3;
    }

    /// Connects when it's time, reads what came in and keeps the link
    /// alive. `last` is a Sender's current input.
    fn drive(
        &mut self,
        transport: Option<&mut (dyn Transport + 'static)>,
        base: &str,
        running: bool,
        last: Option<NetValue>,
        now: u64,
    ) {
        if self.socket.is_none() {
            let Some(t) = transport else { return };
            if now < self.retry_at {
                return;
            }
            self.socket = Some(t.connect(&url(base, &self.channel)));
        }
        let events = self.socket.as_mut().map(|s| s.poll()).unwrap_or_default();
        for event in events {
            match event {
                SocketEvent::Open => {
                    self.open = true;
                    self.told_run = running;
                    self.send(ClientMsg::Hello {
                        role: self.role,
                        run: running,
                    });
                    self.ping_at = now + PING_MS;
                    self.pong_by = None;
                }
                SocketEvent::Text(text) => {
                    if let Some(msg) = proto::parse(&text) {
                        // The relay answers: the link works.
                        self.failures = 0;
                        self.take(msg, last, now);
                    }
                }
                SocketEvent::Closed => {
                    self.drop_link(now);
                    return;
                }
            }
        }
        if !self.open {
            return;
        }
        if self.pong_by.is_some_and(|by| now >= by) {
            self.drop_link(now);
            return;
        }
        if now >= self.ping_at {
            self.send(ClientMsg::Ping);
            self.ping_at = now + PING_MS;
            self.pong_by.get_or_insert(now + PONG_MS);
        }
    }

    fn take(&mut self, msg: RelayMsg, last: Option<NetValue>, now: u64) {
        match msg {
            RelayMsg::Peer(p) => {
                self.peer = p;
                if p == Peer::None && self.role == Role::Recv {
                    self.value = None;
                }
            }
            RelayMsg::Busy(b) if self.role == Role::Send => {
                self.busy = Some(b);
                // Waiting, nothing goes; the channel got, the current
                // value goes at once, as the relay has none from us.
                self.pending = None;
                if !b && let Some(v) = last {
                    self.send_value(v, now);
                }
            }
            RelayMsg::Val(v) if self.role == Role::Recv => self.value = Some(v),
            RelayMsg::Pong => self.pong_by = None,
            RelayMsg::Busy(_) | RelayMsg::Val(_) => {}
        }
    }

    /// GO and STOP, and a Sender's value when its turn comes.
    fn speak(&mut self, running: bool, now: u64) {
        if !self.open {
            return;
        }
        if self.told_run != running {
            self.told_run = running;
            self.send(ClientMsg::Run(running));
        }
        match self.busy {
            Some(false) => {}
            // Waiting: what it has goes when it gets the channel.
            Some(true) => self.pending = None,
            // Not told yet: the first word is the turn.
            None => return,
        }
        if now * 3 + SEND_SLACK_3 >= self.send_at3
            && let Some(v) = self.pending.take()
        {
            self.send_value(v, now);
        }
    }
}

/// The network parts' connections.
#[derive(Default)]
pub struct Hub {
    transport: Option<Box<dyn Transport>>,
    base: String,
    /// By part id.
    eps: BTreeMap<u32, Endpoint>,
}

impl Hub {
    /// Connects through `transport` to the relay at `base`
    /// (`wss://gustavn.se/openwgt/relay`), from the next [`Hub::sync`]; the
    /// connections there were are closed.
    pub fn set_transport(&mut self, transport: Box<dyn Transport>, base: &str) {
        self.transport = Some(transport);
        base.clone_into(&mut self.base);
        self.eps.clear();
    }

    /// Whether there is a transport to connect with.
    #[must_use]
    pub fn connects(&self) -> bool {
        self.transport.is_some()
    }

    /// Once a frame, `now` on the editor's clock in milliseconds: the
    /// parts of `doc` matched with their connections (a new channel, or a
    /// part gone, closes one), the relay heard and told GO/STOP
    /// (`running`) and the Senders' values, and the parts given their
    /// state and the Receivers what came in.
    pub fn sync(&mut self, doc: &mut Document, running: bool, now: u64) {
        let mut seen = BTreeSet::new();
        for g in &mut doc.gadgets {
            let Some(role) = parts::role(g.core.tag) else { continue };
            let channel = parts::channel(&g.core);
            let id = g.core.id;
            let ep = if channel.is_empty() {
                None
            } else {
                seen.insert(id);
                let ep = self
                    .eps
                    .entry(id)
                    .or_insert_with(|| Endpoint::new(role, channel.clone()));
                if ep.role != role || ep.channel != channel {
                    *ep = Endpoint::new(role, channel);
                }
                Some(ep)
            };
            let any = g.behavior.as_any_mut();
            if let Some(s) = any.downcast_mut::<Sender>() {
                let out = s.outbox.take();
                if let Some(ep) = ep {
                    if out.is_some() {
                        ep.pending = out;
                    }
                    ep.drive(self.transport.as_deref_mut(), &self.base, running, s.last, now);
                    ep.speak(running, now);
                    s.status = ep.status();
                } else {
                    s.status = Status::default();
                }
            } else if let Some(r) = any.downcast_mut::<Receiver>() {
                let (status, value) = match ep {
                    Some(ep) => {
                        ep.drive(self.transport.as_deref_mut(), &self.base, running, None, now);
                        ep.speak(running, now);
                        (ep.status(), ep.value)
                    }
                    None => (Status::default(), None),
                };
                r.status = status;
                if r.inbox != value {
                    r.inbox = value;
                    r.inbox_gen = r.inbox_gen.wrapping_add(1);
                }
            }
        }
        self.eps.retain(|id, _| seen.contains(id));
    }
}

#[cfg(test)]
mod tests {
    use super::super::fake::Fake;
    use super::*;
    use crate::assets::Assets;
    use crate::gadget::{Gadget, Tag};

    const BASE: &str = "wss://relay.test/openwgt/relay";
    const NOTHING: [&str; 0] = [];
    const HELLO_SEND: &str = r#"{"t":"hello","role":"send","run":false,"v":1}"#;
    const HELLO_RECV: &str = r#"{"t":"hello","role":"recv","run":false,"v":1}"#;

    fn part(tag: Tag, id: u32, channel: &str) -> Gadget {
        let mut g = crate::doc::new_gadget(&Assets::default(), tag, id, 0, 0);
        g.core.settings.set_label(0, channel.as_bytes());
        g
    }

    /// A widget of `parts`, and a hub on a fake relay.
    fn setup(parts: Vec<Gadget>) -> (Document, Hub, Fake) {
        let mut doc = Document::new();
        doc.gadgets = parts;
        let fake = Fake::default();
        let mut hub = Hub::default();
        hub.set_transport(Box::new(fake.clone()), BASE);
        (doc, hub, fake)
    }

    fn sender(doc: &mut Document, g: usize) -> &mut Sender {
        doc.gadgets[g].behavior.as_any_mut().downcast_mut().unwrap()
    }

    fn receiver(doc: &mut Document, g: usize) -> &mut Receiver {
        doc.gadgets[g].behavior.as_any_mut().downcast_mut().unwrap()
    }

    fn status(doc: &mut Document, g: usize) -> Status {
        let any = doc.gadgets[g].behavior.as_any_mut();
        if let Some(s) = any.downcast_mut::<Sender>() {
            s.status
        } else {
            any.downcast_mut::<Receiver>().unwrap().status
        }
    }

    /// A Sender connected on line 0 and holding the channel at 16 ms.
    fn online_sender(running: bool) -> (Document, Hub, Fake) {
        let (mut doc, mut hub, fake) = setup(vec![part(parts::SENDER, 1, "cat-sun-oak")]);
        hub.sync(&mut doc, running, 0);
        fake.event(0, SocketEvent::Open);
        fake.say(0, r#"{"t":"busy","busy":false}"#);
        fake.say(0, r#"{"t":"peer","s":"open"}"#);
        hub.sync(&mut doc, running, 16);
        fake.sent(0);
        (doc, hub, fake)
    }

    #[test]
    fn urls() {
        assert_eq!(url(BASE, "cat-sun-oak"), "wss://relay.test/openwgt/relay/cat-sun-oak");
        assert_eq!(
            url("wss://relay.test/openwgt/relay/", "a"),
            "wss://relay.test/openwgt/relay/a"
        );
        assert_eq!(url("ws://127.0.0.1:8787", "a"), "ws://127.0.0.1:8787/openwgt/relay/a");
        assert_eq!(url("ws://127.0.0.1:8787/", "a"), "ws://127.0.0.1:8787/openwgt/relay/a");
        assert_eq!(url("wss://x.workers.dev/relay", "a"), "wss://x.workers.dev/relay/a");
    }

    #[test]
    fn without_a_transport_nothing_connects() {
        let mut doc = Document::new();
        doc.gadgets = vec![
            part(parts::SENDER, 1, "cat-sun-oak"),
            part(parts::RECEIVER, 2, "cat-sun-oak"),
        ];
        let mut hub = Hub::default();
        assert!(!hub.connects());
        sender(&mut doc, 0).outbox = Some(NetValue::Number(1.0));
        for now in [0, 1_000, 60_000] {
            hub.sync(&mut doc, true, now);
        }
        assert_eq!(status(&mut doc, 0), Status::default());
        assert_eq!(status(&mut doc, 1).lamp(), 1);
        assert_eq!(receiver(&mut doc, 1).inbox, None);
    }

    #[test]
    fn hello_on_open_then_the_lamp_follows_the_peer() {
        let (mut doc, mut hub, fake) = setup(vec![part(parts::RECEIVER, 7, "Cat Sun Oak")]);
        hub.sync(&mut doc, false, 0);
        assert_eq!(fake.count(), 1);
        assert_eq!(fake.url(0), "wss://relay.test/openwgt/relay/cat-sun-oak");
        assert_eq!(fake.sent(0), NOTHING, "nothing before it's open");
        assert_eq!(status(&mut doc, 0).lamp(), 1);
        fake.event(0, SocketEvent::Open);
        hub.sync(&mut doc, false, 16);
        assert_eq!(fake.sent(0), [HELLO_RECV]);
        assert_eq!(status(&mut doc, 0).lamp(), 2, "online, nobody there");
        fake.say(0, r#"{"t":"peer","s":"open"}"#);
        hub.sync(&mut doc, false, 32);
        assert_eq!(status(&mut doc, 0).lamp(), 3);
        fake.say(0, r#"{"t":"peer","s":"run"}"#);
        hub.sync(&mut doc, false, 48);
        assert_eq!(status(&mut doc, 0).lamp(), 4);
        // Rubbish is ignored.
        fake.say(0, "{nope");
        fake.say(0, r#"{"t":"what"}"#);
        hub.sync(&mut doc, false, 64);
        assert_eq!(status(&mut doc, 0).lamp(), 4);
        // One connection, kept.
        hub.sync(&mut doc, false, 80);
        assert_eq!(fake.count(), 1);
        assert!(!fake.dropped(0));
    }

    #[test]
    fn a_waiting_sender_shows_busy_and_sends_when_it_gets_the_channel() {
        let (mut doc, mut hub, fake) = setup(vec![part(parts::SENDER, 1, "cat-sun-oak")]);
        hub.sync(&mut doc, true, 0);
        fake.event(0, SocketEvent::Open);
        fake.say(0, r#"{"t":"busy","busy":true}"#);
        fake.say(0, r#"{"t":"peer","s":"run"}"#);
        sender(&mut doc, 0).last = Some(NetValue::Number(5.0));
        sender(&mut doc, 0).outbox = Some(NetValue::Number(5.0));
        hub.sync(&mut doc, true, 16);
        assert_eq!(
            fake.sent(0),
            [r#"{"t":"hello","role":"send","run":true,"v":1}"#],
            "nothing while waiting"
        );
        let s = status(&mut doc, 0);
        assert!(s.busy);
        assert_eq!(s.lamp(), 4, "the lamp still shows the receivers");
        sender(&mut doc, 0).last = Some(NetValue::Number(6.0));
        sender(&mut doc, 0).outbox = Some(NetValue::Number(6.0));
        hub.sync(&mut doc, true, 32);
        assert_eq!(fake.sent(0), NOTHING);
        // The channel is ours: the current value at once.
        fake.say(0, r#"{"t":"busy","busy":false}"#);
        hub.sync(&mut doc, true, 48);
        assert_eq!(fake.sent(0), [r#"{"t":"val","k":"n","v":6.0}"#]);
        assert!(!status(&mut doc, 0).busy);
        // Nothing to send: nothing sent.
        let (mut doc, mut hub, fake) = setup(vec![part(parts::SENDER, 1, "cat-sun-oak")]);
        hub.sync(&mut doc, false, 0);
        fake.event(0, SocketEvent::Open);
        fake.say(0, r#"{"t":"busy","busy":false}"#);
        hub.sync(&mut doc, false, 16);
        assert_eq!(fake.sent(0), [HELLO_SEND]);
    }

    #[test]
    fn values_reach_the_receiver_and_go_with_the_sender() {
        let (mut doc, mut hub, fake) = setup(vec![part(parts::RECEIVER, 2, "cat-sun-oak")]);
        hub.sync(&mut doc, false, 0);
        fake.event(0, SocketEvent::Open);
        fake.say(0, r#"{"t":"peer","s":"run"}"#);
        fake.say(0, r#"{"t":"val","k":"n","v":42}"#);
        hub.sync(&mut doc, false, 16);
        let r = receiver(&mut doc, 0);
        assert_eq!(r.inbox, Some(NetValue::Number(42.0)));
        let generation = r.inbox_gen;
        // Unchanged: the same generation.
        hub.sync(&mut doc, false, 32);
        assert_eq!(receiver(&mut doc, 0).inbox_gen, generation);
        fake.say(0, r#"{"t":"val","k":"b","v":true}"#);
        hub.sync(&mut doc, false, 48);
        let r = receiver(&mut doc, 0);
        assert_eq!(r.inbox, Some(NetValue::Bool(true)));
        assert_ne!(r.inbox_gen, generation);
        // Several in one frame: the last.
        fake.say(0, r#"{"t":"val","k":"p","v":1050}"#);
        fake.say(0, r#"{"t":"val","k":"n","v":-1.5}"#);
        hub.sync(&mut doc, false, 64);
        assert_eq!(receiver(&mut doc, 0).inbox, Some(NetValue::Number(-1.5)));
        // The Sender left: nothing.
        fake.say(0, r#"{"t":"peer","s":"none"}"#);
        hub.sync(&mut doc, false, 80);
        assert_eq!(receiver(&mut doc, 0).inbox, None);
        assert_eq!(status(&mut doc, 0).lamp(), 2);
        // The part made again (an undo, a load): given it all again.
        fake.say(0, r#"{"t":"peer","s":"run"}"#);
        fake.say(0, r#"{"t":"val","k":"n","v":3}"#);
        hub.sync(&mut doc, false, 96);
        doc.gadgets[0] = part(parts::RECEIVER, 2, "cat-sun-oak");
        assert_eq!(status(&mut doc, 0).lamp(), 1);
        hub.sync(&mut doc, false, 112);
        let r = receiver(&mut doc, 0);
        assert_eq!((r.inbox, r.status.lamp()), (Some(NetValue::Number(3.0)), 4));
        assert_ne!(r.inbox_gen, 0);
        assert_eq!(fake.count(), 1, "the same connection");
    }

    #[test]
    fn the_receiver_gives_what_came_in_after_a_tick() {
        let assets = Assets::load(None, Some(&crate::opendata::builtin())).unwrap();
        let mut doc = Document::new();
        let mut r = crate::doc::new_gadget(&assets, parts::RECEIVER, 1, 0, 0);
        r.core.settings.set_label(0, b"cat-sun-oak");
        doc.gadgets.push(r);
        let fake = Fake::default();
        let mut hub = Hub::default();
        hub.set_transport(Box::new(fake.clone()), BASE);
        let mut env = crate::sim::Env::default();
        hub.sync(&mut doc, true, 0);
        doc.start(&assets, &mut env);
        assert_eq!(doc.gadgets[0].core.number, 0.0);
        fake.event(0, SocketEvent::Open);
        fake.say(0, r#"{"t":"peer","s":"run"}"#);
        fake.say(0, r#"{"t":"val","k":"n","v":42}"#);
        hub.sync(&mut doc, true, 16);
        assert_eq!(doc.gadgets[0].core.number, 0.0, "not before the tick");
        doc.tick(&assets, &mut env);
        assert_eq!(doc.gadgets[0].core.number, 42.0);
        fake.say(0, r#"{"t":"peer","s":"none"}"#);
        hub.sync(&mut doc, true, 32);
        doc.tick(&assets, &mut env);
        assert_eq!(doc.gadgets[0].core.number, 0.0);
    }

    #[test]
    fn go_and_stop_are_told() {
        let (mut doc, mut hub, fake) = online_sender(false);
        hub.sync(&mut doc, true, 32);
        assert_eq!(fake.sent(0), [r#"{"t":"run","run":true}"#]);
        hub.sync(&mut doc, true, 48);
        assert_eq!(fake.sent(0), NOTHING, "once");
        hub.sync(&mut doc, false, 64);
        assert_eq!(fake.sent(0), [r#"{"t":"run","run":false}"#]);
        // Changed before the link is up: the hello says it.
        let (mut doc, mut hub, fake) = setup(vec![part(parts::RECEIVER, 2, "cat-sun-oak")]);
        hub.sync(&mut doc, false, 0);
        hub.sync(&mut doc, true, 16);
        fake.event(0, SocketEvent::Open);
        hub.sync(&mut doc, true, 32);
        assert_eq!(fake.sent(0), [r#"{"t":"hello","role":"recv","run":true,"v":1}"#]);
    }

    #[test]
    fn a_sender_sends_once_on_go_then_at_most_30_a_second() {
        let (mut doc, mut hub, fake) = online_sender(false);
        // GO: the run, then the value the start put in the outbox.
        sender(&mut doc, 0).outbox = Some(NetValue::Number(1.0));
        hub.sync(&mut doc, true, 1_000);
        assert_eq!(
            fake.sent(0),
            [r#"{"t":"run","run":true}"#, r#"{"t":"val","k":"n","v":1.0}"#]
        );
        // A new value every frame for two seconds: the latest goes when
        // its turn comes, about 30 a second, and the last one in the end.
        let mut vals = Vec::new();
        let mut now = 1_000;
        for i in 0..120 {
            now += if i % 3 == 0 { 17 } else { 16 };
            sender(&mut doc, 0).outbox = Some(NetValue::Number(f64::from(i)));
            hub.sync(&mut doc, true, now);
            vals.extend(fake.sent(0).into_iter().map(|m| (now, m)));
        }
        for _ in 0..3 {
            now += 16;
            hub.sync(&mut doc, true, now);
            vals.extend(fake.sent(0).into_iter().map(|m| (now, m)));
        }
        assert!(vals.iter().all(|(_, m)| m.starts_with(r#"{"t":"val""#)));
        assert_eq!(vals.last().unwrap().1, r#"{"t":"val","k":"n","v":119.0}"#);
        // In any second at most 30, give or take the one on GO.
        let times: Vec<u64> = std::iter::once(1_000).chain(vals.iter().map(|(t, _)| *t)).collect();
        for (i, &t) in times.iter().enumerate() {
            let n = times[i..].iter().take_while(|&&u| u < t + 1_000).count();
            assert!(n <= 31, "{n} from {t}");
        }
        assert!((50..=62).contains(&vals.len()), "{}", vals.len());
        // Unchanged: nothing.
        now += 1_000;
        hub.sync(&mut doc, true, now);
        assert_eq!(fake.sent(0), NOTHING);
        // After a pause, one goes at once.
        sender(&mut doc, 0).outbox = Some(NetValue::Bool(true));
        hub.sync(&mut doc, true, now + 1);
        assert_eq!(fake.sent(0), [r#"{"t":"val","k":"b","v":true}"#]);
    }

    #[test]
    fn a_lost_link_comes_back_after_1_2_4_up_to_30_seconds() {
        let (mut doc, mut hub, fake) = setup(vec![part(parts::RECEIVER, 2, "cat-sun-oak")]);
        hub.sync(&mut doc, false, 0);
        assert_eq!(fake.count(), 1);
        // Never connects: tries again after 1, 2, 4, ... 30, 30 s.
        let mut now = 0;
        let mut waits = Vec::new();
        for n in 0..8 {
            fake.event(n, SocketEvent::Closed);
            hub.sync(&mut doc, false, now);
            assert!(fake.dropped(n));
            let closed = now;
            while fake.count() == n + 1 {
                now += 100;
                hub.sync(&mut doc, false, now);
                assert_eq!(status(&mut doc, 0).lamp(), 1);
            }
            waits.push(now - closed);
        }
        assert_eq!(waits, [1_000, 2_000, 4_000, 8_000, 16_000, 30_000, 30_000, 30_000]);
        // Once the relay answers, from 1 s again; its value goes with it.
        fake.event(8, SocketEvent::Open);
        fake.say(8, r#"{"t":"peer","s":"run"}"#);
        fake.say(8, r#"{"t":"val","k":"n","v":9}"#);
        hub.sync(&mut doc, false, now);
        assert_eq!(fake.sent(8), [HELLO_RECV]);
        assert_eq!(receiver(&mut doc, 0).inbox, Some(NetValue::Number(9.0)));
        fake.event(8, SocketEvent::Closed);
        hub.sync(&mut doc, false, now + 16);
        assert_eq!(receiver(&mut doc, 0).inbox, None);
        assert_eq!(status(&mut doc, 0).lamp(), 1);
        hub.sync(&mut doc, false, now + 16 + 999);
        assert_eq!(fake.count(), 9);
        hub.sync(&mut doc, false, now + 16 + 1_000);
        assert_eq!(fake.count(), 10);
    }

    #[test]
    fn pings_every_20_seconds_and_a_missing_pong_reconnects() {
        let (mut doc, mut hub, fake) = online_sender(false);
        hub.sync(&mut doc, false, 20_015);
        assert_eq!(fake.sent(0), NOTHING);
        hub.sync(&mut doc, false, 20_016);
        assert_eq!(fake.sent(0), [r#"{"t":"ping"}"#]);
        fake.say(0, r#"{"t":"pong"}"#);
        hub.sync(&mut doc, false, 30_100);
        hub.sync(&mut doc, false, 40_016);
        assert_eq!(fake.sent(0), [r#"{"t":"ping"}"#]);
        assert!(!fake.dropped(0));
        // No pong this time: dead after 10 s, tried again 1 s later.
        hub.sync(&mut doc, false, 50_015);
        assert!(!fake.dropped(0));
        hub.sync(&mut doc, false, 50_016);
        assert!(fake.dropped(0));
        assert_eq!(status(&mut doc, 0).lamp(), 1);
        hub.sync(&mut doc, false, 51_016);
        assert_eq!(fake.count(), 2);
    }

    #[test]
    fn a_new_channel_or_a_part_gone_closes_the_link() {
        let (mut doc, mut hub, fake) = setup(vec![
            part(parts::SENDER, 1, "cat-sun-oak"),
            part(parts::RECEIVER, 2, "cat-sun-oak"),
            part(Tag::new(b"cnst"), 3, "not-a-channel"),
        ]);
        hub.sync(&mut doc, false, 0);
        assert_eq!(fake.count(), 2, "one for each network part");
        doc.gadgets[0].core.settings.set_label(0, b"dog-moon-elm");
        hub.sync(&mut doc, false, 16);
        assert!(fake.dropped(0) && !fake.dropped(1));
        assert_eq!(fake.count(), 3);
        assert_eq!(fake.url(2), "wss://relay.test/openwgt/relay/dog-moon-elm");
        // No channel: no link.
        doc.gadgets[0].core.settings.set_label(0, b"");
        hub.sync(&mut doc, false, 32);
        assert!(fake.dropped(2));
        assert_eq!(fake.count(), 3);
        assert_eq!(status(&mut doc, 0), Status::default());
        // Deleted.
        doc.gadgets.remove(1);
        hub.sync(&mut doc, false, 48);
        assert!(fake.dropped(1));
        // Another transport: everything again through it.
        let other = Fake::default();
        doc.gadgets[0].core.settings.set_label(0, b"cat-sun-oak");
        hub.set_transport(Box::new(other.clone()), BASE);
        hub.sync(&mut doc, false, 64);
        assert_eq!((fake.count(), other.count()), (3, 1));
    }
}
