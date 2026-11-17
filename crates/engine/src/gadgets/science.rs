//! The science lab: the Pendulum (`Pend`), the Gravity Chamber (`Drop`),
//! the Cannon (`Cano`), the Solar System (`Plnt`) and Area / Volume
//! (`ArVo`). docs/gadgets/science.md has the original's code; the physics
//! is closed-form, timed by the run clock in Mac ticks (1/60 s), in the
//! edition's units ([`Physics`]) and seconds.

use formats::hr::Rect;

use super::common::{any, draw_broken, parse_num, point, scan_f64, trunc, trunc_sat};
use crate::assets::Module;
use crate::gadget::{Behavior, Core, DrawCx};
use crate::lang::Lang;
use crate::sim::Io;

/// Preset heights in feet (`0x4903bc`): Mt. Everest, Mt. Vesuvius, the
/// Empire State Building, the Eiffel Tower, a T. rex, a man.
const HEIGHTS_FT: [f64; 6] = [
    f64::from_bits(0x40dc_5940_0000_0000),
    f64::from_bits(0x40b0_5e00_0000_0000),
    f64::from_bits(0x4097_0000_0000_0000),
    f64::from_bits(0x408e_c000_0000_0000),
    f64::from_bits(0x4034_0000_0000_0000),
    f64::from_bits(0x4018_0000_0000_0000),
];

/// Gravity in ft/s² (`0x4903ec`): the Sun, Jupiter, the Earth, the Moon.
const GRAVITY_FT_S2: [f64; 4] = [
    f64::from_bits(0x408c_2970_a3d7_0a3e),
    f64::from_bits(0x4052_fd38_ef34_d6a1),
    f64::from_bits(0x4040_17ae_147a_e148),
    f64::from_bits(0x4015_e2c3_c9ee_cbfc),
];

/// The cannon's launch speeds in ft/s (`0x49ab88`): Apollo 10, a rifle
/// bullet, a jet, a baseball pitch, none.
const CANNON_SPEED_FT_S: [f64; 5] = [
    f64::from_bits(0x40e1_c104_4444_4444),
    f64::from_bits(0x4096_eaaa_aaaa_aaaa),
    f64::from_bits(0x4089_b666_6666_6666),
    f64::from_bits(0x4060_8000_0000_0000),
    0.0,
];

/// ft/s to mph (`0x44fe15`).
const MPH: f64 = 0.681_818_181_818_181_8;

/// The Swedish edition's heights in meters (Uppfinnarverkstan's
/// `WIDGET.EXE`, `0x4903cc`), rounded: 8848, 1280, 448, 300, 6, 1.8.
const HEIGHTS_M_SV: [f64; 6] = [
    f64::from_bits(0x40c1_4800_0000_0000),
    f64::from_bits(0x4094_0000_0000_0000),
    f64::from_bits(0x407c_0000_0000_0000),
    f64::from_bits(0x4072_c000_0000_0000),
    f64::from_bits(0x4018_0000_0000_0000),
    f64::from_bits(0x3ffc_cccc_cccc_cccd),
];

/// The French edition's heights in meters (Eurêka's `WIDGET.EXE`,
/// `0x4903d8`): the English feet times 0.3048, 8848.0392 down to 1.8288.
const HEIGHTS_M_FR: [f64; 6] = [
    f64::from_bits(0x40c1_4805_0481_6f01),
    f64::from_bits(0x4093_f472_b020_c49c),
    f64::from_bits(0x407c_0aa6_4c2f_837c),
    f64::from_bits(0x4072_bec5_6d5c_faad),
    f64::from_bits(0x4018_624d_d2f1_a9fc),
    f64::from_bits(0x3ffd_42c3_c9ee_cbfc),
];

/// Gravity in m/s², the same in both editions (Swedish `0x4903fc`, French
/// `0x490408`): 28, 2.36, 1 and 0.17 times 9.81.
const GRAVITY_M_S2: [f64; 4] = [
    f64::from_bits(0x4071_2ae1_47ae_147b),
    f64::from_bits(0x4037_26cf_41f2_12d7),
    f64::from_bits(0x4023_9eb8_51eb_851f),
    f64::from_bits(0x3ffa_aee6_31f8_a091),
];

/// The Swedish edition's launch speeds in m/s (`0x49ac70`): 27 200,
/// 1100, 600 and 100 km/h, none.
const CANNON_SPEED_M_S_SV: [f64; 5] = [
    f64::from_bits(0x40bd_838e_38e3_8e39),
    f64::from_bits(0x4073_18e3_8e38_e38e),
    f64::from_bits(0x4064_d555_5555_5555),
    f64::from_bits(0x403b_c71c_71c7_1c72),
    0.0,
];

/// The French edition's launch speeds in m/s (`0x49abd4`): 39 889, 1609,
/// 903 and 145 km/h (the English mph in km/h), none.
const CANNON_SPEED_M_S_FR: [f64; 5] = [
    f64::from_bits(0x40c5_a423_8e38_e38e),
    f64::from_bits(0x407b_ef1c_71c7_1c72),
    f64::from_bits(0x406f_5aaa_aaaa_aaab),
    f64::from_bits(0x4044_238e_38e3_8e39),
    0.0,
];

/// m/s to km/h (Swedish `0x44ed3d`, French `0x44f209`), where the
/// English build has [`MPH`].
const KMH: f64 = f64::from_bits(0x400c_cccc_cccc_cccd);

/// An edition's units. The Swedish (Uppfinnarverkstan) and French
/// (Eurêka) editions reckon in meters, kilometers and kilograms where the
/// English build has feet, miles and pounds: their code is the English
/// code, with these tables and the cannon's speed factor changed
/// (docs/gadgets/science.md, "The editions").
#[derive(Debug)]
pub struct Physics {
    /// The preset heights: Mt. Everest, Mt. Vesuvius, the Empire State
    /// Building, the Eiffel Tower, a T. rex, a person.
    pub heights: [f64; 6],
    /// Gravity: the Sun, Jupiter, the Earth, the Moon.
    pub gravity: [f64; 4],
    /// The cannon's launch speeds: Apollo 10, a rifle bullet, a jet, a
    /// pitch, none.
    pub cannon_speed: [f64; 5],
    /// The cannon's Speed output per unit of speed: ft/s to mph, or m/s to
    /// km/h.
    pub speed_out: f64,
    /// The Solar System's numbers, Mercury to Pluto.
    planets: [PlanetData; 9],
}

/// The English build's: feet, miles, pounds.
const IMPERIAL: Physics = Physics {
    heights: HEIGHTS_FT,
    gravity: GRAVITY_FT_S2,
    cannon_speed: CANNON_SPEED_FT_S,
    speed_out: MPH,
    planets: PLANETS,
};

/// The Swedish edition's.
const SWEDISH: Physics = Physics {
    heights: HEIGHTS_M_SV,
    gravity: GRAVITY_M_S2,
    cannon_speed: CANNON_SPEED_M_S_SV,
    speed_out: KMH,
    planets: PLANETS_SV,
};

/// The French edition's.
const FRENCH: Physics = Physics {
    heights: HEIGHTS_M_FR,
    gravity: GRAVITY_M_S2,
    cannon_speed: CANNON_SPEED_M_S_FR,
    speed_out: KMH,
    planets: PLANETS_FR,
};

impl Physics {
    /// The units of the edition in `lang`: the game's language
    /// ([`crate::assets::Assets::lang`]), the CD's or the data pack's.
    #[must_use]
    pub fn of(lang: Lang) -> &'static Physics {
        match lang {
            Lang::En => &IMPERIAL,
            Lang::Sv => &SWEDISH,
            Lang::Fr => &FRENCH,
        }
    }
}

/// The units of the game's language.
fn physics(io: &Io<'_>) -> &'static Physics {
    Physics::of(io.assets.lang)
}

/// 2π as the pendulum stores it (`0x44f1af`, 6.283185307178).
const TWO_PI: f64 = f64::from_bits(0x4019_21fb_5444_261e);
/// 3π/2 (`0x49aaa4`): the pendulum's phase before release.
const THREE_HALF_PI: f64 = f64::from_bits(0x4012_d97c_7f33_1c96);
/// The swing's amplitude in radians (`0x44f04f`, 36°).
const AMPLITUDE: f64 = f64::from_bits(0x3fe4_1b2f_769c_eb4c);
/// 1/2π (`0x44f117`, an x87 extended constant).
const INV_TWO_PI: f64 = 0.159_154_943_091_895_35;

fn table(t: &[f64], i: u32) -> f64 {
    usize::try_from(i).ok().and_then(|i| t.get(i)).copied().unwrap_or(0.0)
}

/// Seconds since `since` on the run clock (`(u32)(now - since) * 1/60`).
fn seconds(io: &Io<'_>, since: u32) -> f64 {
    f64::from(io.elapsed().wrapping_sub(since)) / 60.0
}

/// Frame `n`'s half width and height (for centring), as C divides.
fn half_size(cx: &DrawCx<'_>, sheet: u32, n: u16) -> (i32, i32) {
    cx.pictures
        .rect(cx.assets, Module::Gadget, sheet, n)
        .map_or((0, 0), |r| {
            (
                (i32::from(r.right) - i32::from(r.left)) / 2,
                (i32::from(r.bottom) - i32::from(r.top)) / 2,
            )
        })
}

// ---------------------------------------------------------------------------
// The Pendulum

/// The phase angle `t` seconds after release (`0x44f1b7`).
#[must_use]
pub fn pendulum_phase(t: f64, period: f64) -> f64 {
    if period == 0.0 {
        THREE_HALF_PI
    } else {
        TWO_PI * t / period + THREE_HALF_PI
    }
}

/// The period of a pendulum `length` long under `gravity` (feet and
/// ft/s², or meters and m/s²) (`0x44f137`).
#[must_use]
pub fn pendulum_period(length: f64, gravity: f64) -> f64 {
    (length / gravity).sqrt() * TWO_PI
}

/// `0x44f0d7`: +1 for the first half of each turn of `x`, −1 for the
/// second (the sign of `sin x` for `x >= 0`).
#[must_use]
pub fn half_turn_sign(x: f64) -> f64 {
    if (x * INV_TWO_PI).fract() < 0.5 { 1.0 } else { -1.0 }
}

/// cPendulum: swings once released (input 0), with the period set by its
/// length and gravity; outputs the period, or a pulse each time it passes
/// the centre.
#[derive(Debug)]
pub struct Pendulum {
    /// The phase at the last look (`+0x197`).
    prev: f64,
    /// The phase (`+0x19f`).
    phase: f64,
    /// When it was released, in run ticks; 0 not yet (`+0x1a7`).
    released: u32,
    /// The period in seconds (`+0x1ab`).
    period: f64,
}

impl Default for Pendulum {
    fn default() -> Pendulum {
        Pendulum {
            prev: THREE_HALF_PI,
            phase: THREE_HALF_PI,
            released: 0,
            period: 0.0,
        }
    }
}

#[must_use]
pub fn pendulum() -> Box<dyn Behavior> {
    Box::new(Pendulum::default())
}

impl Behavior for Pendulum {
    fn draw(&self, core: &Core, cx: &mut DrawCx<'_>) {
        cx.frame(core.sheet, 1, core.h, core.v);
        #[allow(clippy::cast_possible_truncation)]
        let a = f64::from((self.phase.sin() * AMPLITUDE) as f32);
        #[allow(clippy::cast_possible_truncation)]
        let (dx, dy) = (trunc(a.sin() * 74.0), trunc(a.cos() * 74.0));
        let (ph, pv) = point(cx, core.sheet, 0);
        let (px, py) = (core.h + ph, core.v + pv);
        let c = cx.color([255, 255, 0]);
        cx.surface.line((px, py), (px + dx, py + dy), c);
        let (hw, hh) = half_size(cx, core.sheet, 2);
        cx.frame(core.sheet, 2, px + dx - hw, py + dy - hh);
    }

    fn tick(&mut self, core: &mut Core, io: &mut Io<'_>) {
        if !io.throttle(core, 6) {
            return;
        }
        if self.released == 0 {
            if io.number(core, 0) != 0.0 {
                self.released = io.elapsed();
            }
            return;
        }
        self.phase = pendulum_phase(seconds(io, self.released), self.period);
        let crossed = half_turn_sign(self.prev) * half_turn_sign(self.phase) < 0.0;
        match core.settings.radios[1] {
            0 => {
                core.number = self.period;
                core.state = false;
            }
            1 => {
                core.state = crossed;
                core.number = 0.0;
            }
            _ => {}
        }
        self.prev = self.phase;
        if crossed {
            io.play(1000);
            io.fire(core, 1);
        }
    }

    fn reset(&mut self, core: &mut Core) {
        core.base_reset();
        self.prev = THREE_HALF_PI;
        self.phase = THREE_HALF_PI;
        self.released = 0;
    }

    fn start(&mut self, core: &mut Core, io: &mut Io<'_>) {
        io.base_start(core);
        let (s, units) = (&core.settings, physics(io));
        let length = if s.radios[0] == 0 {
            table(&units.heights, s.numbers[0])
        } else {
            parse_num(s.label(0))
        };
        self.period = pendulum_period(length, table(&units.gravity, s.numbers[1]));
    }

    fn has_bool(&self, core: &Core, _knot: usize) -> bool {
        core.settings.radios[1] != 0
    }

    fn has_number(&self, core: &Core, knot: usize) -> bool {
        !self.has_bool(core, knot)
    }

    fn init(&mut self, core: &mut Core) {
        // On at Center; the Empire State Building.
        core.settings.radios[1] = 1;
        core.settings.numbers[0] = 2;
    }

    any!();
}

// ---------------------------------------------------------------------------
// The Gravity Chamber

/// How far down the picture is, in pixels, after falling `t` seconds from
/// `height` under `gravity` toward a floor `floor` pixels down
/// (`0x446f00`).
#[must_use]
pub fn drop_pixels(t: f64, gravity: f64, height: f64, floor: i16) -> i32 {
    let d = t * t * gravity * 0.5;
    if height == 0.0 {
        0x7fff_ffff
    } else {
        trunc(d / height * f64::from(floor))
    }
}

/// Each object's floor, in pixels below its falling picture's start
/// (`0x499868`): feather, elephant, baseball.
const FLOOR_PX: [i16; 3] = [79, 77, 94];
/// Each object's first frame (`0x49986e`) and first point (`0x499874`).
const FRAME_BASE: [u16; 3] = [4, 10, 16];
const POINT_BASE: [usize; 3] = [1, 7, 13];

/// cDropChamber: a claw opens and drops a feather, an elephant or a
/// baseball; the output goes on when it hits the floor.
#[derive(Debug)]
pub struct DropChamber {
    /// The height in feet or meters (`+0x197`); kept when the custom
    /// height can't be read.
    height: f64,
    /// When the fall began, in run ticks (`+0x19f`).
    fell_at: u32,
    /// How far it has fallen, in pixels (`+0x1a3`), and as last drawn
    /// (`+0x1a7`).
    y: i32,
    prev_y: i32,
}

impl Default for DropChamber {
    fn default() -> DropChamber {
        DropChamber {
            height: 0.0,
            fell_at: 0,
            y: 0,
            prev_y: 0,
        }
    }
}

#[must_use]
pub fn drop_chamber() -> Box<dyn Behavior> {
    Box::new(DropChamber::default())
}

fn thing(core: &Core) -> usize {
    usize::try_from(core.settings.numbers[2]).map_or(0, |i| i.min(2))
}

impl Behavior for DropChamber {
    fn draw(&self, core: &Core, cx: &mut DrawCx<'_>) {
        let s = core.sheet;
        cx.frame(s, 1, core.h, core.v);
        let th = thing(core);
        let at = |cx: &DrawCx<'_>, i: usize| {
            let (h, v) = point(cx, s, i);
            (core.h + h, core.v + v)
        };
        if core.phase < 4 {
            let (x, y) = at(cx, POINT_BASE[th] + usize::from(core.phase));
            cx.frame(s, FRAME_BASE[th] + u16::from(core.phase), x, y);
        } else {
            let (x, y) = at(cx, 0);
            cx.frame(s, 3, x, y);
            if i32::from(FLOOR_PX[th]) == self.y {
                let (x, y) = at(cx, POINT_BASE[th] + 5);
                cx.frame(s, FRAME_BASE[th] + 5, x, y);
            } else {
                let (x, y) = at(cx, POINT_BASE[th] + 4);
                // The original adds the pixels as a 16-bit coordinate.
                #[allow(clippy::cast_possible_truncation)]
                let dy = i32::from(self.y as i16);
                cx.frame(s, FRAME_BASE[th] + 4, x, y + dy);
            }
        }
    }

    #[allow(clippy::cast_sign_loss)]
    fn tick(&mut self, core: &mut Core, io: &mut Io<'_>) {
        match core.phase {
            0 => {
                if io.number(core, 0) != 0.0 {
                    core.phase = 1;
                }
            }
            1 => {
                if io.throttle(core, 5) {
                    core.phase = 2;
                }
            }
            2 => {
                if io.throttle(core, 5) {
                    core.phase = 3;
                    io.play(1101);
                }
            }
            3 => {
                if io.throttle(core, 5) {
                    core.phase = 4;
                    self.fell_at = io.elapsed();
                }
            }
            4 => {
                let g = table(&physics(io).gravity, core.settings.numbers[0]);
                let t = seconds(io, self.fell_at);
                let floor = FLOOR_PX[thing(core)];
                self.y = drop_pixels(t, g, self.height, floor);
                core.number = t * g;
                if self.y as u32 > self.prev_y as u32 {
                    if self.y as u32 >= i32::from(floor) as u32 {
                        self.y = i32::from(floor);
                        core.number = 0.0;
                        core.state = true;
                        io.play(1001);
                        core.phase = 5;
                        io.fire(core, 1);
                    }
                    self.prev_y = self.y;
                }
            }
            _ => {}
        }
    }

    fn reset(&mut self, core: &mut Core) {
        core.base_reset();
        self.prev_y = 0;
        self.y = 0;
        self.fell_at = 0;
    }

    fn start(&mut self, core: &mut Core, io: &mut Io<'_>) {
        io.base_start(core);
        let s = &core.settings;
        if s.radios[0] == 0 {
            self.height = table(&physics(io).heights, s.numbers[1]);
        } else if let Some(h) = scan_f64(s.label(0)) {
            self.height = h;
        }
    }

    fn has_bool(&self, _core: &Core, _knot: usize) -> bool {
        true
    }

    fn init(&mut self, core: &mut Core) {
        // An elephant, on Earth, from the Empire State Building.
        core.settings.numbers = [2, 2, 1, 0];
    }

    any!();
}

// ---------------------------------------------------------------------------
// The Cannon

/// A cannon shot's flight time in seconds from `height` (feet or meters)
/// (`0x44fa0d`).
#[must_use]
pub fn flight_time(height: f64, gravity: f64) -> f64 {
    (2.0 * height / gravity).sqrt()
}

/// cPhysicsCanon: fires a ball level from a height; outputs the distance,
/// the height or the speed while it flies, or goes on when it lands.
#[derive(Debug)]
pub struct PhysicsCanon {
    /// Ready to fire (`+0x13f`); cleared when the ball lands.
    armed: bool,
    /// When it fired, in run ticks; 0 not yet (`+0x197`).
    fired_at: u32,
    /// The ball's offset in pixels (`+0x19f` across, `+0x19b` down).
    ball_x: i32,
    ball_y: i32,
    /// The height in feet or meters and the flight time in seconds
    /// (`+0x1a3`, `+0x1ab`).
    height: f64,
    flight: f64,
}

impl Default for PhysicsCanon {
    fn default() -> PhysicsCanon {
        PhysicsCanon {
            armed: true,
            fired_at: 0,
            ball_x: 0,
            ball_y: 0,
            height: 0.0,
            flight: 0.0,
        }
    }
}

#[must_use]
pub fn physics_canon() -> Box<dyn Behavior> {
    Box::new(PhysicsCanon::default())
}

impl Behavior for PhysicsCanon {
    fn draw(&self, core: &Core, cx: &mut DrawCx<'_>) {
        cx.frame(core.sheet, 1, core.h, core.v);
        // The original adds the offsets as 16-bit coordinates.
        #[allow(clippy::cast_possible_truncation)]
        let (dx, dy) = (i32::from(self.ball_x as i16), i32::from(self.ball_y as i16));
        cx.frame(core.sheet, 3, core.h + 19 + dx, core.v + 5 + dy);
    }

    fn tick(&mut self, core: &mut Core, io: &mut Io<'_>) {
        if !self.armed {
            return;
        }
        if self.fired_at == 0 {
            if io.number(core, 0) != 0.0 {
                self.fired_at = io.elapsed();
            }
            return;
        }
        let (s, units) = (&core.settings, physics(io));
        let g = table(&units.gravity, s.numbers[1]);
        let v = table(&units.cannon_speed, s.numbers[0]);
        let mode = s.radios[1];
        let t = seconds(io, self.fired_at);
        let vy = g * t;
        let d = vy * t * 0.5;
        if self.height < 1e-100 || self.flight < 1e-100 {
            self.ball_y = 66;
            self.ball_x = 158;
        } else {
            self.ball_y = trunc_sat(d / self.height * 66.0);
            self.ball_x = trunc_sat(t / self.flight * 158.0);
        }
        match mode {
            0 => core.number = t * v,
            1 => core.number = self.height - d,
            3 => core.number = (v * v + vy * vy).sqrt() * units.speed_out,
            _ => {}
        }
        if self.ball_x >= 158 {
            self.armed = false;
            self.ball_y = 66;
            self.ball_x = 158;
            io.play(1002);
            self.fired_at = 0;
            io.fire(core, 1);
            match mode {
                0 => core.number = v * self.flight,
                1 => core.number = 0.0,
                2 => core.state = true,
                3 => {
                    let vy = g * self.flight;
                    core.number = (vy * vy + v * v).sqrt() * units.speed_out;
                }
                _ => {}
            }
        }
    }

    fn reset(&mut self, core: &mut Core) {
        core.base_reset();
        self.armed = true;
        self.ball_x = 0;
        self.ball_y = 0;
        self.fired_at = 0;
    }

    fn start(&mut self, core: &mut Core, io: &mut Io<'_>) {
        io.base_start(core);
        let (s, units) = (&core.settings, physics(io));
        self.height = if s.radios[0] == 0 {
            table(&units.heights, s.numbers[2])
        } else {
            parse_num(s.label(0))
        };
        self.flight = flight_time(self.height, table(&units.gravity, s.numbers[1]));
        match s.radios[1] {
            0 | 3 => core.number = 0.0,
            1 => core.number = self.height,
            2 => core.state = false,
            _ => {}
        }
        self.armed = true;
    }

    fn has_bool(&self, core: &Core, _knot: usize) -> bool {
        core.settings.radios[1] == 2
    }

    fn has_number(&self, core: &Core, knot: usize) -> bool {
        !self.has_bool(core, knot)
    }

    fn init(&mut self, core: &mut Core) {
        // On When Hits.
        core.settings.radios[1] = 2;
    }

    any!();
}

// ---------------------------------------------------------------------------
// The Solar System

/// A planet's numbers (`0x49ac42`, 0x44 bytes each, after a pointer to
/// its caption).
#[derive(Debug, Clone, Copy)]
struct PlanetData {
    /// Earth hours.
    day: f64,
    /// Earth days.
    year: f64,
    /// Million miles (million km) from the Sun.
    distance: f64,
    /// Thousand mph (thousand km/h).
    orbit_speed: f64,
    /// Miles (km).
    diameter: f64,
    /// Water = 1.
    density: f64,
    /// Earth = 1.
    gravity: f64,
    /// 10¹³ gigatons, the English label says (the Swedish edition's says
    /// 10¹², the French 10¹³).
    mass: f64,
}

macro_rules! planets {
    ($name:ident; $($day:literal $year:literal $dist:literal $orbit:literal $diam:literal $dens:literal $grav:literal $mass:literal;)*) => {
        const $name: [PlanetData; 9] = [$(PlanetData {
            day: $day,
            year: $year,
            distance: $dist,
            orbit_speed: $orbit,
            diameter: $diam,
            density: $dens,
            gravity: $grav,
            mass: $mass,
        },)*];
    };
}

/// The captions (`0x49aed6`), in English; drawn through
/// [`crate::lang::tr`], as the editions have their own (the Swedish
/// "MERKURIUS" at `0x49afbe`, the French "Mercure" at `0x49af22`).
const PLANET_NAMES: [&str; 9] = [
    "MERCURY", "VENUS", "EARTH", "MARS", "JUPITER", "SATURN", "URANUS", "NEPTUNE", "PLUTO",
];

// The English build's, Mercury to Pluto.
planets! { PLANETS;
    59.0 87.97 36.0 106.26 3032.0 5.5 0.38 0.332;
    243.0 224.0 67.0 77.62 7523.0 5.24 0.9 4.89;
    24.0 365.26 93.0 66.0 7926.0 5.52 1.0 6.04;
    24.62 687.0 142.0 53.46 4420.0 3.94 0.38 0.642;
    9.83 4332.0 483.0 28.93 88750.0 1.3 2.87 1920.0;
    10.23 10760.0 887.0 21.35 74580.0 0.7 1.32 575.0;
    10.82 30685.0 1783.0 15.09 31600.0 1.3 0.93 88.2;
    15.8 60191.0 2794.0 12.03 30200.0 1.8 1.23 103.9;
    159.32 90475.0 3660.0 10.5 1500.0 0.7 0.03 0.06;
}

// The Swedish edition's (Uppfinnarverkstan's `WIDGET.EXE`, `0x49ad2a`), in
// km, million km and thousand km/h. Venus is 842 km across there, not
// about 12 100: kept, as the edition has it.
planets! { PLANETS_SV;
    59.0 87.97 58.0 172.0 4880.0 5.42 0.38 0.33;
    243.0 224.7 108.0 126.0 842.0 5.26 0.9 4.87;
    24.0 365.26 149.0 107.24 12755.0 5.52 1.0 5.98;
    24.62 687.0 228.0 86.86 7113.0 3.94 0.38 0.642;
    9.83 4332.0 778.0 47.01 142_825.0 1.3 2.53 1920.0;
    10.23 10760.0 1429.0 34.78 120_022.0 0.7 1.07 569.0;
    10.82 30685.0 2870.0 24.48 50853.0 1.3 0.9 86.9;
    15.8 60190.0 4504.0 19.58 48600.0 1.8 1.1 103.9;
    159.32 90800.0 5913.0 17.06 2414.0 0.7 0.07 0.0132;
}

// The French edition's (Eurêka's `WIDGET.EXE`, `0x49ac8e`), in the same
// units: the English numbers converted.
planets! { PLANETS_FR;
    59.0 87.97 58.0 170.0 4851.0 5.5 0.38 0.327;
    243.0 224.0 107.0 124.0 12037.0 5.24 0.9 4.812;
    24.0 365.26 149.0 106.0 12682.0 5.52 1.0 5.943;
    24.62 687.0 227.0 86.0 7072.0 3.94 0.38 0.632;
    9.83 4332.0 773.0 46.0 142_000.0 1.3 2.87 1889.28;
    10.23 10760.0 1419.0 34.0 119_328.0 0.7 1.32 565.0;
    10.82 30685.0 2853.0 24.0 50560.0 1.3 0.93 86.789;
    15.8 60191.0 4470.0 19.0 48320.0 1.8 1.23 102.238;
    159.32 90475.0 5856.0 17.0 2400.0 0.7 0.03 0.059;
}

/// cPlanet: shows a planet and outputs one of its numbers.
#[derive(Debug)]
pub struct Planet;

impl Default for Planet {
    fn default() -> Planet {
        Planet
    }
}

#[must_use]
pub fn planet() -> Box<dyn Behavior> {
    Box::new(Planet)
}

fn planet_index(core: &Core) -> usize {
    usize::try_from(core.settings.numbers[0]).map_or(0, |i| i.min(8))
}

/// The bezel, frame 14 of sheet 1022, is the planet's extent (`0x44feec`).
const PLANET_BOUNDS: Rect = Rect {
    top: 0,
    left: 0,
    bottom: 109,
    right: 118,
};

impl Behavior for Planet {
    fn draw(&self, core: &Core, cx: &mut DrawCx<'_>) {
        let s = core.sheet;
        cx.frame(s, 14, core.h, core.v);
        let p = planet_index(core);
        let n = u16::try_from(p + 1).unwrap_or(1);
        let (hw, hh) = cx.pictures.rect(cx.assets, Module::Gadget, s, n).map_or((0, 0), |r| {
            (
                i32::from(r.right) - i32::from(r.left),
                i32::from(r.bottom) - i32::from(r.top),
            )
        });
        cx.frame(s, n, core.h + 7 + (94 - hw) / 2, core.v + 7 + (88 - hh) / 2);
        // Small Fonts 11, green, bottom-aligned (TA_BOTTOM) at 107; centred
        // over 88 pixels, not the picture's 94.
        let name = crate::text::encode(crate::lang::tr(PLANET_NAMES[p]));
        let name = name.as_slice();
        let w = cx.text_width("Small Fonts", 11, false, name);
        let height = cx
            .assets
            .font("Small Fonts", 11, false)
            .map_or(11, |f| i32::from(f.height));
        let c = cx.color([0, 255, 0]);
        cx.text(
            "Small Fonts",
            11,
            false,
            core.h + 9 + (88 - w) / 2,
            core.v + 107 - height,
            name,
            c,
        );
    }

    fn tick(&mut self, core: &mut Core, io: &mut Io<'_>) {
        let p = &physics(io).planets[planet_index(core)];
        core.number = match core.settings.numbers[1] {
            0 => p.diameter,
            1 => p.day,
            2 => p.year,
            3 => p.distance,
            4 => p.mass,
            5 => p.density,
            6 => p.gravity,
            7 => p.orbit_speed,
            _ => 0.0,
        };
    }

    fn reset(&mut self, core: &mut Core) {
        core.base_reset();
        core.bounds = PLANET_BOUNDS;
    }

    fn has_number(&self, _core: &Core, _knot: usize) -> bool {
        true
    }

    fn init(&mut self, core: &mut Core) {
        // The Earth.
        core.settings.numbers[0] = 2;
    }

    any!();
}

// ---------------------------------------------------------------------------
// Area / Volume

/// π as Area / Volume stores it (`0x441d30`), short of `f64::consts::PI`.
#[allow(clippy::approx_constant)]
const ARVO_PI: f64 = 3.141_592_653_589;
/// 4π/3 (`0x441d30`).
const FOUR_THIRDS_PI: f64 = 4.188_790_204_785_333;

/// The area or volume of a square, circle, cube or sphere of side or
/// radius `x` (`0x441d30`).
#[must_use]
pub fn area_volume(shape: u8, x: f64) -> f64 {
    match shape {
        0 => x * x,
        1 => x * x * ARVO_PI,
        2 => x * x * x,
        3 => x * x * x * FOUR_THIRDS_PI,
        _ => 0.0,
    }
}

/// cAreaVolume: crunches a new input for a moment, then outputs the area
/// or volume; a negative input breaks it.
#[derive(Debug, Default)]
pub struct AreaVolume {
    /// Whether the input's wire was flagged changed when the crunching
    /// began (`+0x197`).
    wire_changed: bool,
}

#[must_use]
pub fn area_volume_gadget() -> Box<dyn Behavior> {
    Box::new(AreaVolume::default())
}

impl Behavior for AreaVolume {
    fn draw(&self, core: &Core, cx: &mut DrawCx<'_>) {
        cx.frame(core.sheet, u16::from(core.phase) + 1, core.h, core.v);
        let (h, v) = point(cx, core.sheet, 0);
        cx.frame(
            core.sheet,
            u16::from(core.settings.radios[0]) + 9,
            core.h + h,
            core.v + v,
        );
        if core.broken {
            draw_broken(core, cx);
        }
    }

    fn tick(&mut self, core: &mut Core, io: &mut Io<'_>) {
        if core.phase == 0 {
            let differs = io.value_changed(core, 0);
            self.wire_changed = io.changed(core, 0);
            if differs || self.wire_changed {
                core.phase = 1;
                io.play(1020);
            }
        } else if io.throttle(core, 9) {
            core.phase += 1;
            if core.phase == 8 {
                let x = io.number(core, 0);
                core.phase = 0;
                core.broken = x < 0.0;
                core.number = if core.broken {
                    0.0
                } else {
                    area_volume(core.settings.radios[0], x)
                };
                if self.wire_changed {
                    io.fire(core, 1);
                }
            }
        }
    }

    fn reset(&mut self, core: &mut Core) {
        core.base_reset();
        core.empty_cache = true;
    }

    fn has_number(&self, _core: &Core, _knot: usize) -> bool {
        true
    }

    fn init(&mut self, core: &mut Core) {
        // Volume of a sphere.
        core.settings.radios[0] = 3;
    }

    any!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gadgets::common::bench::Bench;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0)
    }

    #[test]
    fn pendulum_period_is_two_pi_root_l_over_g() {
        // 6 ft on Earth: 2π√(6/32.185) ≈ 2.7128 s.
        let p = pendulum_period(HEIGHTS_FT[5], GRAVITY_FT_S2[2]);
        assert!(close(p, TWO_PI * (6.0f64 / 32.185).sqrt()));
        assert!((p - 2.7128).abs() < 1e-3);
        // The phase starts held to one side and comes back each period.
        assert!(close(pendulum_phase(0.0, p), THREE_HALF_PI));
        assert!(close(pendulum_phase(p, p), THREE_HALF_PI + TWO_PI));
        assert!(close(pendulum_phase(5.0, 0.0), THREE_HALF_PI));
    }

    #[test]
    fn pendulum_pulses_at_the_centre() {
        let mut b = Bench::new(1.0);
        let mut p = Pendulum::default();
        b.core.settings.radios[1] = 1; // On at Center
        b.core.settings.numbers = [5, 2, 0, 0]; // 6 ft, Earth
        b.start(&mut p);
        let period = p.period;
        // Released at tick 10 (throttle passes: 10 - 0 > 6).
        b.tick_at(&mut p, 10);
        assert_eq!(p.released, 10);
        // Ticks every 7 run ticks; the bob passes the centre at P/4, 3P/4…
        let mut crossings = Vec::new();
        for t in (17..=10 + 60 * 6).step_by(7) {
            b.tick_at(&mut p, t);
            if b.core.state {
                crossings.push(f64::from(t - 10) / 60.0);
                assert!(b.fired(1));
            }
        }
        // Each crossing is seen within one throttle step after it happens.
        assert!(crossings.len() >= 4);
        for (k, &seen) in crossings.iter().enumerate() {
            #[allow(clippy::cast_precision_loss)]
            let at = (0.25 + 0.5 * k as f64) * period;
            assert!(
                seen >= at && seen < at + 7.0 / 60.0,
                "crossing {k} at {at}, seen {seen}"
            );
        }
    }

    #[test]
    fn pendulum_outputs_its_period() {
        let mut b = Bench::new(1.0);
        let mut p = Pendulum::default();
        b.core.settings.radios = [1, 0, 0, 0]; // Custom length, Period Length
        b.core.settings.numbers[1] = 3; // the Moon
        b.core.settings.set_label(0, b"20");
        b.start(&mut p);
        assert!(p.has_number(&b.core, 1));
        b.tick_at(&mut p, 7);
        b.tick_at(&mut p, 14);
        assert!(close(b.core.number, TWO_PI * (20.0 / GRAVITY_FT_S2[3]).sqrt()));
    }

    #[test]
    fn drop_lands_after_root_two_h_over_g() {
        // 20 ft on Earth: √(40/32.185) ≈ 1.1148 s.
        let g = GRAVITY_FT_S2[2];
        let h = HEIGHTS_FT[4];
        assert_eq!(drop_pixels(0.0, g, h, 77), 0);
        assert_eq!(drop_pixels((2.0 * h / g).sqrt() + 1e-9, g, h, 77), 77);
        assert_eq!(drop_pixels(1.0, g, h, 77), trunc(16.0925 / 20.0 * 77.0));
        assert_eq!(drop_pixels(1.0, g, 0.0, 77), 0x7fff_ffff);

        let mut b = Bench::new(1.0);
        let mut d = DropChamber::default();
        d.init(&mut b.core); // elephant, Earth, 1,472 ft
        b.core.settings.numbers[1] = 4; // 20 ft
        b.start(&mut d);
        b.tick_at(&mut d, 0);
        assert_eq!(b.core.phase, 1);
        b.tick_at(&mut d, 6);
        b.tick_at(&mut d, 12);
        b.tick_at(&mut d, 18);
        assert_eq!(b.core.phase, 4);
        assert_eq!(d.fell_at, 18);
        let land = 18.0 + (2.0 * h / g).sqrt() * 60.0;
        let mut t = 18;
        while b.core.phase == 4 {
            t += 1;
            b.tick_at(&mut d, t);
        }
        assert_eq!(b.core.phase, 5);
        assert!(b.core.state);
        assert!(b.fired(1));
        assert!(f64::from(t) >= land - 1.0 && f64::from(t) <= land + 1.0);
        assert_eq!(d.y, 77);
    }

    #[test]
    fn cannon_range_is_speed_times_flight_time() {
        let mut b = Bench::new(1.0);
        let mut c = PhysicsCanon::default();
        b.core.settings.radios = [0, 0, 0, 0]; // preset, Distance
        b.core.settings.numbers = [3, 2, 4, 0]; // 90 mph, Earth, 20 ft
        b.start(&mut c);
        let tf = flight_time(20.0, GRAVITY_FT_S2[2]);
        assert!(close(c.flight, (40.0f64 / 32.185).sqrt()));
        b.tick_at(&mut c, 1);
        assert_eq!(c.fired_at, 1);
        b.tick_at(&mut c, 31);
        assert!(close(b.core.number, 0.5 * 132.0));
        assert_eq!(c.ball_x, trunc(0.5 / tf * 158.0));
        let mut t = 31;
        while c.armed {
            t += 1;
            b.tick_at(&mut c, t);
        }
        assert!(b.fired(1));
        assert!(close(b.core.number, 132.0 * tf));
        assert_eq!((c.ball_x, c.ball_y), (158, 66));
        // Fires once per run.
        b.tick_at(&mut c, t + 100);
        assert!(!b.fired(1));
    }

    #[test]
    fn cannon_speed_at_landing_is_in_mph() {
        let mut b = Bench::new(1.0);
        let mut c = PhysicsCanon::default();
        b.core.settings.radios = [0, 3, 0, 0];
        b.core.settings.numbers = [4, 2, 5, 0]; // no speed, Earth, 6 ft
        b.start(&mut c);
        b.tick_at(&mut c, 1);
        let mut t = 1;
        while c.armed {
            t += 1;
            b.tick_at(&mut c, t);
        }
        let vy = GRAVITY_FT_S2[2] * flight_time(6.0, GRAVITY_FT_S2[2]);
        assert!(close(b.core.number, vy * MPH));
    }

    #[test]
    fn area_volume_formulas() {
        assert!(close(area_volume(0, 3.0), 9.0));
        assert!(close(area_volume(1, 2.0), 4.0 * ARVO_PI));
        assert!(close(area_volume(2, 3.0), 27.0));
        assert!(close(area_volume(3, 1.0), 4.0 / 3.0 * std::f64::consts::PI));
    }

    #[test]
    fn area_volume_crunches_then_outputs() {
        let mut b = Bench::new(2.0);
        let mut a = AreaVolume::default();
        a.init(&mut b.core); // sphere
        b.start(&mut a);
        // The cache starts "none", so a wired input counts as new.
        b.tick_at(&mut a, 0);
        assert_eq!(b.core.phase, 1);
        for k in 1..=7 {
            b.tick_at(&mut a, 10 * k);
        }
        assert_eq!(b.core.phase, 0);
        assert!(close(b.core.number, 8.0 * FOUR_THIRDS_PI));
        // Only a value difference, not a flagged wire: no changed flag.
        assert!(!b.fired(1));

        b.set_input(-1.0);
        b.tick_at(&mut a, 100);
        for k in 1..=7 {
            b.tick_at(&mut a, 100 + 10 * k);
        }
        assert!(b.core.broken);
        assert!(close(b.core.number, 0.0));
        assert!(b.fired(1));
    }

    #[test]
    fn planet_outputs_its_numbers() {
        let mut b = Bench::new(0.0);
        let mut p = Planet;
        p.init(&mut b.core);
        b.core.settings.numbers[1] = 6;
        b.tick_at(&mut p, 0);
        assert!(close(b.core.number, 1.0));
        b.core.settings.numbers = [4, 0, 0, 0];
        b.tick_at(&mut p, 1);
        assert!(close(b.core.number, 88750.0));
    }

    #[test]
    fn metric_editions_tables() {
        let (sv, fr) = (Physics::of(Lang::Sv), Physics::of(Lang::Fr));
        assert_eq!(sv.heights, [8848.0, 1280.0, 448.0, 300.0, 6.0, 1.8]);
        for (m, ft) in fr.heights.iter().zip(HEIGHTS_FT) {
            assert!(close(*m, ft * 0.3048), "{ft} ft");
        }
        for p in [sv, fr] {
            for (g, earths) in p.gravity.iter().zip([28.0, 2.36, 1.0, 0.17]) {
                assert!(close(*g, earths * 9.81));
            }
            assert_eq!(p.speed_out, 3.6);
        }
        let kmh = |p: &Physics| p.cannon_speed.map(|v| v * p.speed_out);
        for (v, k) in kmh(sv).iter().zip([27_200.0, 1100.0, 600.0, 100.0, 0.0]) {
            assert!(close(*v, k), "{v} km/h");
        }
        for (v, k) in kmh(fr).iter().zip([39_889.0, 1609.0, 903.0, 145.0, 0.0]) {
            assert!(close(*v, k), "{v} km/h");
        }
        // The English build's, in mph.
        for (v, k) in kmh(Physics::of(Lang::En))
            .iter()
            .zip([24_791.0, 1000.0, 561.0, 90.0, 0.0])
        {
            assert!(close(*v, k), "{v} mph");
        }
    }

    #[test]
    fn planets_in_the_language_units() {
        // The Earth's diameter: miles, then the editions' kilometers.
        for (lang, km) in [(Lang::En, 7926.0), (Lang::Sv, 12755.0), (Lang::Fr, 12682.0)] {
            let mut b = Bench::new(0.0);
            b.assets.lang = lang;
            let mut p = Planet;
            p.init(&mut b.core);
            b.tick_at(&mut p, 0);
            assert_eq!(b.core.number, km, "{lang:?}");
        }
        // Distance from the Sun, million km, and orbital speed, thousand km/h.
        let sv = &Physics::of(Lang::Sv).planets[2];
        assert_eq!((sv.distance, sv.orbit_speed), (149.0, 107.24));
        let fr = &Physics::of(Lang::Fr).planets[4];
        assert_eq!((fr.diameter, fr.mass), (142_000.0, 1889.28));
    }

    #[test]
    fn a_meter_long_pendulum_beats_two_seconds_on_earth() {
        // 2π√(1/9.81) ≈ 2.006 s: puzzle 5's answer in the metric editions.
        for lang in [Lang::Sv, Lang::Fr] {
            let mut b = Bench::new(1.0);
            b.assets.lang = lang;
            let mut p = Pendulum::default();
            b.core.settings.radios = [1, 0, 0, 0];
            b.core.settings.numbers[1] = 2;
            b.core.settings.set_label(0, b"1");
            b.start(&mut p);
            assert!((p.period - 2.006).abs() < 1e-3, "{}", p.period);
        }
    }

    #[test]
    fn metric_cannon_lands_in_meters_and_km_h() {
        let mut b = Bench::new(1.0);
        b.assets.lang = Lang::Fr;
        let mut c = PhysicsCanon::default();
        b.core.settings.radios = [0, 3, 0, 0];
        b.core.settings.numbers = [3, 2, 5, 0]; // 145 km/h, Earth, 1.83 m
        b.start(&mut c);
        let fr = Physics::of(Lang::Fr);
        let tf = flight_time(fr.heights[5], 9.81);
        assert!(close(c.flight, tf));
        b.tick_at(&mut c, 1);
        let mut t = 1;
        while c.armed {
            t += 1;
            b.tick_at(&mut c, t);
        }
        let (v, vy) = (fr.cannon_speed[3], fr.gravity[2] * tf);
        assert!(close(b.core.number, (vy * vy + v * v).sqrt() * 3.6));
    }

    /// A number as `lang` writes it, from its first digit: thousands
    /// grouped with "," in English, with spaces in Swedish and French,
    /// which write a decimal comma. With the decimals it was given to.
    fn local_number(s: &str, lang: Lang) -> (f64, i32) {
        let (group, point) = if lang == Lang::En { (',', '.') } else { (' ', ',') };
        let start = s.find(|c: char| c.is_ascii_digit() || c == point).unwrap();
        let mut digits = String::new();
        let mut decimals = None;
        for c in s[start..].chars() {
            if c.is_ascii_digit() {
                digits.push(c);
                decimals = decimals.map(|d| d + 1);
            } else if c == point && decimals.is_none() {
                digits.push('.');
                decimals = Some(0);
            } else if c != group && c != '\u{a0}' {
                break;
            }
        }
        (digits.parse().unwrap(), decimals.unwrap_or(0))
    }

    #[test]
    fn reads_numbers_as_each_language_writes_them() {
        assert_eq!(local_number("29,029 ft.(Mount Everest)", Lang::En), (29_029.0, 0));
        assert_eq!(local_number("Moon (.17g)", Lang::En), (0.17, 2));
        assert_eq!(local_number("8 848 m (Mount Everest)", Lang::Sv), (8848.0, 0));
        assert_eq!(local_number("1,83 m (grande personne)", Lang::Fr), (1.83, 2));
        assert_eq!(local_number("(39 889 km/h)", Lang::Fr), (39_889.0, 0));
    }

    /// The presets are read by their place in the list, not their text
    /// (the original never asks a list for its text): each pack's
    /// numbers in the labels must say what the table at that place holds.
    #[test]
    fn the_packs_presets_say_what_the_tables_hold() {
        use crate::assets::Source;
        for lang in Lang::ALL {
            let pack = crate::opendata::builtin_for(lang);
            let combo = |dialog: u32, id: u16| -> Vec<String> {
                let d = pack.read(&format!("WIDGET/1022/{dialog}.bin")).unwrap();
                formats::tables::strings(&d)
                    .into_iter()
                    .filter(|(c, _)| *c == id)
                    .map(|(_, t)| t)
                    .collect()
            };
            let near = |label: &str, value: f64| {
                let (n, decimals) = local_number(label, lang);
                assert!(
                    (n - value).abs() <= 0.5 * 10f64.powi(-decimals) + 1e-9,
                    "{lang:?}: {label:?} for {value}"
                );
            };
            let p = Physics::of(lang);
            for (dialog, id) in [(1000, 107), (1001, 109), (1002, 111)] {
                let labels = combo(dialog, id);
                assert_eq!(labels.len(), 6);
                for (label, h) in labels.iter().zip(p.heights) {
                    near(label, h);
                }
            }
            let speeds = combo(1002, 107);
            assert_eq!(speeds.len(), 5);
            for (label, v) in speeds.iter().zip(p.cannon_speed).take(4) {
                let inside = &label[label.find('(').unwrap()..];
                near(inside, v * p.speed_out);
            }
        }
    }
}
