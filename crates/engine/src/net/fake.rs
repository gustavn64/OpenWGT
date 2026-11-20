//! A relay in memory for the tests: a [`Transport`] whose sockets the test
//! talks through, line by line, in the order they were opened.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use super::hub::{Socket, SocketEvent, Transport};

/// One socket as the fake relay sees it.
#[derive(Default)]
struct Line {
    url: String,
    sent: Vec<String>,
    events: VecDeque<SocketEvent>,
    dropped: bool,
}

/// The other end of every socket a hub opened through it.
#[derive(Clone, Default)]
pub(crate) struct Fake(Rc<RefCell<Vec<Line>>>);

struct FakeSocket {
    relay: Fake,
    n: usize,
}

impl Transport for Fake {
    fn connect(&mut self, url: &str) -> Box<dyn Socket> {
        let mut lines = self.0.borrow_mut();
        lines.push(Line {
            url: url.to_owned(),
            ..Line::default()
        });
        Box::new(FakeSocket {
            relay: self.clone(),
            n: lines.len() - 1,
        })
    }
}

impl Socket for FakeSocket {
    fn send(&mut self, text: &str) -> bool {
        self.relay.0.borrow_mut()[self.n].sent.push(text.to_owned());
        true
    }

    fn poll(&mut self) -> Vec<SocketEvent> {
        self.relay.0.borrow_mut()[self.n].events.drain(..).collect()
    }
}

impl Drop for FakeSocket {
    fn drop(&mut self) {
        self.relay.0.borrow_mut()[self.n].dropped = true;
    }
}

impl Fake {
    /// How many sockets were opened.
    pub(crate) fn count(&self) -> usize {
        self.0.borrow().len()
    }

    pub(crate) fn url(&self, n: usize) -> String {
        self.0.borrow()[n].url.clone()
    }

    /// Whether socket `n` was closed (dropped).
    pub(crate) fn dropped(&self, n: usize) -> bool {
        self.0.borrow()[n].dropped
    }

    /// What socket `n` sent since last asked.
    pub(crate) fn sent(&self, n: usize) -> Vec<String> {
        std::mem::take(&mut self.0.borrow_mut()[n].sent)
    }

    /// Socket `n` reports `e` at its next poll.
    pub(crate) fn event(&self, n: usize, e: SocketEvent) {
        self.0.borrow_mut()[n].events.push_back(e);
    }

    /// The relay says `text` on socket `n`.
    pub(crate) fn say(&self, n: usize, text: &str) {
        self.event(n, SocketEvent::Text(text.to_owned()));
    }
}
