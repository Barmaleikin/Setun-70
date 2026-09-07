//! Machine state and the execution cycle of Setun-70.
//!
//! §4 conventions: T, S and t are aliases of stack-page cells addressed
//! through p (§5.1):
//!   T — the window itself:      m[ph, 3pa−1 : 3pa+1]   (3 syllables = 18 trits)
//!   S — the cell behind it:     m[ph, 3(pa−1)−1 : 3(pa−1)+1]
//!   t — senior syllable of T:   m[ph, 3pa−1]
//! Per §3.2 the last index of a range is least significant: the window value
//! places syllable 3pa−1 in the senior (high) tryte and syllable 3pa+1 in
//! the junior (low) tryte, while in address space the junior syllable sits
//! at the higher address.
//!
//! Implemented: CYCLE (fetch, dispatch, interrupt polling of `v`), REFSYL
//! push, the BASIC group B1..B27, the MACRO dispatcher, and the IR interrupt
//! entry (page 13 vector table, causes §5.2). SPEC ops and level-2 exchange
//! are explicit `Trap::Unsupported` stubs.
//!
//! OCR caveats (flagged, to verify against programming manuals): operand
//! length = k1 + 2 (N→1, Z→2, P→3); B1/B2/B3 reconstructions as coded;
//! MACRO entry "ch := ca := −13" read literally (trampoline on page −13,
//! macro address in ka).

use analemma::dword::DWord;
use analemma::trit::Trit;
use analemma::tryte::Tryte;
use analemma::word::Word;
use crate::io::IoBus;
use crate::memory::{MemError, Memory1, Memory2};
use crate::registers::{InterruptCause, Mode, OpGroup, Registers, TritWord};

/// Why the processor left the normal flow (§4 labels IR / FINISH, §5.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trap {
    /// FINISH: start := 0.
    Finish,
    /// The description requires a layer not implemented yet.
    Unsupported(&'static str),
}

/// Internal control transfer: enter IR, or surface a trap to the caller.
enum Stop {
    Ir(InterruptCause),
    Trap(Trap),
}

/// Why `run` stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunOutcome {
    /// start became 0 (FINISH).
    Finished,
    /// The step budget ran out (e.g. an idle loop).
    StepLimitExceeded,
    /// An unimplemented layer was hit.
    Trap(Trap),
}

pub struct Machine {
    pub mem: Memory1,
    pub mem2: Memory2,
    pub regs: Registers,
    pub io: IoBus,
}

impl Machine {
    /// Inert machine: memory zeroed, start = Z.
    pub fn new() -> Self {
        Machine { mem: Memory1::zeroed(), mem2: Memory2::zeroed(), regs: Registers::zeroed(), io: IoBus::default() }
    }

    /// Power-on sequence — the START block of §4:
    /// start := 1, I/O triggers cleared, mode := interrupt, ch := 12,
    /// ca := −40, stack page ph := 0, window pa := 0, Y := 0.
    pub fn power_on(&mut self) {
        self.regs.start = Trit::P;
        self.regs.a = [Trit::Z; 3];
        self.regs.set_mode(Mode::Interrupt);
        self.regs.set_ch(12);
        self.regs.set_ca(-40);
        self.regs.set_ph(0);
        self.regs.set_pa(0);
        self.regs.y = Word::zero();
    }

    pub fn is_running(&self) -> bool {
        self.regs.start == Trit::P
    }

    // ------------------------------------------------------------------
    // Stack window: T / S / t aliases
    // ------------------------------------------------------------------

    fn read_word_at(&self, page: i8, senior: i8) -> Option<Word> {
        let hi = self.mem.read(page, senior)?;
        let mid = self.mem.read(page, senior + 1)?;
        let lo = self.mem.read(page, senior + 2)?;
        Some(Word::from_trytes(&lo, &mid, &hi))
    }

    fn write_word_at(&mut self, page: i8, senior: i8, w: &Word) -> Result<(), MemError> {
        let (lo, mid, hi) = w.to_trytes();
        self.mem.write(page, senior + 2, lo)?;
        self.mem.write(page, senior + 1, mid)?;
        self.mem.write(page, senior, hi)
    }

    fn window_senior(&self) -> i8 {
        3 * self.regs.pa() - 1
    }

    fn behind_senior(&self) -> i8 {
        3 * (self.regs.pa() - 1) - 1
    }

    /// T — the stack window (top 18 trits of the stack).
    pub fn stack_t(&self) -> Option<Word> {
        self.read_word_at(self.regs.ph(), self.window_senior())
    }

    pub fn set_stack_t(&mut self, w: &Word) -> Result<(), MemError> {
        let s = self.window_senior();
        self.write_word_at(self.regs.ph(), s, w)
    }

    /// S — the stack cell directly behind the window.
    pub fn stack_s(&self) -> Option<Word> {
        self.read_word_at(self.regs.ph(), self.behind_senior())
    }

    pub fn set_stack_s(&mut self, w: &Word) -> Result<(), MemError> {
        let s = self.behind_senior();
        self.write_word_at(self.regs.ph(), s, w)
    }

    /// t — the senior syllable of the window, m[ph, 3pa−1].
    pub fn stack_t_senior(&self) -> Option<Tryte> {
        self.mem.read(self.regs.ph(), self.window_senior())
    }

    pub fn set_stack_t_senior(&mut self, v: Tryte) -> Result<(), MemError> {
        self.mem.write(self.regs.ph(), self.window_senior(), v)
    }

    /// Sign of a balanced word via the library's `top_nonzero`
    /// (stack data is left-aligned and not normalised, so trits[17]
    /// alone is not a sign test).
    fn sign(w: &Word) -> Trit {
        let i = w.top_nonzero();
        if i < 0 { Trit::Z } else { w.trits[i as usize] }
    }

    /// Balanced value of an inclusive MSB-first trit field of a word.
    fn word_field(w: &Word, from: usize, to: usize) -> i64 {
        let mut v: i64 = 0;
        for i in from..=to {
            v = v * 3 + w.trits[i].to_i8() as i64;
        }
        v
    }

    fn need_t(&self) -> Result<Word, Stop> {
        self.stack_t().ok_or(Stop::Ir(InterruptCause::StackOverflow))
    }
    fn need_s(&self) -> Result<Word, Stop> {
        self.stack_s().ok_or(Stop::Ir(InterruptCause::StackExhausted))
    }
    fn put_t(&mut self, w: &Word) -> Result<(), Stop> {
        self.set_stack_t(w).map_err(|_| Stop::Ir(InterruptCause::StackOverflow))
    }
    fn put_s(&mut self, w: &Word) -> Result<(), Stop> {
        self.set_stack_s(w).map_err(|_| Stop::Ir(InterruptCause::StackExhausted))
    }

    // ------------------------------------------------------------------
    // Instruction fetch and cycle
    // ------------------------------------------------------------------

    pub fn fetch_syllable(&mut self) -> Option<()> {
        let s = self.mem.read(self.regs.ch(), self.regs.ca())?;
        self.regs.set_k(&s);
        Some(())
    }

    /// INCRC (§4): `ca := ca + 1`; in user/macro mode, ca = 40 first raises
    /// the page-exhausted request v[9].
    fn inc_rc(&mut self) {
        if self.regs.mode() != Mode::Interrupt && self.regs.ca() == 40 {
            self.regs.v.set_trit(8, Trit::P);
        }
        let ca = self.regs.ca();
        self.regs.set_ca(ca + 1);
    }

    /// Operand length encoded by the k1 trit (decoding assumption, see
    /// module docs): len = k1 + 2.
    fn operand_len(k1: Trit) -> i8 {
        k1.to_i8() + 2
    }

    /// REFSYL (§4): T := 0, then copy `len` syllables whose senior sits at
    /// ka on page h[k2] into the senior part of the window; INCRC.
    fn ref_syllable(&mut self) -> Result<(), Stop> {
        let len = Self::operand_len(self.regs.k1());
        if !(1..=3).contains(&len) {
            return Err(Stop::Ir(InterruptCause::ForbiddenSyllable));
        }
        let ka = self.regs.ka();
        if self.regs.mode() != Mode::Interrupt && (ka as i16 - len as i16) < -40 {
            return Err(Stop::Ir(InterruptCause::OperandCrossPage));
        }
        let page = self.regs.h(self.regs.k2());
        let data = self.mem.read_syllables(page, ka, len as u8)
            .ok_or(Stop::Ir(InterruptCause::OperandCrossPage))?;
        self.put_t(&Word::zero())?;
        let senior = self.window_senior();
        for i in 0..len as i8 {
            self.mem.write(self.regs.ph(), senior + i, data[i as usize])
                .map_err(|_| Stop::Ir(InterruptCause::StackOverflow))?;
        }
        self.inc_rc();
        Ok(())
    }

    /// Rotates the c save areas — the shared prologue of MACRO and IR (§4):
    /// c[20..32] := c[8..20]; c[8..20] := 0; c[16..20] := c[4..8];
    /// c[10..14] := c[0..4]  (0-based).
    fn rotate_c_save(&mut self) {
        let mut tmp = [Trit::Z; 12];
        tmp.copy_from_slice(&self.regs.c.trits[8..20]);
        for t in &mut self.regs.c.trits[8..20] {
            *t = Trit::Z;
        }
        self.regs.c.trits[20..32].copy_from_slice(&tmp);
        let mut tmp4 = [Trit::Z; 4];
        tmp4.copy_from_slice(&self.regs.c.trits[4..8]); // ca
        self.regs.c.trits[16..20].copy_from_slice(&tmp4);
        let mut tmp4b = [Trit::Z; 4];
        tmp4b.copy_from_slice(&self.regs.c.trits[0..4]); // c1, ch
        self.regs.c.trits[10..14].copy_from_slice(&tmp4b);
    }

    /// IR (§4): enter the interrupt routine for `cause`. The vector is the
    /// syllable m[13, w] — page 13 holds the vector table at fixed
    /// addresses −40..−28. Current c and p states are pushed into the save
    /// areas; mode becomes interrupt (c1 := 1), program continues at
    /// (ch, ca) = (13, −13).
    pub fn enter_interrupt(&mut self, cause: InterruptCause) {
        self.rotate_c_save();
        self.regs.set_mode(Mode::Interrupt);
        self.regs.set_ch(13);
        self.regs.set_ca(-13);
        // Swap current and saved stack pointers: p[0..5] <-> p[5..10].
        let mut saved = [Trit::Z; 5];
        saved.copy_from_slice(&self.regs.p.trits[5..10]);
        let (cur, sav) = self.regs.p.trits.split_at_mut(5);
        sav.copy_from_slice(cur);
        cur.copy_from_slice(&saved);
        self.regs.set_w(cause);
        self.regs.set_pa(self.regs.pa() + 1);
        let _ = self.set_stack_t(&Word::zero());
        if let Some(s) = self.mem.read(13, self.regs.w_value()) {
            let _ = self.set_stack_t_senior(s);
        }
    }

    /// One CYCLE of §4: poll `v`, fetch, dispatch. Conditions that the
    /// description routes to IR enter the interrupt routine inside this
    /// call; FINISH and unsupported layers surface as `Err(Trap::…)`.
    pub fn step(&mut self) -> Result<(), Trap> {
        if !self.is_running() {
            return Err(Trap::Finish);
        }
        // CYCLE: in user/macro mode scan the interrupt requests v[1..9].
        if self.regs.mode() != Mode::Interrupt {
            for j in 0..9usize {
                if self.regs.v.trits[j] == Trit::P {
                    self.regs.v.set_trit(j, Trit::Z);
                    if let Some(cause) = InterruptCause::from_value(j as i8 - 40) {
                        self.enter_interrupt(cause);
                        return Ok(());
                    }
                }
            }
        }
        if self.fetch_syllable().is_none() {
            self.enter_interrupt(InterruptCause::PageExhausted);
            return Ok(());
        }
        let r = if self.regs.is_op_syllable() {
            match self.regs.op_group() {
                OpGroup::Basic => self.exec_basic(),
                OpGroup::Spec => self.exec_spec(),
                OpGroup::Macro => self.exec_macro(),
            }
        } else if self.regs.mode() != Mode::Interrupt && self.regs.pa() == 13 {
            Err(Stop::Ir(InterruptCause::StackOverflow))
        } else {
            self.regs.set_pa(self.regs.pa() + 1);
            self.ref_syllable()
        };
        match r {
            Ok(()) => Ok(()),
            Err(Stop::Ir(cause)) => {
                self.enter_interrupt(cause);
                Ok(())
            }
            Err(Stop::Trap(t)) => Err(t),
        }
    }

    /// MACRO dispatch (§4). In interrupt mode a macro syllable stops the
    /// machine (FINISH, §5.3); in macro mode it is forbidden (→ IR −28);
    /// in user mode the program state is saved, mode becomes macro, and
    /// control passes to the macro entry on page −13 (OCR-literal reading,
    /// see module docs).
    fn exec_macro(&mut self) -> Result<(), Stop> {
        match self.regs.mode() {
            Mode::Interrupt => {
                self.regs.start = Trit::Z;
                Err(Stop::Trap(Trap::Finish))
            }
            Mode::Macro => Err(Stop::Ir(InterruptCause::ForbiddenSyllable)),
            Mode::User => {
                if self.regs.pa() == 13 {
                    return Err(Stop::Ir(InterruptCause::StackOverflow));
                }
                self.regs.set_pa(self.regs.pa() + 1);
                self.rotate_c_save();
                self.regs.set_mode(Mode::Macro);
                self.regs.set_ch(-13);
                self.regs.set_ca(-13);
                let ka = self.regs.ka();
                let _ = self.set_stack_t(&Word::zero());
                if let Some(s) = self.mem.read(-13, ka) {
                    let _ = self.set_stack_t_senior(s);
                }
                Ok(())
            }
        }
    }

    // ------------------------------------------------------------------
    // BASIC group B1..B27 (ko −13..=+13)
    // ------------------------------------------------------------------

    fn exec_basic(&mut self) -> Result<(), Stop> {
        if self.regs.mode() != Mode::Interrupt && self.regs.pa() == -13 {
            return Err(Stop::Ir(InterruptCause::StackExhausted));
        }
        match self.regs.op_code() {
            // B1 "LST": x := (S,Y) · 3^t — long shift by t trits
            // (OCR-reconstructed; see module docs).
            -13 => {
                let s = self.need_s()?;
                let mut x = DWord::new(s, self.regs.y);
                let t = self.stack_t_senior()
                    .ok_or(Stop::Ir(InterruptCause::StackOverflow))?.to_i16() as i32;
                x = if t >= 0 { x.shl(t as usize) } else { x.shr((-t) as usize) };
                self.put_s(&x.hi)?;
                self.regs.y = x.lo;
                self.regs.set_pa(self.regs.pa() - 1);
                self.inc_rc();
                Ok(())
            }
            // B2 "COT": if |S| ≥ 3^17/2 then ca := t else INCRC; pop.
            -12 => {
                // |S| > 3^17/2, i.e. |S| >= 64570082: constant as data.
                let s = self.need_s()?.abs();
                let t = self.stack_t_senior()
                    .ok_or(Stop::Ir(InterruptCause::StackOverflow))?.to_i16();
                let (diff, _) = s.sub(&Word::from_i64(64_570_081));
                let big = Self::sign(&diff) == Trit::P; // |S| > 64570081
                self.regs.set_pa(self.regs.pa() - 1);
                if big {
                    self.regs.set_ca(t as i8);
                } else {
                    self.inc_rc();
                }
                Ok(())
            }
            // B3 "XNN": normalise (T,Y), e := e + i (OCR-reconstructed).
            -11 => {
                let t = self.need_t()?;
                let (x, i) = DWord::new(t, self.regs.y).normalize();
                let (e, _) = self.regs.e.add(&Tryte::from_i16(i as i16));
                self.regs.e = e;
                self.put_t(&x.hi)?;
                self.regs.y = x.lo;
                self.inc_rc();
                Ok(())
            }
            // B4 "E−1"
            -10 => {
                let (e, _) = self.regs.e.sub(&Tryte::from_i16(1));
                self.regs.e = e;
                self.inc_rc();
                Ok(())
            }
            // B5 "E=0"
            -9 => {
                self.regs.e = Tryte::zero();
                self.inc_rc();
                Ok(())
            }
            // B6 "E+1"
            -8 => {
                let (e, _) = self.regs.e.add(&Tryte::from_i16(1));
                self.regs.e = e;
                self.inc_rc();
                Ok(())
            }
            // B7 "T−E": T := T − e·3^12.
            -7 => {
                // T := T − e·3^12 (e aligned to the senior tryte).
                let t = self.need_t()?;
                let aligned = Word::from_trytes(&Tryte::zero(), &Tryte::zero(), &self.regs.e);
                let (r, _) = t.sub(&aligned);
                self.put_t(&r)?;
                self.inc_rc();
                Ok(())
            }
            // B8 "E=T": e := t; pop.
            -6 => {
                let t = self.stack_t_senior()
                    .ok_or(Stop::Ir(InterruptCause::StackOverflow))?;
                self.regs.e.trits = t.trits;
                self.regs.set_pa(self.regs.pa() - 1);
                self.inc_rc();
                Ok(())
            }
            // B9 "T+E": T := T + e·3^12.
            -5 => {
                let t = self.need_t()?;
                let aligned = Word::from_trytes(&Tryte::zero(), &Tryte::zero(), &self.regs.e);
                let (r, _) = t.add(&aligned);
                self.put_t(&r)?;
                self.inc_rc();
                Ok(())
            }
            // B10 "CLT": if S < 0 then ca := t; pop.
            -4 => self.cond_jump(|s| Self::sign(s) == Trit::N),
            // B11 "CET": if S = 0 then ca := t; pop.
            -3 => self.cond_jump(|s| s.trits.iter().all(|&x| x == Trit::Z)),
            // B12 "CGT": if S > 0 then ca := t; pop.
            -2 => self.cond_jump(|s| Self::sign(s) == Trit::P),
            // B13 "T=C": T := 0; t := ca; INCRC.
            -1 => {
                let ca = self.regs.ca();
                self.put_t(&Word::zero())?;
                self.set_stack_t_senior(Tryte::from_i16(ca as i16))
                    .map_err(|_| Stop::Ir(InterruptCause::StackOverflow))?;
                self.inc_rc();
                Ok(())
            }
            // B14 "R=T": R := T; pop.
            0 => {
                self.regs.r = self.need_t()?;
                self.regs.set_pa(self.regs.pa() - 1);
                self.inc_rc();
                Ok(())
            }
            // B15 "C=T": ca := t; pop. (No INCRC: ca is overwritten.)
            1 => {
                let t = self.stack_t_senior()
                    .ok_or(Stop::Ir(InterruptCause::StackOverflow))?;
                self.regs.set_ca(t.to_i16() as i8);
                self.regs.set_pa(self.regs.pa() - 1);
                Ok(())
            }
            // B16 "T=W": k := t; REFSYL.
            2 => {
                let t = self.stack_t_senior()
                    .ok_or(Stop::Ir(InterruptCause::StackOverflow))?;
                self.regs.set_k(&t);
                self.ref_syllable()
            }
            // B17 "YFT": Y := 0; pop.
            3 => {
                self.regs.y = Word::zero();
                self.regs.set_pa(self.regs.pa() - 1);
                self.inc_rc();
                Ok(())
            }
            // B18 "W=S": k := t; store the senior `len` syllables of S into
            // m[h[k2], ka−len+1 ..= ka] (RAM pages only, |page| ≤ 4); pop.
            4 => {
                let t = self.stack_t_senior()
                    .ok_or(Stop::Ir(InterruptCause::StackOverflow))?;
                self.regs.set_k(&t);
                let len = Self::operand_len(self.regs.k1());
                if !(1..=3).contains(&len) {
                    return Err(Stop::Ir(InterruptCause::ForbiddenSyllable));
                }
                let ka = self.regs.ka();
                if self.regs.mode() != Mode::Interrupt && (ka as i16 - len as i16) < -40 {
                    return Err(Stop::Ir(InterruptCause::OperandCrossPage));
                }
                let page = self.regs.h(self.regs.k2());
                if page.abs() <= 4 {
                    let s = self.need_s()?;
                    let (lo, mid, hi) = s.to_trytes();
                    self.mem.write_syllables(page, ka, len as u8, &[hi, mid, lo])
                        .map_err(|_| Stop::Ir(InterruptCause::ForbiddenSyllable))?;
                }
                self.regs.set_pa(self.regs.pa() - 1);
                self.inc_rc();
                Ok(())
            }
            // B19 "SMT": S := SMT(S, T); pop.
            5 => {
                let t = self.need_t()?;
                let mut s = self.need_s()?;
                for i in 0..18 {
                    s.trits[i] = s.trits[i].mul(t.trits[i]);
                }
                self.put_s(&s)?;
                self.regs.set_pa(self.regs.pa() - 1);
                self.inc_rc();
                Ok(())
            }
            // B20 "Y=T": Y := T; pop.
            6 => {
                self.regs.y = self.need_t()?;
                self.regs.set_pa(self.regs.pa() - 1);
                self.inc_rc();
                Ok(())
            }
            // B21 "SAT": S := SAT(S, T); pop.
            7 => {
                let t = self.need_t()?;
                let mut s = self.need_s()?;
                for i in 0..18 {
                    let (a, b) = (s.trits[i], t.trits[i]);
                    s.trits[i] = if a == Trit::Z { b } else if a != b && b != Trit::Z { Trit::Z } else { a };
                }
                self.put_s(&s)?;
                self.regs.set_pa(self.regs.pa() - 1);
                self.inc_rc();
                Ok(())
            }
            // B22 "S−T": S := S − T; pop.
            8 => {
                let t = self.need_t()?;
                let s = self.need_s()?;
                let (r, _) = s.sub(&t);
                self.put_s(&r)?;
                self.regs.set_pa(self.regs.pa() - 1);
                self.inc_rc();
                Ok(())
            }
            // B23 "TDN": T := −T (in place, no pop).
            9 => {
                let t = self.need_t()?;
                self.put_t(&t.neg())?;
                self.inc_rc();
                Ok(())
            }
            // B24 "S+T": S := S + T; pop.
            10 => {
                let t = self.need_t()?;
                let s = self.need_s()?;
                let (r, _) = s.add(&t);
                self.put_s(&r)?;
                self.regs.set_pa(self.regs.pa() - 1);
                self.inc_rc();
                Ok(())
            }
            // B25 "LBT": (S,Y) := S·3^18 + T·R·3^2; pop.
            11 => {
                // (S,Y) := S·3^18 + T·R·3^2
                let s = self.need_s()?;
                let t = self.need_t()?;
                let (lo, hi) = Word::zero().mul_acc(&t, &self.regs.r);
                let (hi, _) = hi.add(&s);
                self.put_s(&hi)?;
                self.regs.y = lo;
                self.regs.set_pa(self.regs.pa() - 1);
                self.inc_rc();
                Ok(())
            }
            // B26 "L*T": R := S; (S,Y) := T·R·3^2; pop.
            12 => {
                // R := S; (S,Y) := T·R·3^2
                let s = self.need_s()?;
                let t = self.need_t()?;
                self.regs.r = s;
                let (lo, hi) = Word::zero().mul_acc(&t, &s);
                self.put_s(&hi)?;
                self.regs.y = lo;
                self.regs.set_pa(self.regs.pa() - 1);
                self.inc_rc();
                Ok(())
            }
            // B27 "LHT": (S,Y) := (S,Y) + T·R·3^2; pop.
            13 => {
                // (S,Y) := (S,Y) + T·R·3^2
                let s = self.need_s()?;
                let t = self.need_t()?;
                let (lo, hi) = self.regs.y.mul_acc(&t, &self.regs.r);
                let (hi, _) = hi.add(&s);
                self.put_s(&hi)?;
                self.regs.y = lo;
                self.regs.set_pa(self.regs.pa() - 1);
                self.inc_rc();
                Ok(())
            }
            _ => Err(Stop::Trap(Trap::Unsupported("basic op outside −13..=13"))),
        }
    }

    fn cond_jump(&mut self, cond: impl Fn(&Word) -> bool) -> Result<(), Stop> {
        let s = self.need_s()?;
        let t = self.stack_t_senior()
            .ok_or(Stop::Ir(InterruptCause::StackOverflow))?;
        self.regs.set_pa(self.regs.pa() - 1);
        if cond(&s) {
            self.regs.set_ca(t.to_i16() as i8);
        } else {
            self.inc_rc();
        }
        Ok(())
    }


    // ------------------------------------------------------------------
    // SPEC group S1..S27 (ko −13..=+13)
    // ------------------------------------------------------------------

    /// Executes the SPEC group. In user mode SPEC ops are forbidden — that
    /// check happens in `step` (§5.3) before this is called. G/u transfer
    /// follows the §6 tape correspondence: tape position i (1-based) maps
    /// to syllable element 7−i; a punch encodes ±1 (sign track g[7]),
    /// no punch encodes 0. The encoding is exact for single-sign syllables
    /// only (§6 notes the ambiguity for mixed signs).
    fn exec_spec(&mut self) -> Result<(), Stop> {
        // §5.3: SPEC ops are forbidden in user mode.
        if self.regs.mode() == Mode::User {
            return Err(Stop::Ir(InterruptCause::ForbiddenSyllable));
        }
        if self.regs.mode() != Mode::Interrupt && self.regs.pa() == -13 {
            return Err(Stop::Ir(InterruptCause::StackExhausted));
        }
        let n = self.regs.op_code() + 14; // S-number 1..=27
        let idx = ((n - 1) % 3) as usize; // l = -1, 0, +1 -> 0, 1, 2
        match n {
            // S1..S3 "COPYG": TRANSIN — g[l] -> the window's senior syllable.
            1 | 2 | 3 => {
                let sign = self.regs.g[idx].trits[6];
                let mut t = [Trit::Z; 6];
                for i in 0..6 {
                    if self.regs.g[idx].trits[i] == Trit::P {
                        t[5 - i] = if sign == Trit::P { Trit::P } else { Trit::N };
                    }
                }
                self.put_t(&Word::zero())?;
                self.set_stack_t_senior(Tryte { trits: t })
                    .map_err(|_| Stop::Ir(InterruptCause::StackOverflow))?;
                self.regs.a[idx] = Trit::P;
                self.inc_rc();
                Ok(())
            }
            // S4..S6 "COPYF": RAM page hf := drum page f[l, q[l]].
            4 | 5 | 6 => {
                let z = self.need_t()?;
                let hf = Self::word_field(&z, 3, 5) as i8;
                if hf.abs() <= 4 {
                    let q = self.regs.q[idx].field_i32(0, 7) as i32;
                    if let Some(src) = self.mem2.read_page(idx as i8 - 1, q) {
                        let page = *src;
                        let _ = self.mem.copy_page_from(hf, &page);
                    }
                }
                self.regs.set_pa(self.regs.pa() - 1);
                self.inc_rc();
                Ok(())
            }
            // S7..S9 "LOADQ": q[l] := z[5:12]; SUIT(l) — the page search is
            // instantaneous in emulation, so the request v[l+6] is raised
            // at once.
            7 | 8 | 9 => {
                let z = self.need_t()?;
                let qv = Self::word_field(&z, 4, 11);
                self.regs.q[idx].set_field(0, 7, qv as i32);
                self.regs.v.set_trit(idx + 4, Trit::P);
                self.regs.set_pa(self.regs.pa() - 1);
                self.inc_rc();
                Ok(())
            }
            // S10 "COPYP": T := (ph, pa) packed into an 18-trit word.
            10 => {
                let mut tw = TritWord::<18>::zero();
                tw.set_field(4, 5, self.regs.ph() as i32);
                tw.set_field(8, 10, self.regs.pa() as i32);
                self.put_t(&Word { trits: tw.trits })?;
                self.inc_rc();
                Ok(())
            }
            // S11 "EXCHP": swap current and saved stack pointers.
            11 => {
                let mut saved = [Trit::Z; 5];
                saved.copy_from_slice(&self.regs.p.trits[5..10]);
                let (cur, sav) = self.regs.p.trits.split_at_mut(5);
                sav.copy_from_slice(cur);
                cur.copy_from_slice(&saved);
                self.inc_rc();
                Ok(())
            }
            // S12 "LOADP": (ph, pa) := T.
            12 => {
                let t = self.need_t()?;
                let ph = Self::word_field(&t, 4, 5) as i8;
                let pa = Self::word_field(&t, 8, 10) as i8;
                self.regs.set_ph(ph);
                self.regs.set_pa(pa);
                self.inc_rc();
                Ok(())
            }
            // S13 "COPYMC": push the saved context c[8..20] onto the stack,
            // shifting the save areas down.
            13 => {
                let mut z = [Trit::Z; 18];
                z[0..12].copy_from_slice(&self.regs.c.trits[8..20]);
                let mut older = [Trit::Z; 12];
                older.copy_from_slice(&self.regs.c.trits[20..32]);
                self.regs.c.trits[8..20].copy_from_slice(&older);
                self.regs.c.trits[20..32].copy_from_slice(&z[0..12]);
                self.put_t(&Word { trits: z })?;
                self.inc_rc();
                Ok(())
            }
            // S14 "RETNMC": restore c[0..8] from the saved context, rotate
            // the save areas back. No INCRC: the restored ca is the return
            // address (the syllable after the macro call).
            14 => {
                let mut z = [Trit::Z; 12];
                z.copy_from_slice(&self.regs.c.trits[8..20]);
                let mut older = [Trit::Z; 12];
                older.copy_from_slice(&self.regs.c.trits[20..32]);
                self.regs.c.trits[8..20].copy_from_slice(&older);
                self.regs.c.trits[20..32].copy_from_slice(&z);
                self.regs.c.trits[0..4].copy_from_slice(&z[2..6]);
                self.regs.c.trits[4..8].copy_from_slice(&z[8..12]);
                Ok(())
            }
            // S15 "LOADMC": pop a context from the stack into c[8..20],
            // shifting the save areas up.
            15 => {
                let t = self.need_t()?;
                let mut cur = [Trit::Z; 12];
                cur.copy_from_slice(&self.regs.c.trits[8..20]);
                self.regs.c.trits[20..32].copy_from_slice(&cur);
                self.regs.c.trits[8..20].copy_from_slice(&t.trits[0..12]);
                self.regs.set_pa(self.regs.pa() - 1);
                self.inc_rc();
                Ok(())
            }
            // S16..S18 "LOADH": h[l] := z[4:6].
            16 | 17 | 18 => {
                let z = self.need_t()?;
                let hv = Self::word_field(&z, 3, 5) as i8;
                self.regs.h[idx].set_field(0, 2, hv as i32);
                self.regs.set_pa(self.regs.pa() - 1);
                self.inc_rc();
                Ok(())
            }
            // S19..S21 "LOADU": u[l] := t; wake the I/O group.
            19 | 20 | 21 => {
                let t = self.stack_t_senior()
                    .ok_or(Stop::Ir(InterruptCause::StackOverflow))?;
                for i in 0..4 {
                    self.regs.u[idx].trits[i] = t.trits[i];
                }
                self.regs.a[idx] = Trit::P;
                self.regs.set_pa(self.regs.pa() - 1);
                self.inc_rc();
                Ok(())
            }
            // S22..S24 "LOADF": drum page f[l, q[l]] := RAM page hf.
            22 | 23 | 24 => {
                let z = self.need_t()?;
                let hf = Self::word_field(&z, 3, 5) as i8;
                let q = self.regs.q[idx].field_i32(0, 7) as i32;
                if let Some(page) = self.mem.page(hf) {
                    let page = *page;
                    self.mem2.write_page(idx as i8 - 1, q, page);
                }
                self.regs.set_pa(self.regs.pa() - 1);
                self.inc_rc();
                Ok(())
            }
            // S25..S27 "LOADG": TRANSOUT — the window's senior syllable -> g[l].
            25 | 26 | 27 => {
                let t = self.stack_t_senior()
                    .ok_or(Stop::Ir(InterruptCause::StackOverflow))?;
                let mut has_neg = false;
                for i in 0..6 {
                    if t.trits[5 - i] == Trit::Z {
                        self.regs.g[idx].trits[i] = Trit::Z;
                    } else {
                        self.regs.g[idx].trits[i] = Trit::P;
                    }
                    if t.trits[5 - i] == Trit::N {
                        has_neg = true;
                    }
                }
                self.regs.g[idx].trits[6] = if has_neg { Trit::Z } else { Trit::P };
                self.regs.a[idx] = Trit::P;
                self.regs.set_pa(self.regs.pa() - 1);
                self.inc_rc();
                Ok(())
            }
            _ => Err(Stop::Trap(Trap::Unsupported("spec op outside −13..=13"))),
        }
    }


    // ------------------------------------------------------------------
    // Peripherals (INOUT process, §4) and the run loop
    // ------------------------------------------------------------------

    /// Services one transfer for every group whose sync trigger a[i] is
    /// set — the emulation of the INOUT process. PASS (u[i,1]=1) sends the
    /// g pattern to the device and raises v[i+3]; TAKE (u[i,1]=−1) fills g
    /// from the device, raising v[i+3] when a byte is available (at tape
    /// end the group keeps waiting, a[i] stays set).
    pub fn service_io(&mut self) {
        for idx in 0..3usize {
            if self.regs.a[idx] != Trit::P {
                continue;
            }
            let dev = self.regs.u[idx].field_i32(2, 3); // u[i, 3:4]
            let dir = self.regs.u[idx].trits[0];        // u[i, 1]
            if dev == 0 || dir == Trit::Z {
                self.regs.a[idx] = Trit::Z; // SWOFF / nothing to do
                continue;
            }
            let done = if dir == Trit::P {
                let mut byte = 0u8;
                for b in 0..6 {
                    if self.regs.g[idx].trits[b] == Trit::P {
                        byte |= 1 << b;
                    }
                }
                if self.regs.g[idx].trits[6] == Trit::P {
                    byte |= 1 << 6;
                }
                match self.io.get(idx as i8 - 1, dev as i8) {
                    Some(d) => {
                        d.borrow_mut().write(byte);
                        true
                    }
                    None => true, // no device attached: drop
                }
            } else {
                match self.io.get(idx as i8 - 1, dev as i8).and_then(|d| d.borrow_mut().read()) {
                    Some(byte) => {
                        for b in 0..6 {
                            self.regs.g[idx].trits[b] =
                                if byte & (1 << b) != 0 { Trit::P } else { Trit::Z };
                        }
                        self.regs.g[idx].trits[6] =
                            if byte & (1 << 6) != 0 { Trit::P } else { Trit::N };
                        true
                    }
                    None => false, // not ready: retry on the next service
                }
            };
            if done {
                self.regs.v.set_trit(idx + 1, Trit::P);
                self.regs.a[idx] = Trit::Z;
            }
        }
    }

    /// Runs the machine with a per-step hook (called after the I/O
    /// service with the machine state and the step number).
    pub fn run_with<F: FnMut(&Machine, u64)>(
        &mut self,
        max_steps: u64,
        mut hook: F,
    ) -> RunOutcome {
        let mut steps = 0u64;
        loop {
            if !self.is_running() {
                return RunOutcome::Finished;
            }
            if steps >= max_steps {
                return RunOutcome::StepLimitExceeded;
            }
            match self.step() {
                Ok(()) => {
                    self.service_io();
                    hook(self, steps);
                    steps += 1;
                }
                Err(Trap::Finish) => return RunOutcome::Finished, // FINISH is normal stop
                Err(t) => return RunOutcome::Trap(t),
            }
        }
    }

    /// Runs the machine: step + I/O service until FINISH, a trap, or the
    /// step budget is exhausted.
    pub fn run(&mut self, max_steps: u64) -> RunOutcome {
        self.run_with(max_steps, |_, _| {})
    }

    // ------------------------------------------------------------------
    // 36-trit double-word helpers
    // ------------------------------------------------------------------

}
