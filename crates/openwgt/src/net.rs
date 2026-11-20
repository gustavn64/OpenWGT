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

