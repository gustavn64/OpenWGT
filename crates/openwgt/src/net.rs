//! The network parts' connections to the relay (`engine::net::hub`): web
//! sockets through `ewebsock`, natively a thread for each with `rustls`
//! (and the Mozilla roots built in) for `wss://`, in the browser the
//! page's `WebSocket`.

use engine::net::{Socket, SocketEvent, Transport};

/// The relay the parts connect to unless `--relay` (natively also
/// `OPENWGT_RELAY`, in the browser `?relay=`) names another.
pub const DEFAULT_RELAY: &str = "wss://gustavn.se/openwgt/relay";

/// The relay's messages are at most 256 bytes; anything much bigger is
/// not the relay's.
const MAX_FRAME: usize = 4096;

/// The relay to connect to: `--relay`'s, else (natively) `OPENWGT_RELAY`'s,
/// else [`DEFAULT_RELAY`].
pub fn relay(given: Option<&str>) -> String {
    #[cfg(not(target_arch = "wasm32"))]
    let env = std::env::var("OPENWGT_RELAY").ok();
    #[cfg(target_arch = "wasm32")]
    let env: Option<String> = None;
    given
        .map(str::to_owned)
        .or(env)
        .filter(|r| !r.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_RELAY.to_owned())
}

/// Opens web sockets.
#[derive(Default)]
pub struct WsTransport;

impl Transport for WsTransport {
    fn connect(&mut self, url: &str) -> Box<dyn Socket> {
        let options = ewebsock::Options {
            max_incoming_frame_size: MAX_FRAME,
            ..ewebsock::Options::default()
        };
        // `wss://` with rustls' own CryptoProvider, ring's (the only one
        // its features have): nothing to set up.
        let (tx, rx) = match ewebsock::connect(url, options) {
            Ok((tx, rx)) => (Some(tx), Some(rx)),
            Err(e) => {
                eprintln!("{url}: {e}");
                (None, None)
            }
        };
        let failed = rx.is_none();
        Box::new(WsSocket {
            tx,
            rx,
            open: false,
            failed,
        })
    }
}

/// One connection; dropping it (its sender) closes it.
struct WsSocket {
    tx: Option<ewebsock::WsSender>,
    /// `None` once closed.
    rx: Option<ewebsock::WsReceiver>,
    open: bool,
    /// Couldn't even start: [`SocketEvent::Closed`] at the first poll.
    failed: bool,
}

impl Socket for WsSocket {
    fn send(&mut self, text: &str) -> bool {
        match &mut self.tx {
            Some(tx) if self.open => {
                tx.send(ewebsock::WsMessage::Text(text.to_owned()));
                true
            }
            _ => false,
        }
    }

    fn poll(&mut self) -> Vec<SocketEvent> {
        if std::mem::take(&mut self.failed) {
            return vec![SocketEvent::Closed];
        }
        let mut events = Vec::new();
        while let Some(event) = self.rx.as_ref().and_then(ewebsock::WsReceiver::try_recv) {
            match event {
                ewebsock::WsEvent::Opened => {
                    self.open = true;
                    events.push(SocketEvent::Open);
                }
                ewebsock::WsEvent::Message(ewebsock::WsMessage::Text(text)) => events.push(SocketEvent::Text(text)),
                // The relay sends text alone; tungstenite answers pings
                // itself.
                ewebsock::WsEvent::Message(_) => {}
                // An error ends the connection too (natively its thread;
                // in the browser a close follows, unheard): said once.
                ewebsock::WsEvent::Error(_) | ewebsock::WsEvent::Closed => {
                    self.tx = None;
                    self.rx = None;
                    self.open = false;
                    events.push(SocketEvent::Closed);
                }
            }
        }
        events
    }
}

/// Through a real relay: `OPENWGT_RELAY`, else the local `wrangler dev`
/// (`relay/README.md`). `cargo test -p openwgt -- --ignored`.
#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use std::time::{Duration, Instant};

    use engine::assets::Assets;
    use engine::doc::Document;
    use engine::editor::Editor;
    use engine::gadget::Tag;
    use engine::net::Status;
    use engine::net::parts::{self, Receiver, Sender};

    use super::WsTransport;

    const FRAME: Duration = Duration::from_millis(16);

    fn assets() -> Assets {
        let mut a = crate::args(["--open"].iter().map(std::ffi::OsString::from));
        a.without_cd();
        crate::assets_for(&a, None, engine::lang::Lang::En).unwrap()
    }

    fn connected(doc: Document) -> Editor {
        let relay = std::env::var("OPENWGT_RELAY").unwrap_or_else(|_| "ws://127.0.0.1:8787".to_owned());
        let mut e = Editor::new(assets(), doc, 640, 433);
        e.net.set_transport(Box::new(WsTransport), &relay);
        e
    }

    /// A widget with a Constant at `value` wired into a Sender on `channel`.
    fn sending(channel: &str, value: &str) -> Editor {
        let a = assets();
        let mut c = engine::doc::new_gadget(&a, Tag::new(b"cnst"), 1, 10, 10);
        c.core.settings.set_label(0, value.as_bytes());
        c.behavior.settings_changed(&mut c.core);
        let mut s = engine::doc::new_gadget(&a, parts::SENDER, 2, 200, 10);
        s.core.settings.set_label(0, channel.as_bytes());
        let mut doc = Document::new();
        doc.gadgets = vec![c, s];
        doc.connect(0, 0, 1, 0, Vec::new(), 0).unwrap();
        connected(doc)
    }

    /// A widget with a Receiver on `channel`.
    fn receiving(channel: &str) -> Editor {
        let mut r = engine::doc::new_gadget(&assets(), parts::RECEIVER, 1, 10, 10);
        r.core.settings.set_label(0, channel.as_bytes());
        let mut doc = Document::new();
        doc.gadgets = vec![r];
        connected(doc)
    }

    fn set_constant(e: &mut Editor, value: &str) {
        let g = &mut e.doc.gadgets[0];
        g.core.settings.set_label(0, value.as_bytes());
        g.behavior.settings_changed(&mut g.core);
    }

    fn status(e: &mut Editor) -> Status {
        for g in &mut e.doc.gadgets {
            let any = g.behavior.as_any_mut();
            if let Some(s) = any.downcast_mut::<Sender>() {
                return s.status;
            }
            if let Some(r) = any.downcast_mut::<Receiver>() {
                return r.status;
            }
        }
        panic!("no network part");
    }

    fn lamp(e: &mut Editor) -> u16 {
        status(e).lamp()
    }

    /// What the Receiver gives.
    fn number(e: &Editor) -> f64 {
        e.doc.gadgets[0].core.number
    }

    /// Frames go by in every editor until `done`, for up to 15 seconds.
    fn until(eds: &mut [&mut Editor], what: &str, done: impl Fn(&mut [&mut Editor]) -> bool) {
        let end = Instant::now() + Duration::from_secs(15);
        let start = Instant::now();
        while !done(eds) {
            assert!(Instant::now() < end, "no {what} in 15 s");
            std::thread::sleep(FRAME);
            for e in eds.iter_mut() {
                e.update(16, (12, 0, 0));
            }
        }
        eprintln!("{what}: {} ms", start.elapsed().as_millis());
    }

    // The values sent are whole numbers, exact on both sides.
    #[allow(clippy::float_cmp)]
    #[test]
    #[ignore = "needs a relay: OPENWGT_RELAY, else wrangler dev on 127.0.0.1:8787"]
    fn end_to_end() {
        let channel = engine::net::random_channel();
        eprintln!("channel {channel}");
        let mut a = sending(&channel, "42");
        let mut b = receiving(&channel);

        // Both open, both stopped: amber.
        until(&mut [&mut a, &mut b], "amber on both", |e| {
            lamp(e[0]) == 3 && lamp(e[1]) == 3
        });
        assert_eq!(number(&b), 0.0);

        // GO in the Receiver's: the Sender's lamp goes green.
        b.run();
        until(&mut [&mut a, &mut b], "the Sender green", |e| lamp(e[0]) == 4);
        assert_eq!(lamp(&mut b), 3);

        // GO in the Sender's: its value arrives, the Receiver's lamp green.
        a.run();
        until(&mut [&mut a, &mut b], "42 at the Receiver", |e| {
            lamp(e[1]) == 4 && number(e[1]) == 42.0
        });

        // A change goes too.
        set_constant(&mut a, "7");
        until(&mut [&mut a, &mut b], "7 at the Receiver", |e| number(e[1]) == 7.0);

        // A second Sender waits: BUSY, and its value goes nowhere.
        let mut c = sending(&channel, "99");
        c.run();
        until(&mut [&mut a, &mut b, &mut c], "the second Sender BUSY", |e| {
            let s = status(e[2]);
            s.online && s.busy && s.lamp() == 4
        });
        assert!(!status(&mut a).busy);
        let wait = Instant::now() + Duration::from_millis(500);
        until(&mut [&mut a, &mut b, &mut c], "half a second", |_| {
            Instant::now() >= wait
        });
        assert_eq!(number(&b), 7.0);

        // The first one leaves: the second takes over and sends its value.
        drop(a);
        until(&mut [&mut b, &mut c], "the takeover", |e| {
            !status(e[1]).busy && number(e[0]) == 99.0
        });

        // STOP in the Sender's: amber at the Receiver, its value kept.
        c.stop();
        until(&mut [&mut b, &mut c], "amber at the Receiver", |e| lamp(e[0]) == 3);
        assert_eq!(number(&b), 99.0);

        // The Sender gone: red, and off and 0.
        drop(c);
        until(&mut [&mut b], "red at the Receiver", |e| {
            lamp(e[0]) == 2 && number(e[0]) == 0.0
        });

        // Another channel name: the Receiver's connection goes with it.
        b.doc.gadgets[0].core.settings.set_label(0, b"Another Name!");
        until(&mut [&mut b], "a new connection", |e| lamp(e[0]) == 1);
        until(&mut [&mut b], "the new channel", |e| lamp(e[0]) == 2);
        assert_eq!(engine::net::parts::channel(&b.doc.gadgets[0].core), "another-name");
    }
}
