//! The run under the debugging tools: the Debug menu and its keys (Pause,
//! a tick or a second at a time, slow motion, stopping scripts), and the
//! tools' switch.

use formats::dialog::MenuItem;

use super::super::{Editor, TICKS_PER_SECOND, TickEnd, menu::Menu};
use crate::lang::tr;

/// The Debug menu's commands (the port's own, after the others' 0x9001…).
pub mod cmd {
    pub const PAUSE: u16 = 0x9010;
    pub const STEP_TICK: u16 = 0x9011;
    pub const STEP_SECOND: u16 = 0x9012;
    pub const SPEED_1: u16 = 0x9013;
    pub const SPEED_2: u16 = 0x9014;
    pub const SPEED_4: u16 = 0x9015;
    pub const SPEED_10: u16 = 0x9016;
    pub const SPEED_60: u16 = 0x9017;
    pub const STOP_SCRIPTS: u16 = 0x9018;
}

/// F6 and F7: Pause and Step 1 Tick.
const VK_F6: u16 = 0x75;
const VK_F7: u16 = 0x76;

/// Whether `id` is one of the Debug menu's commands.
#[must_use]
pub fn is_command(id: u16) -> bool {
    (cmd::PAUSE..=cmd::STOP_SCRIPTS).contains(&id)
}

/// How fast the run goes: the time the widget sees is the real time
/// divided by this.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Speed {
    #[default]
    Normal,
    Half,
    Quarter,
    Tenth,
    Sixtieth,
}

impl Speed {
    pub const ALL: [Speed; 5] = [
        Speed::Normal,
        Speed::Half,
        Speed::Quarter,
        Speed::Tenth,
        Speed::Sixtieth,
    ];

    /// The real time's divisor.
    #[must_use]
    pub fn divisor(self) -> u64 {
        match self {
            Speed::Normal => 1,
            Speed::Half => 2,
            Speed::Quarter => 4,
            Speed::Tenth => 10,
            Speed::Sixtieth => 60,
        }
    }

    /// Its menu command.
    #[must_use]
    pub fn command(self) -> u16 {
        match self {
            Speed::Normal => cmd::SPEED_1,
            Speed::Half => cmd::SPEED_2,
            Speed::Quarter => cmd::SPEED_4,
            Speed::Tenth => cmd::SPEED_10,
            Speed::Sixtieth => cmd::SPEED_60,
        }
    }

    /// As the status shows it: `½×`, `¼×`, `1/10×`; none at full speed.
    #[must_use]
    pub fn label(self) -> Option<&'static str> {
        match self {
            Speed::Normal => None,
            Speed::Half => Some("½×"),
            Speed::Quarter => Some("¼×"),
            Speed::Tenth => Some("1/10×"),
            Speed::Sixtieth => Some("1/60×"),
        }
    }
}

/// The Debug menu's items.
fn items() -> Vec<MenuItem> {
    let item = |id: u16, text: &str| MenuItem::Command {
        id,
        text: text.to_owned(),
        flags: 0,
    };
    vec![
        item(cmd::PAUSE, tr("&Pause\tF6")),
        item(cmd::STEP_TICK, tr("&Step 1 Tick\tF7")),
        item(cmd::STEP_SECOND, tr("Step 1 S&econd")),
        MenuItem::Separator,
        item(cmd::SPEED_1, tr("&Normal Speed")),
        item(cmd::SPEED_2, tr("Slow Motion &1/2")),
        item(cmd::SPEED_4, tr("Slow Motion 1/&4")),
        item(cmd::SPEED_10, tr("Slow Motion 1/1&0")),
        item(cmd::SPEED_60, tr("Slow Motion 1/&60")),
        MenuItem::Separator,
        item(cmd::STOP_SCRIPTS, tr("Stop Sc&ripts")),
    ]
}

/// Whether `menu` has the Debug menu.
fn has_debug(menu: &Menu) -> bool {
    menu.items.iter().any(|p| match p {
        MenuItem::Popup { items, .. } => items
            .iter()
            .any(|i| matches!(i, MenuItem::Command { id, .. } if *id == cmd::PAUSE)),
        _ => false,
    })
}

impl Editor {
    /// Whether the debugging tools are there: the enhancements and their
    /// own switch on.
    #[must_use]
    pub fn debugging(&self) -> bool {
        self.tray.enhanced && self.debug.tools
    }

    /// The debugging tools' switch, as the host kept it.
    pub fn set_debug_tools(&mut self, on: bool) {
        self.debug.tools = on;
        self.update_menu();
    }

    #[must_use]
    pub fn debug_tools(&self) -> bool {
        self.debug.tools
    }

    /// The debugging tools' switch as the player set it in the Settings
    /// window, once, for the host to keep.
    pub fn take_debug_tools(&mut self) -> Option<bool> {
        self.debug.new_tools.take()
    }

    /// Settings ▸ Debugging tools.
    pub(in crate::editor) fn debug_tools_changed(&mut self, on: bool) {
        self.set_debug_tools(on);
        self.debug.new_tools = Some(on);
    }

    /// The milliseconds of `ms` the running widget sees: none while
    /// paused, fewer in slow motion; all of them without the tools (which
    /// then let the run go as it was).
    pub(in crate::editor) fn debug_scale(&mut self, ms: u64) -> u64 {
        if !self.debugging() {
            self.debug.run.paused = false;
            self.debug.run.speed = Speed::Normal;
            return ms;
        }
        if self.debug.run.paused {
            return 0;
        }
        let d = self.debug.run.speed.divisor();
        let t = self.debug.run.carry + ms;
        self.debug.run.carry = t % d;
        t / d
    }

    /// The Debug menu's part of `update_menu`: there or not as the tools
    /// are (put in or taken out while no menu is open), its items greyed
    /// and checked.
    pub(in crate::editor) fn update_debug_menu(&mut self) {
        let on = self.debugging();
        if on != has_debug(&self.menu) && !self.menu.is_open() {
            let (grayed, checked) = (self.menu.grayed.clone(), self.menu.checked.clone());
            self.menu = self.load_menu();
            self.menu.grayed = grayed;
            self.menu.checked = checked;
        }
        if !on {
            return;
        }
        if !self.running {
            self.menu.grayed.push(cmd::PAUSE);
        }
        if !self.debug.scripts_running {
            self.menu.grayed.push(cmd::STOP_SCRIPTS);
        }
        if self.debug.run.paused {
            self.menu.checked.push(cmd::PAUSE);
        }
        self.menu.checked.push(self.debug.run.speed.command());
    }

    /// The menu bar from the resources gets the Debug menu with the tools.
    pub(in crate::editor) fn add_debug_menu(&self, menu: &mut Menu) {
        if self.debugging() {
            menu.add_debug(items());
        }
    }

    /// Runs Debug menu command `id`.
    pub(in crate::editor) fn debug_command(&mut self, id: u16) {
        if !self.debugging() {
            return;
        }
        match id {
            cmd::PAUSE => {
                if self.running {
                    self.debug.run.paused = !self.debug.run.paused;
                }
            }
            cmd::STEP_TICK => self.debug_step(1),
            cmd::STEP_SECOND => self.debug_step(u32::try_from(TICKS_PER_SECOND).unwrap_or(60)),
            cmd::STOP_SCRIPTS => self.debug.stop_scripts = true,
            _ => {
                if let Some(s) = Speed::ALL.into_iter().find(|s| s.command() == id) {
                    self.debug.run.speed = s;
                    self.debug.run.carry = 0;
                }
            }
        }
    }

    /// Step: the run held, then `n` ticks of it (fewer if a part ends it);
    /// from editing, GO first.
    fn debug_step(&mut self, n: u32) {
        if !self.running {
            self.run();
        }
        self.debug.run.paused = true;
        for _ in 0..n {
            if let TickEnd::Stopped { .. } = self.tick_once() {
                break;
            }
        }
    }

    /// The tools' keys, F6 and F7 (alone); whether `vk` was one.
    pub(in crate::editor) fn debug_key(&mut self, vk: u16) -> bool {
        if !self.debugging() || self.shift || self.ctrl || self.alt {
            return false;
        }
        let id = match vk {
            VK_F6 => cmd::PAUSE,
            VK_F7 => cmd::STEP_TICK,
            _ => return false,
        };
        self.command(id);
        true
    }
}
