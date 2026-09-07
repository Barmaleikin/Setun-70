//! Processor registers of Setun-70 (description §4 formal parameters, §5.1).
//!
//! The description numbers trits starting from 1; here everything is 0-based.
//! Field convention (§3.2): in a field `trits[from..=to]` the MOST
//! significant trit is `from`; the value is the balanced sum
//! `Σ trits[from+i] · 3^(n−1−i)`.

use analemma::trit::Trit;
use analemma::tryte::Tryte;
use analemma::word::Word;

/// Fixed-width trit register with MSB-first field accessors.
#[derive(Clone, Copy, Debug)]
pub struct TritWord<const N: usize> {
    pub trits: [Trit; N],
}

impl<const N: usize> Default for TritWord<N> {
    fn default() -> Self {
        Self::zero()
    }
}

impl<const N: usize> TritWord<N> {
    pub fn zero() -> Self {
        TritWord { trits: [Trit::Z; N] }
    }

    /// Balanced value of the inclusive field `trits[from..=to]`,
    /// most significant trit at `from`.
    pub fn field_i32(&self, from: usize, to: usize) -> i32 {
        let mut v: i32 = 0;
        for i in from..=to {
            v = v * 3 + self.trits[i].to_i8() as i32;
        }
        v
    }

    /// Writes a balanced value into the inclusive field `trits[from..=to]`,
    /// most significant trit at `from`. Values outside the field wrap,
    /// mirroring machine behaviour.
    pub fn set_field(&mut self, from: usize, to: usize, value: i32) {
        let mut v = value;
        for i in (from..=to).rev() {
            let rem = v.rem_euclid(3) as i8;
            let t = if rem == 2 { -1 } else { rem };
            self.trits[i] = Trit::from_i8(t).unwrap();
            v = (v - t as i32) / 3;
        }
    }

    pub fn trit(&self, i: usize) -> Trit {
        self.trits[i]
    }

    pub fn set_trit(&mut self, i: usize, t: Trit) {
        self.trits[i] = t;
    }
}

/// The group of an operational syllable, selected by k3 (§2.4, §5.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpGroup {
    /// k3 = −1: second-level operation, dispatch to a subroutine (macro).
    Macro,
    /// k3 = 0: first-level, hardware, any mode.
    Basic,
    /// k3 = +1: first-level, privileged (forbidden in user mode).
    Spec,
}

impl OpGroup {
    pub fn from_trit(t: Trit) -> Self {
        match t {
            Trit::N => OpGroup::Macro,
            Trit::Z => OpGroup::Basic,
            Trit::P => OpGroup::Spec,
        }
    }
}

/// Processor register file.
///
/// Sizes and layouts per §5.1; macro save areas of `c` (trits 8..32) and
/// of `p` (trits 5..10) are included for MACRO/IR (§4).
#[derive(Clone, Debug)]
pub struct Registers {
    /// Modes and the executing-syllable pointer:
    /// `c[0]` = c1 (mode), `c[1..4]` = ch (program page, 3 trits),
    /// `c[4..8]` = ca (syllable address, 4 trits);
    /// `c[8..20]`, `c[20..32]` = macro/interrupt save areas.
    pub c: TritWord<32>,
    /// Stack pointer: `p[0..2]` = ph (stack page, 2 trits → RAM by type),
    /// `p[2..5]` = pa (window position, 3 trits, −13..=13);
    /// `p[5..10]` = save area.
    pub p: TritWord<10>,
    /// R — multiplier register (18 trits, analemma Word).
    pub r: Word,
    /// Y — lower product bits register (18 trits, analemma Word).
    pub y: Word,
    /// v[1..9] — interrupt requests: `v[i] = 1` means a pending request.
    pub v: TritWord<9>,
    /// e — small accumulator (6 trits, analemma Tryte).
    pub e: Tryte,
    /// k — the executing syllable (6 trits):
    /// `k[0]` = k1 (operand length, address syllable),
    /// `k[1]` = k2 (operand-page register selector h[k2]),
    /// `k[2]` = k3 (op group / senior address trit),
    /// `k[2..6]` = ka (operand address, 4 trits),
    /// `k[3..6]` = ko (operation code, 3 trits).
    pub k: TritWord<6>,
    /// w — interrupt cause (4 trits, values −40..=−28 per §5.2).
    pub w: TritWord<4>,
    /// hf — exchange page pointer (3 trits).
    pub hf: TritWord<3>,
    /// h[−1..1] — open operand page registers (3 trits each, page numbers).
    pub h: [TritWord<3>; 3],
    /// q[−1..1] — level-2 exchange page numbers (8 trits each).
    pub q: [TritWord<8>; 3],
    /// g[−1..1] — I/O group buffers (7 trits each).
    pub g: [TritWord<7>; 3],
    /// u[−1..1] — I/O group mode selectors (4 trits each).
    pub u: [TritWord<4>; 3],
    /// a[−1..1] — I/O group sync triggers.
    pub a: [Trit; 3],
    /// Machine state trigger: P = running, Z = stopped (FINISH).
    pub start: Trit,
}

/// Machine mode c1 (§2.6, §5.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// c1 = 1: interrupt mode — all ops allowed, interrupts blocked.
    Interrupt,
    /// c1 = 0: macro mode.
    Macro,
    /// c1 = −1: user mode — spec ops forbidden.
    User,
}

impl Mode {
    pub fn from_trit(t: Trit) -> Self {
        match t {
            Trit::P => Mode::Interrupt,
            Trit::Z => Mode::Macro,
            Trit::N => Mode::User,
        }
    }
    pub fn to_trit(self) -> Trit {
        match self {
            Mode::Interrupt => Trit::P,
            Mode::Macro => Trit::Z,
            Mode::User => Trit::N,
        }
    }
}

impl Registers {
    pub fn zeroed() -> Self {
        Registers {
            c: TritWord::zero(),
            p: TritWord::zero(),
            r: Word::zero(),
            y: Word::zero(),
            v: TritWord::zero(),
            e: Tryte::zero(),
            k: TritWord::zero(),
            w: TritWord::zero(),
            hf: TritWord::zero(),
            h: [TritWord::zero(); 3],
            q: [TritWord::zero(); 3],
            g: [TritWord::zero(); 3],
            u: [TritWord::zero(); 3],
            a: [Trit::Z; 3],
            start: Trit::Z,
        }
    }

    // --- c: mode and program pointer ---

    pub fn mode(&self) -> Mode {
        Mode::from_trit(self.c.trits[0])
    }
    pub fn set_mode(&mut self, m: Mode) {
        self.c.trits[0] = m.to_trit();
    }
    /// ch — page of the executing program (3 trits, −13..=13).
    pub fn ch(&self) -> i8 {
        self.c.field_i32(1, 3) as i8
    }
    pub fn set_ch(&mut self, v: i8) {
        self.c.set_field(1, 3, v as i32);
    }
    /// ca — address of the executing syllable (4 trits, −40..=40).
    pub fn ca(&self) -> i8 {
        self.c.field_i32(4, 7) as i8
    }
    pub fn set_ca(&mut self, v: i8) {
        self.c.set_field(4, 7, v as i32);
    }

    // --- p: stack pointer ---

    /// ph — RAM page occupied by the stack (2 trits, −4..=4 → RAM by type).
    pub fn ph(&self) -> i8 {
        self.p.field_i32(0, 1) as i8
    }
    pub fn set_ph(&mut self, v: i8) {
        self.p.set_field(0, 1, v as i32);
    }
    /// pa — window position within the stack page (3 trits, −13..=13;
    /// 27 positions of 3 syllables per page).
    pub fn pa(&self) -> i8 {
        self.p.field_i32(2, 4) as i8
    }
    pub fn set_pa(&mut self, v: i8) {
        self.p.set_field(2, 4, v as i32);
    }

    // --- k: the executing syllable ---

    pub fn set_k(&mut self, syllable: &Tryte) {
        self.k.trits = syllable.trits;
    }

    /// k1 — operand length in syllables (address syllables only).
    pub fn k1(&self) -> Trit {
        self.k.trits[0]
    }
    /// k2 — selector of the operand-page register h[k2].
    pub fn k2(&self) -> Trit {
        self.k.trits[1]
    }
    /// k3 — op group selector (operational syllable) / senior address trit.
    pub fn k3(&self) -> Trit {
        self.k.trits[2]
    }
    /// ka — operand address, senior syllable of the data (4 trits, −40..=40).
    pub fn ka(&self) -> i8 {
        self.k.field_i32(2, 5) as i8
    }
    /// ko — operation code (3 trits, −13..=13).
    pub fn ko(&self) -> i8 {
        self.k.field_i32(3, 5) as i8
    }
    /// Alias of `ko` in the terms of the description.
    pub fn op_code(&self) -> i8 {
        self.ko()
    }

    /// Operational syllable test: k1 = 0 and k2 = 0 (§4, CYCLE).
    pub fn is_op_syllable(&self) -> bool {
        self.k1() == Trit::Z && self.k2() == Trit::Z
    }

    pub fn op_group(&self) -> OpGroup {
        OpGroup::from_trit(self.k3())
    }

    // --- h: open operand page registers ---

    /// Records the interrupt cause in w (4 trits, §5.2).
    pub fn set_w(&mut self, cause: InterruptCause) {
        self.w.set_field(0, 3, cause as i32);
    }
    /// The recorded interrupt cause.
    pub fn w_value(&self) -> i8 {
        self.w.field_i32(0, 3) as i8
    }

    /// h[k2] — the operand page selected by the k2 trit of the syllable.
    pub fn h(&self, k2: Trit) -> i8 {
        self.h[(k2.to_i8() + 1) as usize].field_i32(0, 2) as i8
    }
    pub fn set_h(&mut self, k2: Trit, page: i8) {
        self.h[(k2.to_i8() + 1) as usize].set_field(0, 2, page as i32);
    }
}

/// Interrupt causes w (§5.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InterruptCause {
    Clock = -40,
    IoGroupMinus1 = -39,
    IoGroup0 = -38,
    IoGroupPlus1 = -37,
    Level2Minus1 = -36,
    Level2Zero = -35,
    Level2Plus1 = -34,
    PanelButton = -33,
    PageExhausted = -32,  // ca overflow, v[9]
    StackExhausted = -31, // pa = -13 on pop
    StackOverflow = -30,  // pa = 13 on push
    OperandCrossPage = -29, // ka + k1 overflow
    ForbiddenSyllable = -28,
}

impl InterruptCause {
    /// Maps a polled `v`-request value (−40..=−32, §5.2) to its cause.
    /// Causes −31..=−28 are raised directly by operations, not polled.
    pub fn from_value(v: i8) -> Option<InterruptCause> {
        use InterruptCause::*;
        Some(match v {
            -40 => Clock,
            -39 => IoGroupMinus1,
            -38 => IoGroup0,
            -37 => IoGroupPlus1,
            -36 => Level2Minus1,
            -35 => Level2Zero,
            -34 => Level2Plus1,
            -33 => PanelButton,
            -32 => PageExhausted,
            _ => return None,
        })
    }
}
