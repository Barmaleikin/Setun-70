use analemma::trit::Trit;
use analemma::tryte::Tryte;
use analemma::word::Word;
use crate::machine::{Machine, Trap};
use crate::memory::{MemError, Memory1, PAGE_COUNT, RAM_MAX, RAM_MIN, ROM_PAGE_COUNT};
use crate::registers::{InterruptCause, Mode, OpGroup, Registers, TritWord};

// ============================================================================
// Level-1 memory
// ============================================================================

#[test]
fn test_page_map() {
    assert_eq!(PAGE_COUNT, 27);
    assert_eq!(ROM_PAGE_COUNT, 18);
    for p in RAM_MIN..=RAM_MAX {
        assert!(Memory1::is_ram(p));
        assert!(!Memory1::is_rom(p));
    }
    for &p in &[-13i8, -5, 5, 13] {
        assert!(Memory1::is_rom(p));
        assert!(!Memory1::is_ram(p));
    }
    assert!(!Memory1::is_ram(14) && !Memory1::is_rom(14));
    assert!(!Memory1::is_ram(-14) && !Memory1::is_rom(-14));
}

#[test]
fn test_ram_read_write() {
    let mut m = Memory1::zeroed();
    let t = Tryte::from_i16(-159);
    m.write(-4, -40, t).unwrap();
    m.write(4, 40, t).unwrap();
    assert_eq!(m.read(-4, -40).unwrap().to_i16(), -159);
    assert_eq!(m.read(4, 40).unwrap().to_i16(), -159);
    assert_eq!(m.read(-4, -39).unwrap().to_i16(), 0);
}

#[test]
fn test_rom_is_read_only() {
    let mut m = Memory1::zeroed();
    assert!(matches!(m.write(5, 0, Tryte::zero()), Err(MemError::ReadOnly { page: 5 })));
    assert!(matches!(m.write(-13, 0, Tryte::zero()), Err(MemError::ReadOnly { page: -13 })));
    assert!(matches!(m.write(0, 41, Tryte::zero()), Err(MemError::BadAddress { .. })));
    assert!(matches!(m.write(0, -41, Tryte::zero()), Err(MemError::BadAddress { .. })));
    assert!(matches!(m.read(13, 40), Some(_)));
    assert!(m.read(14, 0).is_none());
    assert!(m.read(0, -41).is_none());
}

#[test]
fn test_rom_page_image() {
    let mut m = Memory1::zeroed();
    let mut img = crate::memory::Page::zeroed();
    img.syllables[0] = Tryte::from_i16(42); // syllable -40
    m.load_rom_page(13, img).unwrap();
    assert_eq!(m.read(13, -40).unwrap().to_i16(), 42);
    assert!(matches!(m.load_rom_page(0, img), Err(MemError::BadAddress { .. })));
}

#[test]
fn test_syllable_triplet_ordering() {
    let mut m = Memory1::zeroed();
    // Ascending addresses 10, 11, 12; senior syllable at 12.
    m.write(0, 10, Tryte::from_i16(1)).unwrap();
    m.write(0, 11, Tryte::from_i16(2)).unwrap();
    m.write(0, 12, Tryte::from_i16(3)).unwrap();
    let tri = m.read_syllables(0, 12, 3).unwrap();
    assert_eq!(tri[0].to_i16(), 3); // senior first
    assert_eq!(tri[1].to_i16(), 2);
    assert_eq!(tri[2].to_i16(), 1);
    // Write back through the senior-first API and re-read.
    m.write_syllables(0, 12, 3, &[Tryte::from_i16(30), Tryte::from_i16(20), Tryte::from_i16(10)]).unwrap();
    assert_eq!(m.read(0, 10).unwrap().to_i16(), 10);
    assert_eq!(m.read(0, 11).unwrap().to_i16(), 20);
    assert_eq!(m.read(0, 12).unwrap().to_i16(), 30);
    assert!(m.read_syllables(0, -40, 3).is_none()); // junior would fall off the page
}

// ============================================================================
// Register file
// ============================================================================

#[test]
fn test_field_roundtrip() {
    let mut c = TritWord::<32>::zero();
    for (from, to, vals) in [(1usize, 3usize, vec![-13i32, -1, 0, 1, 12, 13]),
                             (4, 7, vec![-40, -13, 0, 13, 40])] {
        for v in vals {
            c.set_field(from, to, v);
            assert_eq!(c.field_i32(from, to), v, "field {}..={} value {}", from, to, v);
        }
    }
    // Wrap beyond the field mirrors machine arithmetic.
    c.set_field(1, 3, 14); // > 13, must wrap within the 3-trit field
    assert!(c.field_i32(1, 3) <= 13);
}

#[test]
fn test_power_on_state() {
    let mut m = Machine::new();
    assert!(!m.is_running());
    m.power_on();
    assert!(m.is_running());
    assert_eq!(m.regs.mode(), Mode::Interrupt); // c1 := 1
    assert_eq!(m.regs.ch(), 12);
    assert_eq!(m.regs.ca(), -40);
    assert_eq!(m.regs.ph(), 0);
    assert_eq!(m.regs.pa(), 0);
    assert_eq!(m.regs.y.to_i64(), 0);
    assert_eq!(m.regs.a, [Trit::Z; 3]);
}

#[test]
fn test_syllable_decode() {
    let mut r = Registers::zeroed();
    // Operational syllable: k1 = 0, k2 = 0, k3 = 0 (Basic), ko = -13 (B1 "LST").
    r.k.set_field(3, 5, -13);
    assert!(r.is_op_syllable());
    assert_eq!(r.op_group(), OpGroup::Basic);
    assert_eq!(r.op_code(), -13);
    // Same trits read as an address field: ka = k[2..6].
    assert_eq!(r.ka(), -13);
    // Spec group, code +13 (S27 "LOADG3").
    let mut r2 = Registers::zeroed();
    r2.k.set_trit(2, Trit::P);
    r2.k.set_field(3, 5, 13);
    assert!(r2.is_op_syllable());
    assert_eq!(r2.op_group(), OpGroup::Spec);
    assert_eq!(r2.op_code(), 13);
    // Address syllable: k1 != 0 or k2 != 0.
    let mut r3 = Registers::zeroed();
    r3.k.set_trit(0, Trit::P); // k1 = 1 syllable
    assert!(!r3.is_op_syllable());
}

#[test]
fn test_stack_pointer_ranges() {
    let mut r = Registers::zeroed();
    for v in [-4i8, 0, 4] {
        r.set_ph(v);
        assert_eq!(r.ph(), v);
    }
    for v in [-13i8, 0, 13] {
        r.set_pa(v);
        assert_eq!(r.pa(), v);
    }
}

#[test]
fn test_interrupt_causes() {
    assert_eq!(InterruptCause::Clock as i8, -40);
    assert_eq!(InterruptCause::ForbiddenSyllable as i8, -28);
    assert_eq!(InterruptCause::PageExhausted as i8, -32);
}

// ============================================================================
// Stack window (T / S / t aliases)
// ============================================================================

#[test]
fn test_window_addresses_at_extremes() {
    let mut m = Machine::new();
    m.power_on();
    m.regs.set_pa(13); // senior = 3*13-1 = 38; syllables 38, 39, 40
    assert!(m.stack_t().is_some());
    m.regs.set_pa(-13); // senior = -40; syllables -40, -39, -38
    assert!(m.stack_t().is_some());
    // S would occupy -43..-37: junior syllable -43 is OFF the page.
    // (The machine intercepts this earlier as interrupt -31; here the
    // memory-level view simply reports None.)
    assert!(m.stack_s().is_none());
}

#[test]
fn test_window_t_roundtrip() {
    let mut m = Machine::new();
    m.power_on();
    m.regs.set_ph(2);
    m.regs.set_pa(5);
    let w = Word::from_i64(123_456);
    m.set_stack_t(&w).unwrap();
    let back = m.stack_t().unwrap();
    assert_eq!(back.to_i64(), 123_456);
    // t is the senior syllable = high tryte of T.
    let (_, _, hi) = w.to_trytes();
    assert_eq!(m.stack_t_senior().unwrap().to_i16(), hi.to_i16());
    // S is the cell behind: initially zero.
    assert_eq!(m.stack_s().unwrap().to_i64(), 0);
    let s = Word::from_i64(-7);
    m.set_stack_s(&s).unwrap();
    assert_eq!(m.stack_s().unwrap().to_i64(), -7);
    // The window lives at m[ph, 3pa-1 : 3pa+1]: check the senior syllable cell.
    assert_eq!(m.mem.read(2, 3 * 5 - 1).unwrap().to_i16(), hi.to_i16());
}

#[test]
fn test_fetch_syllable() {
    let mut m = Machine::new();
    m.power_on();
    // Operational syllable of the Basic group with ko = -13 (B1 "LST"):
    // trits [Z, Z, Z, N, N, N], i.e. the tryte value -13 * 3^3 = -351.
    // (ch is a 3-trit field; page 12 — the power-on page — is ROM, so for
    // the test we point ch at a RAM page.)
    m.mem.write(0, -40, Tryte::from_i16(-351)).unwrap();
    m.regs.set_ch(0);
    m.regs.set_ca(-40);
    m.fetch_syllable().unwrap();
    assert_eq!(m.regs.op_code(), -13);
    assert!(m.regs.is_op_syllable());
    assert_eq!(m.regs.op_group(), OpGroup::Basic);
    // As an address field the same trits read ka = -13.
    assert_eq!(m.regs.ka(), -13);
}


// ============================================================================
// Execution cycle: address syllables (REFSYL push) and the BASIC group
// ============================================================================

/// Address syllable with the k1+2 length encoding (N→1, Z→2, P→3).
fn addr_syllable(k1: Trit, k2: Trit, ka: i8) -> Tryte {
    let mut tw = TritWord::<6>::zero();
    tw.set_trit(0, k1);
    tw.set_trit(1, k2);
    tw.set_field(2, 5, ka as i32);
    Tryte { trits: tw.trits }
}

/// Operational syllable of the given group with the given code.
fn op_syllable(group: Trit, ko: i8) -> Tryte {
    let mut tw = TritWord::<6>::zero();
    tw.set_trit(2, group);
    tw.set_field(3, 5, ko as i32);
    Tryte { trits: tw.trits }
}

/// Writes a right-aligned 18-trit operand (value in the junior syllable)
/// for a len-3 push whose senior syllable sits at `ka`.
fn push3_data(m: &mut Machine, ka: i8, v: i64) {
    let w = Word::from_i64(v);
    let (lo, mid, hi) = w.to_trytes();
    m.mem.write(0, ka - 2, lo).unwrap();
    m.mem.write(0, ka - 1, mid).unwrap();
    m.mem.write(0, ka, hi).unwrap();
}

/// Test bench: program on page 0 (ch=0, ca from -40), data on page 0 via
/// h[Z]=0, stack on page 1 (ph=1), user mode.
fn bench() -> Machine {
    let mut m = Machine::new();
    m.power_on();
    m.regs.set_mode(Mode::User);
    m.regs.set_ch(0);
    m.regs.set_ca(-40);
    m.regs.set_ph(1);
    m.regs.set_h(Trit::Z, 0);
    m
}

#[test]
fn test_push_and_s_plus_t() {
    // Right-aligned 3-syllable operands: AЛУ sees plain integers.
    let mut m = bench();
    push3_data(&mut m, 12, 5);
    push3_data(&mut m, 15, 7);
    m.mem.write(0, -40, addr_syllable(Trit::P, Trit::Z, 12)).unwrap(); // push 5
    m.mem.write(0, -39, addr_syllable(Trit::P, Trit::Z, 15)).unwrap(); // push 7
    m.mem.write(0, -38, op_syllable(Trit::Z, 10)).unwrap();            // S+T
    m.step().unwrap();
    assert_eq!(m.regs.pa(), 1);
    m.step().unwrap();
    assert_eq!(m.regs.pa(), 2);
    m.step().unwrap();
    assert_eq!(m.regs.pa(), 1);            // pop
    assert_eq!(m.stack_t().unwrap().to_i64(), 12);
    assert_eq!(m.regs.ca(), -37);          // INCRC after the op
}

#[test]
fn test_push_len1_zero_fills_junior_trytes() {
    let mut m = bench();
    m.mem.write(0, 10, Tryte::from_i16(5)).unwrap();
    m.mem.write(0, -40, addr_syllable(Trit::N, Trit::Z, 10)).unwrap();
    m.step().unwrap();
    // Window: senior tryte = 5, mid and junior zeroed by T := 0.
    assert_eq!(m.stack_t().unwrap().to_i64(), 5 * 3_i64.pow(12));
}

#[test]
fn test_tdn_negates_window_in_place() {
    let mut m = bench();
    push3_data(&mut m, 12, 5);
    m.mem.write(0, -40, addr_syllable(Trit::P, Trit::Z, 12)).unwrap();
    m.mem.write(0, -39, op_syllable(Trit::Z, 9)).unwrap(); // TDN
    m.step().unwrap();
    m.step().unwrap();
    assert_eq!(m.stack_t().unwrap().to_i64(), -5);
    assert_eq!(m.regs.pa(), 1); // no pop for one-place ops
}

#[test]
fn test_cgt_taken_and_not_taken() {
    // Taken: push 5, push 20 (jump target), CGT -> ca := 20.
    let mut m = bench();
    m.mem.write(0, 10, Tryte::from_i16(5)).unwrap();
    m.mem.write(0, 11, Tryte::from_i16(20)).unwrap();
    m.mem.write(0, -40, addr_syllable(Trit::N, Trit::Z, 10)).unwrap();
    m.mem.write(0, -39, addr_syllable(Trit::N, Trit::Z, 11)).unwrap();
    m.mem.write(0, -38, op_syllable(Trit::Z, -2)).unwrap(); // CGT
    m.step().unwrap();
    m.step().unwrap();
    m.step().unwrap();
    assert_eq!(m.regs.ca(), 20);
    assert_eq!(m.regs.pa(), 1);

    // Not taken: push -5, push 20, CGT -> INCRC only.
    let mut m = bench();
    m.mem.write(0, 10, Tryte::from_i16(-5)).unwrap();
    m.mem.write(0, 11, Tryte::from_i16(20)).unwrap();
    m.mem.write(0, -40, addr_syllable(Trit::N, Trit::Z, 10)).unwrap();
    m.mem.write(0, -39, addr_syllable(Trit::N, Trit::Z, 11)).unwrap();
    m.mem.write(0, -38, op_syllable(Trit::Z, -2)).unwrap();
    m.step().unwrap();
    m.step().unwrap();
    m.step().unwrap();
    assert_eq!(m.regs.ca(), -37);
}

#[test]
fn test_w_eq_s_stores_stack_to_memory() {
    let mut m = bench();
    m.mem.write(0, 10, Tryte::from_i16(5)).unwrap();                    // the value
    m.mem.write(0, 20, addr_syllable(Trit::N, Trit::Z, 30)).unwrap();   // the address syllable
    m.mem.write(0, -40, addr_syllable(Trit::N, Trit::Z, 10)).unwrap();  // push value
    m.mem.write(0, -39, addr_syllable(Trit::N, Trit::Z, 20)).unwrap();  // push address syllable
    m.mem.write(0, -38, op_syllable(Trit::Z, 4)).unwrap();              // W=S
    m.step().unwrap();
    m.step().unwrap();
    m.step().unwrap();
    assert_eq!(m.mem.read(0, 30).unwrap().to_i16(), 5); // stored senior syllable
    assert_eq!(m.regs.pa(), 1);
}

#[test]
fn test_l_mul_b26() {
    // L*T: R := S; (S,Y) := T*R*9. With T=3, S=2: Y := 54, S := 0.
    let mut m = bench();
    push3_data(&mut m, 12, 2);
    push3_data(&mut m, 15, 3);
    m.mem.write(0, -40, addr_syllable(Trit::P, Trit::Z, 12)).unwrap();
    m.mem.write(0, -39, addr_syllable(Trit::P, Trit::Z, 15)).unwrap();
    m.mem.write(0, -38, op_syllable(Trit::Z, 12)).unwrap(); // L*T
    m.step().unwrap();
    m.step().unwrap();
    m.step().unwrap();
    assert_eq!(m.regs.y.to_i64(), 54);
    assert_eq!(m.stack_t().unwrap().to_i64(), 0);
    assert_eq!(m.regs.r.to_i64(), 2);
}

#[test]
fn test_spec_forbidden_in_user_mode() {
    let mut m = bench();
    m.mem.write(0, -40, op_syllable(Trit::P, 1)).unwrap(); // any SPEC op
    m.step().unwrap(); // -> IR entry (w = -28), not an error
    assert_eq!(m.regs.mode(), Mode::Interrupt);
    assert_eq!(m.regs.ch(), 13);
    assert_eq!(m.regs.w_value(), -28);
}

#[test]
fn test_stack_overflow_enters_interrupt() {
    let mut m = bench();
    m.regs.set_pa(13);
    m.mem.write(0, -40, addr_syllable(Trit::N, Trit::Z, 10)).unwrap();
    m.step().unwrap();
    assert_eq!(m.regs.mode(), Mode::Interrupt);
    assert_eq!(m.regs.ch(), 13);
    assert_eq!(m.regs.ca(), -13);
    assert_eq!(m.regs.w_value(), -30); // StackOverflow
    // IR swaps the stack pointer with the saved one: the interrupted
    // pa = 13 goes to the save area, the restored (zeroed) pointer gets
    // pa := pa + 1.
    assert_eq!(m.regs.pa(), 1);
    assert_eq!(m.regs.p.field_i32(7, 9), 13); // saved pa
}

#[test]
fn test_operand_cross_page_enters_interrupt() {
    let mut m = bench();
    m.mem.write(0, -40, addr_syllable(Trit::P, Trit::Z, -40)).unwrap(); // len 3 at -40
    m.step().unwrap();
    assert_eq!(m.regs.w_value(), -29); // OperandCrossPage
    assert_eq!(m.regs.mode(), Mode::Interrupt);
}

#[test]
fn test_interrupt_vector_fetch() {
    // The vector for cause -30 is the syllable m[13, -30] (page 13 table).
    let mut m = bench();
    m.regs.set_pa(13);
    let mut img = crate::memory::Page::zeroed();
    img.syllables[(-30 - -40) as usize] = Tryte::from_i16(77);
    m.mem.load_rom_page(13, img).unwrap();
    m.mem.write(0, -40, addr_syllable(Trit::N, Trit::Z, 10)).unwrap();
    m.step().unwrap();
    assert_eq!(m.regs.w_value(), -30);
    assert_eq!(m.stack_t_senior().unwrap().to_i16(), 77);
}

#[test]
fn test_interrupt_polling_v9_page_exhausted() {
    let mut m = bench();
    m.regs.set_ca(40);
    m.mem.write(0, 10, Tryte::from_i16(5)).unwrap();
    m.mem.write(0, 40, addr_syllable(Trit::N, Trit::Z, 10)).unwrap();
    m.step().unwrap(); // push; INCRC at ca=40 raises v[9], ca wraps to -40
    assert_eq!(m.regs.ca(), -40);
    assert_eq!(m.regs.v.trits[8], Trit::P);
    m.step().unwrap(); // poll: v[9] -> IR entry
    assert_eq!(m.regs.mode(), Mode::Interrupt);
    assert_eq!(m.regs.ch(), 13);
    assert_eq!(m.regs.w_value(), -32); // PageExhausted
    assert_eq!(m.regs.v.trits[8], Trit::Z);
}

#[test]
fn test_macro_dispatch_saves_state() {
    let mut m = bench(); // user mode
    // Macro syllable: k1 = k2 = Z, k3 = N. ka shares its senior trit with
    // k3, so we read the final ka back from the syllable itself.
    let mut tw = TritWord::<6>::zero();
    tw.set_field(2, 5, 5);
    tw.set_trit(2, Trit::N);
    let ka = tw.field_i32(2, 5) as i8;
    let mut img = crate::memory::Page::zeroed();
    img.syllables[(ka - -40) as usize] = Tryte::from_i16(111); // m[-13, ka]
    m.mem.load_rom_page(-13, img).unwrap();
    m.mem.write(0, -40, Tryte { trits: tw.trits }).unwrap();
    m.step().unwrap();
    assert_eq!(m.regs.mode(), Mode::Macro);
    assert_eq!(m.regs.ch(), -13);
    assert_eq!(m.regs.ca(), -13);
    assert_eq!(m.regs.pa(), 1);
    // Saved return state: old ca (-40) at c[16..20], old (c1, ch) at c[10..14].
    assert_eq!(m.regs.c.field_i32(16, 19), -40);
    // Prefetched entry syllable on the zeroed window.
    assert_eq!(m.stack_t_senior().unwrap().to_i16(), 111);
    assert_eq!(m.stack_t().unwrap().to_i64(), 111 * 3_i64.pow(12));
}

#[test]
fn test_macro_in_interrupt_mode_finishes() {
    let mut m = Machine::new();
    m.power_on(); // interrupt mode
    m.regs.set_ch(0);
    m.regs.set_ca(-40);
    m.regs.set_ph(1);
    m.mem.write(0, -40, op_syllable(Trit::N, 0)).unwrap(); // any MACRO op
    assert_eq!(m.step(), Err(Trap::Finish));
    assert!(!m.is_running());
}


// ============================================================================
// SPEC group: stack-pointer ops, macro context, I/O buffers, drum exchange
// ============================================================================

/// Bench in macro mode (SPEC ops allowed).
fn spec_bench() -> Machine {
    let mut m = bench();
    m.regs.set_mode(Mode::Macro);
    m
}

#[test]
fn test_exchp_s11() {
    let mut m = spec_bench();
    m.regs.set_ph(1);
    m.regs.set_pa(2);
    m.regs.p.set_field(5, 6, 3);  // saved ph
    m.regs.p.set_field(7, 9, 5);  // saved pa
    m.mem.write(0, -40, op_syllable(Trit::P, -3)).unwrap(); // EXCHP
    m.step().unwrap();
    assert_eq!(m.regs.ph(), 3);
    assert_eq!(m.regs.pa(), 5);
    assert_eq!(m.regs.p.field_i32(5, 6), 1); // old current went to the save area
    assert_eq!(m.regs.p.field_i32(7, 9), 2);
    assert_eq!(m.regs.ca(), -39); // INCRC, no pop
}

#[test]
fn test_copyp_loadp_s10_s12() {
    let mut m = spec_bench();
    m.regs.set_ph(1);
    m.regs.set_pa(2);
    m.mem.write(0, -40, op_syllable(Trit::P, -4)).unwrap(); // COPYP
    m.step().unwrap();
    let w = m.stack_t().unwrap();
    let tw = TritWord::<18> { trits: w.trits };
    assert_eq!(tw.field_i32(4, 5), 1);  // ph
    assert_eq!(tw.field_i32(8, 10), 2); // pa
    // Corrupt the pointer, then restore it from the packed window.
    // (Changing ph/pa relocates the window, so the packed word must be
    // placed at the new window before LOADP — as real programs do by
    // keeping the pointer on the stack, not in p.)
    m.regs.set_ph(3);
    m.regs.set_pa(4);
    m.set_stack_t(&w).unwrap();
    m.mem.write(0, -39, op_syllable(Trit::P, -2)).unwrap(); // LOADP
    m.step().unwrap();
    assert_eq!(m.regs.ph(), 1);
    assert_eq!(m.regs.pa(), 2);
}

#[test]
fn test_copymc_retmc_s13_s14() {
    let mut m = spec_bench();
    // Simulate a saved caller context: user mode, ch = 0, ca = -30.
    m.regs.c.set_trit(10, Trit::N);
    m.regs.c.set_field(11, 13, 0);
    m.regs.c.set_field(16, 19, -30);
    m.mem.write(0, -40, op_syllable(Trit::P, -1)).unwrap(); // COPYMC
    m.step().unwrap();
    let w = m.stack_t().unwrap();
    assert_eq!(w.trits[10 - 8], Trit::N); // saved c1 copied to the window
    // LOADMC pops it back into the save slot.
    m.mem.write(0, -39, op_syllable(Trit::P, 1)).unwrap(); // LOADMC
    m.step().unwrap();
    assert_eq!(m.regs.c.trits[10], Trit::N);
    assert_eq!(m.regs.c.field_i32(16, 19), -30);
    // RETNMC restores the caller's c1/ch/ca. No INCRC.
    // (ca already points at -38 after LOADMC's INCRC.)
    m.mem.write(0, -38, op_syllable(Trit::P, 0)).unwrap(); // RETNMC
    m.step().unwrap();
    assert_eq!(m.regs.mode(), Mode::User);
    assert_eq!(m.regs.ch(), 0);
    assert_eq!(m.regs.ca(), -30);
}

#[test]
fn test_loadh_s16() {
    let mut m = spec_bench();
    push3_data(&mut m, 12, 0);
    // Encode hf = -5 into trits[3..6] of the window word: -5 -> [1,1,-1].
    let mut tw = TritWord::<18>::zero();
    tw.set_field(3, 5, -5);
    m.regs.set_pa(1); // window exists, so the pop lands at 0
    m.set_stack_t(&Word { trits: tw.trits }).unwrap();
    m.mem.write(0, -40, op_syllable(Trit::P, 2)).unwrap(); // LOADH1 (l = -1)
    m.step().unwrap();
    assert_eq!(m.regs.h(Trit::N), -5);
    assert_eq!(m.regs.pa(), 0); // popped
}

#[test]
fn test_loadu_s19() {
    let mut m = spec_bench();
    m.regs.set_pa(1);
    m.set_stack_t_senior(Tryte::from_i16(42)).unwrap();
    m.mem.write(0, -40, op_syllable(Trit::P, 5)).unwrap(); // LOADU1
    m.step().unwrap();
    // Positional copy of the senior syllable's low 4 trits (u is 4 trits).
    let t = Tryte::from_i16(42);
    assert_eq!(m.regs.u[0].trits, [t.trits[0], t.trits[1], t.trits[2], t.trits[3]]);
    assert_eq!(m.regs.a[0], Trit::P);
    assert_eq!(m.regs.pa(), 0);
}

#[test]
fn test_transout_transin_roundtrip_s25_s1() {
    let mut m = spec_bench();
    m.regs.set_pa(1);
    // -13 is a single-sign syllable ([N,N,N,Z,Z,Z]): exact roundtrip.
    m.set_stack_t_senior(Tryte::from_i16(-13)).unwrap();
    m.mem.write(0, -40, op_syllable(Trit::P, 11)).unwrap(); // LOADG1
    m.step().unwrap();
    assert_eq!(m.regs.g[0].trits[6], Trit::Z); // no 7th-track punch: negatives present
    assert_eq!(m.regs.g[0].trits[0], Trit::Z); // tape pos 1 <-> syllable element 6 (zero)
    assert_eq!(m.regs.g[0].trits[5], Trit::P); // tape pos 6 <-> element 1 (-13)
    // Now read the pattern back.
    m.regs.set_pa(1);
    m.mem.write(0, -39, op_syllable(Trit::P, -13)).unwrap(); // COPYG1
    m.step().unwrap();
    assert_eq!(m.stack_t_senior().unwrap().to_i16(), -13);
}

#[test]
fn test_loadf_copyf_drum_s22_s4() {
    let mut m = spec_bench();
    m.regs.q[1].set_field(0, 7, 7); // q[0] = 7 (l = 0)
    m.mem.write(2, -40, Tryte::from_i16(99)).unwrap(); // RAM page 2
    let mut tw = TritWord::<18>::zero();
    tw.set_field(3, 5, 2); // hf = 2
    m.regs.set_pa(1);
    m.set_stack_t(&Word { trits: tw.trits }).unwrap();
    m.mem.write(0, -40, op_syllable(Trit::P, 8)).unwrap(); // LOADF2
    m.step().unwrap();
    // Change RAM, then pull the page back from the drum.
    m.mem.write(2, -40, Tryte::from_i16(1)).unwrap();
    m.regs.set_pa(1);
    m.set_stack_t(&Word { trits: tw.trits }).unwrap();
    m.mem.write(0, -39, op_syllable(Trit::P, -10)).unwrap(); // COPYF2
    m.step().unwrap();
    assert_eq!(m.mem.read(2, -40).unwrap().to_i16(), 99);
}

#[test]
fn test_loadq_suit_s7() {
    let mut m = spec_bench();
    m.regs.set_pa(1);
    let mut tw = TritWord::<18>::zero();
    tw.set_field(4, 11, 123); // q value in trits[4..12]
    m.set_stack_t(&Word { trits: tw.trits }).unwrap();
    m.mem.write(0, -40, op_syllable(Trit::P, -7)).unwrap(); // LOADQ1
    m.step().unwrap();
    assert_eq!(m.regs.q[0].field_i32(0, 7), 123);
    assert_eq!(m.regs.v.trits[4], Trit::P); // SUIT raised v[5] at once
    assert_eq!(m.regs.pa(), 0);
}


// ============================================================================
// Peripherals (io) and the run loop
// ============================================================================

use crate::io::{row_to_syllable, syllable_to_row, TapeReader};
use crate::machine::RunOutcome;

#[test]
fn test_syllable_row_roundtrip() {
    use analemma::trit::Trit as Tr;
    for v in [-364i16, -40, -13, -12, -10, -9, -5, -4, -3, -1, 0,
              1, 2, 3, 4, 5, 9, 10, 12, 13, 40, 41, 364] {
        let t = Tryte::from_i16(v);
        let back = row_to_syllable(syllable_to_row(&t));
        let nz: Vec<i8> = t.trits.iter().map(|&x| x.to_i8()).filter(|&x| x != 0).collect();
        let expected = if nz.iter().all(|&x| x > 0) || nz.iter().all(|&x| x < 0) || nz.is_empty() {
            // mono-sign syllable: §6 roundtrips exactly
            v
        } else {
            // mixed signs: all punches take the sign track value (no 7th
            // punch when negatives exist) -> every nonzero trit becomes -1
            let mut e = 0i16;
            for (i, &x) in t.trits.iter().enumerate() {
                if x != Tr::Z { e -= 3i16.pow(i as u32); }
            }
            e
        };
        assert_eq!(back.to_i16(), expected, "value {}", v);
    }
}

#[test]
fn test_run_end_to_end_program() {
    // POLIZ program: push 6, push 7, S+T, store the sum at m[0, 20],
    // then jump to the push syllable forever (idle loop).
    let mut m = bench();
    m.mem.write(0, 10, Tryte::from_i16(6)).unwrap();
    m.mem.write(0, 11, Tryte::from_i16(7)).unwrap();
    m.mem.write(0, 12, addr_syllable(Trit::N, Trit::Z, 20)).unwrap(); // target syllable
    m.mem.write(0, 13, Tryte::from_i16(-35)).unwrap(); // jump back to the store
    m.mem.write(0, -40, addr_syllable(Trit::N, Trit::Z, 10)).unwrap();
    m.mem.write(0, -39, addr_syllable(Trit::N, Trit::Z, 11)).unwrap();
    m.mem.write(0, -38, op_syllable(Trit::Z, 10)).unwrap(); // S+T
    m.mem.write(0, -37, addr_syllable(Trit::N, Trit::Z, 12)).unwrap();
    m.mem.write(0, -36, op_syllable(Trit::Z, 4)).unwrap();  // W=S
    m.mem.write(0, -35, addr_syllable(Trit::N, Trit::Z, 13)).unwrap();
    m.mem.write(0, -34, op_syllable(Trit::Z, 1)).unwrap();  // C=T
    let outcome = m.run(1000);
    assert_eq!(outcome, RunOutcome::StepLimitExceeded); // idle loop, no faults
    assert_eq!(m.mem.read(0, 20).unwrap().to_i16(), 13); // 6 + 7 stored
}


/// Shared byte sink for inspecting punch output from a test.
#[derive(Clone, Default)]
struct SharedSink(std::rc::Rc<std::cell::RefCell<Vec<u8>>>);

impl crate::io::ByteDevice for SharedSink {
    fn read(&mut self) -> Option<u8> {
        None
    }
    fn write(&mut self, byte: u8) {
        self.0.borrow_mut().push(byte);
    }
}

#[test]
fn test_punch_device_via_loadg() {
    let mut m = spec_bench();
    let sink = SharedSink::default();
    // LOADG1 (ko=+11) serves group l=-1, i.e. array index 0: attach, drive
    // and expect v-requests all on that group.
    m.io.attach(-1, 1, std::rc::Rc::new(std::cell::RefCell::new(sink.clone())));
    m.regs.u[0].set_trit(0, Trit::P); // direction = PASS
    m.regs.u[0].set_field(2, 3, 1);   // device number 1
    m.regs.set_pa(1);
    m.set_stack_t_senior(Tryte::from_i16(-13)).unwrap();
    m.mem.write(0, -40, op_syllable(Trit::P, 11)).unwrap(); // LOADG1
    m.step().unwrap();
    m.service_io();
    assert_eq!(m.regs.v.trits[1], Trit::P); // v[i+3] = v[2] for i = -1
    assert_eq!(*sink.0.borrow(), vec![0b0011_1000]); // -13 as a §6 row
}

#[test]
fn test_io_take_pass_chain() {
    // Reader -> TAKE -> g -> TRANSIN -> syllable -> TRANSOUT -> g'
    // -> PASS -> punch. Syllable -13 roundtrips through the §6 row.
    let mut m = spec_bench();
    // The whole chain runs on group l = -1 (array index 0): LOADG1/COPYG1
    // serve it, so TAKE must fill g[0] and PASS must read g[0].
    m.io.attach(-1, 1, std::rc::Rc::new(std::cell::RefCell::new(TapeReader::new(vec![0b0011_1000]))));
    let sink = SharedSink::default();
    m.io.attach(-1, 2, std::rc::Rc::new(std::cell::RefCell::new(sink.clone())));
    // Take phase.
    m.regs.u[0].set_trit(0, Trit::N);
    m.regs.u[0].set_field(2, 3, 1);
    m.regs.a[0] = Trit::P;
    m.service_io();
    assert_eq!(m.regs.v.trits[1], Trit::P);
    assert_eq!(m.regs.g[0].trits[5], Trit::P); // punch at position 6
    assert_eq!(m.regs.g[0].trits[6], Trit::N); // sign track: N = negative flag
    // The completed TAKE raised v; a real machine would service it via the
    // page-13 handler. Clear it so the next step runs our COPYG1.
    m.regs.v.set_trit(1, Trit::Z);
    // TRANSIN into the window.
    m.regs.set_pa(1);
    m.mem.write(0, -40, op_syllable(Trit::P, -13)).unwrap(); // COPYG1
    m.step().unwrap();
    assert_eq!(m.stack_t_senior().unwrap().to_i16(), -13);
    // Pass phase: TRANSOUT into g, then PASS to device 2.
    // (LOADG1 writes g[l=-1] == index 0, so drive group -1 == index 0.)
    m.regs.set_pa(1);
    m.set_stack_t_senior(Tryte::from_i16(-13)).unwrap();
    m.mem.write(0, -39, op_syllable(Trit::P, 11)).unwrap(); // LOADG1
    m.step().unwrap();
    m.regs.u[0].set_trit(0, Trit::P);
    m.regs.u[0].set_field(2, 3, 2);
    m.regs.a[0] = Trit::P;
    m.service_io();
    assert_eq!(*sink.0.borrow(), vec![0b0011_1000]);
}


// ============================================================================
// Resident firmware (power-on from ROM page 12)
// ============================================================================

#[test]
fn test_power_on_minimal_firmware() {
    // Part 1: the pure power-on path. h registers reset to 0 (RAM, empty),
    // so the first syllables may only use stack ops and T=C constants;
    // a MACRO syllable in interrupt mode stops the machine (§5.3 FINISH).
    let mut m = Machine::new();
    m.power_on(); // ch = 12, ca = -40, interrupt mode
    let s = |a: i8| (a - -40) as usize;
    let mut img = crate::memory::Page::zeroed();
    img.syllables[s(-40)] = op_syllable(Trit::Z, -1); // T=C: window := ca
    img.syllables[s(-39)] = op_syllable(Trit::Z, 9);  // TDN
    img.syllables[s(-38)] = op_syllable(Trit::Z, 9);  // TDN
    img.syllables[s(-37)] = op_syllable(Trit::N, 0);  // MACRO -> FINISH
    m.mem.load_rom_page(12, img).unwrap();
    assert_eq!(m.run(100), RunOutcome::Finished);
    assert!(!m.is_running());
    assert_eq!(m.stack_t_senior().unwrap().to_i16(), -40); // -40 -> +40 -> -40

    // Part 2: the firmware body once h is primed. (The reset values of h
    // are not stated in the description; the real program equipment either
    // received them from hardware or synthesised them — see README.)
    // push 6, push 7 (from ROM 12 via h[Z]=12), S+T, store to RAM, FINISH.
    let mut m = Machine::new();
    m.power_on();
    m.regs.set_h(Trit::Z, 12); // operand page = ROM 12
    // h[N], h[P] stay 0 (RAM): the store target syllable uses k2 = N.
    let mut img = crate::memory::Page::zeroed();
    img.syllables[s(10)] = Tryte::from_i16(6);
    img.syllables[s(11)] = Tryte::from_i16(7);
    img.syllables[s(12)] = addr_syllable(Trit::N, Trit::N, 20); // -> h[N] = 0
    img.syllables[s(-40)] = addr_syllable(Trit::N, Trit::Z, 10);
    img.syllables[s(-39)] = addr_syllable(Trit::N, Trit::Z, 11);
    img.syllables[s(-38)] = op_syllable(Trit::Z, 10); // S+T
    img.syllables[s(-37)] = addr_syllable(Trit::N, Trit::Z, 12);
    img.syllables[s(-36)] = op_syllable(Trit::Z, 4);  // W=S -> m[0, 20]
    img.syllables[s(-35)] = op_syllable(Trit::N, 0);  // MACRO -> FINISH
    m.mem.load_rom_page(12, img).unwrap();
    assert_eq!(m.run(1000), RunOutcome::Finished);
    assert_eq!(m.mem.read(0, 20).unwrap().to_i16(), 13); // 6 + 7
    assert_eq!(m.regs.ch(), 12);                          // ran from ROM
}


// ============================================================================
// Disassembler, dumps, tracing
// ============================================================================

use crate::asm::{decode, dump_page, format_syllable, trace_state, Syllable, BAS_NAMES, SPEC_NAMES};

#[test]
fn test_disasm_name_tables_complete() {
    assert_eq!(BAS_NAMES.len(), 27);
    assert_eq!(SPEC_NAMES.len(), 27);
    for ko in -13..=13 {
        let t = op_syllable(Trit::Z, ko);
        assert_eq!(format_syllable(&t), BAS_NAMES[(ko + 13) as usize]);
        let t = op_syllable(Trit::P, ko);
        assert_eq!(format_syllable(&t), SPEC_NAMES[(ko + 13) as usize]);
    }
}

#[test]
fn test_disasm_spot_names() {
    assert_eq!(format_syllable(&op_syllable(Trit::Z, -13)), "LST");
    assert_eq!(format_syllable(&op_syllable(Trit::Z, 10)), "S+T");
    assert_eq!(format_syllable(&op_syllable(Trit::Z, 13)), "LHT");
    assert_eq!(format_syllable(&op_syllable(Trit::P, -13)), "COPYG1");
    assert_eq!(format_syllable(&op_syllable(Trit::P, 13)), "LOADG3");
    assert_eq!(format_syllable(&op_syllable(Trit::P, 0)), "RETNMC");
}

#[test]
fn test_disasm_macro_and_addr() {
    let t = op_syllable(Trit::N, 0);
    match decode(&t) {
        Syllable::Op { group, .. } => assert_eq!(group, OpGroup::Macro),
        _ => panic!("macro decode"),
    }
    assert_eq!(format_syllable(&t), "MACRO @-27"); // ka shares its senior trit with k3
    assert_eq!(format_syllable(&addr_syllable(Trit::N, Trit::Z, 20)), "Ref1 h[0]@20");
    assert_eq!(format_syllable(&addr_syllable(Trit::P, Trit::N, -13)), "Ref3 h[-]@-13");
    assert_eq!(format_syllable(&addr_syllable(Trit::Z, Trit::P, 40)), "Ref2 h[+]@40");
}

#[test]
fn test_dump_page_on_firmware() {
    let mut m = Machine::new();
    m.power_on();
    m.regs.set_h(Trit::Z, 12);
    let s = |a: i8| (a - -40) as usize;
    let mut img = crate::memory::Page::zeroed();
    img.syllables[s(10)] = Tryte::from_i16(6);
    img.syllables[s(-40)] = addr_syllable(Trit::N, Trit::Z, 10);
    img.syllables[s(-39)] = op_syllable(Trit::Z, 10); // S+T
    img.syllables[s(-38)] = op_syllable(Trit::N, 0);  // MACRO -> FINISH
    m.mem.load_rom_page(12, img).unwrap();
    let dump = dump_page(&m, 12);
    assert!(dump.contains("--- page 12 (ROM)"));
    assert!(dump.contains("Ref1 h[0]@10"));
    assert!(dump.contains("S+T"));
    assert!(dump.contains("MACRO @-27"));
    assert!(dump.contains("ZPZZPN") || dump.contains("ZZZZZZ"));
}

#[test]
fn test_trace_state_and_run_with() {
    // Interrupt mode: here a MACRO syllable is FINISH (§5.3).
    let mut m = bench();
    m.regs.set_mode(Mode::Interrupt);
    m.mem.write(0, 10, Tryte::from_i16(5)).unwrap();
    m.mem.write(0, 11, Tryte::from_i16(7)).unwrap();
    m.mem.write(0, -40, addr_syllable(Trit::N, Trit::Z, 10)).unwrap();
    m.mem.write(0, -39, addr_syllable(Trit::N, Trit::Z, 11)).unwrap();
    m.mem.write(0, -38, op_syllable(Trit::Z, 10)).unwrap(); // S+T
    m.mem.write(0, -37, op_syllable(Trit::N, 0)).unwrap();  // MACRO -> FINISH
    let trace0 = trace_state(&m);
    assert!(trace0.contains("mode=Interrupt (0,-40)"));
    assert!(trace0.contains("next=Ref1 h[0]@10"));
    let mut seen = 0u64;
    let outcome = m.run_with(100, |mach, step| {
        seen += 1;
        if step == 0 {
            let tr = trace_state(mach);
            assert!(tr.contains("mode=Interrupt"));
            assert!(tr.contains("v=........."));
        }
    });
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(seen, 3); // two Refs + S+T; MACRO finishes without a step
    let tr = trace_state(&m);
    assert!(tr.contains("mode=Interrupt (0,-37)")); // ca after INCRC of S+T
}


// ============================================================================
// Assembler
// ============================================================================

use crate::asm::{assemble, assemble_syllable, AsmError};

#[test]
fn test_assemble_roundtrip() {
    // Every mnemonic roundtrips: assemble(format_syllable(t)) == t.
    for ko in -13..=13 {
        for (group, _) in [(Trit::Z, 0), (Trit::P, 0)] {
            let t = op_syllable(group, ko);
            let text = format_syllable(&t);
            let back = assemble_syllable(&text).unwrap();
            assert_eq!(back.to_i16(), t.to_i16(), "{}", text);
        }
    }
    for &(len_k1, k2, ka) in &[(Trit::N, Trit::Z, 20i8), (Trit::N, Trit::N, -13),
                               (Trit::Z, Trit::P, 40), (Trit::P, Trit::N, -40),
                               (Trit::Z, Trit::N, 0), (Trit::P, Trit::Z, 13)] {
        let t = addr_syllable(len_k1, k2, ka);
        let text = format_syllable(&t);
        let back = assemble_syllable(&text).unwrap();
        assert_eq!(back.to_i16(), t.to_i16(), "{}", text);
    }
    // MACRO: ka in -40..=-14.
    let t = op_syllable(Trit::N, 0);
    let back = assemble_syllable(&format_syllable(&t)).unwrap();
    assert_eq!(back.to_i16(), t.to_i16());
    // Data literal.
    assert_eq!(assemble_syllable("=-159").unwrap().to_i16(), -159);
    assert_eq!(assemble_syllable("=364").unwrap().to_i16(), 364);
}

#[test]
fn test_assemble_errors() {
    assert!(matches!(assemble_syllable("FOO"), Err(AsmError::Unknown(_))));
    assert!(matches!(assemble_syllable("Ref2 h[0]@5"), Err(AsmError::NotRepresentable(_))));
    assert!(matches!(assemble_syllable("MACRO @5"), Err(AsmError::NotRepresentable(_))));
    assert!(matches!(assemble_syllable("MACRO @-27"), Ok(_)));
    assert!(matches!(assemble_syllable("Ref4 h[+]@1"), Err(AsmError::BadForm(_))));
    assert!(matches!(assemble_syllable("=500"), Err(AsmError::BadForm(_))));
}

#[test]
fn test_firmware_written_as_text() {
    // The stage-6 firmware body (part 2), written in assembly text.
    let mut m = Machine::new();
    m.power_on();
    m.regs.set_h(Trit::Z, 12);
    let s = |a: i8| (a - -40) as usize;
    let mut img = crate::memory::Page::zeroed();
    img.syllables[s(10)] = assemble_syllable("=6").unwrap();
    img.syllables[s(11)] = assemble_syllable("=7").unwrap();
    img.syllables[s(12)] = assemble_syllable("Ref1 h[-]@20").unwrap(); // -> h[N] = 0 (RAM)
    let program = assemble(&[
        "Ref1 h[0]@10",
        "Ref1 h[0]@11",
        "S+T",
        "Ref1 h[0]@12",
        "W=S",
        "MACRO @-27", // FINISH in interrupt mode
    ]).unwrap();
    for (i, t) in program.iter().enumerate() {
        img.syllables[s(-40 + i as i8)] = *t;
    }
    m.mem.load_rom_page(12, img).unwrap();
    assert_eq!(m.run(1000), RunOutcome::Finished);
    assert_eq!(m.mem.read(0, 20).unwrap().to_i16(), 13);
    // And the disassembler reads our text back.
    let dump = dump_page(&m, 12);
    assert!(dump.contains("Ref1 h[-]@20"));
    assert!(dump.contains("W=S"));
}


// ============================================================================
// Character layer (§6.1)
// ============================================================================

use crate::charset::{control_name, decode_char, encode_char, is_control};

#[test]
fn test_charset_decode() {
    assert_eq!(decode_char(364), Some('0'));   // 444
    assert_eq!(decode_char(363), Some('1'));   // 443
    assert_eq!(decode_char(-364), Some(']'));  // WWW
    assert_eq!(decode_char(121), Some('А'));   // 144
    assert_eq!(decode_char(-256), Some('_'));  // XZW
    assert_eq!(decode_char(-121), Some('-'));  // ZWW hyphen
    assert_eq!(decode_char(1), Some('Я'));     // 001
    assert_eq!(decode_char(3), Some('Ю'));     // 003
    assert_eq!(decode_char(270), None);        // 330 пропуск (control)
    assert!(is_control(270));
    assert_eq!(control_name(270), Some("Пропуск"));
    assert_eq!(control_name(283), Some("Возврат каретки")); // 344
    assert_eq!(decode_char(0), None);          // unassigned
    assert!(!is_control(0));
}

#[test]
fn test_charset_encode() {
    assert_eq!(encode_char('0'), Some(364));
    assert_eq!(encode_char('А'), Some(121));
    assert_eq!(encode_char('-'), Some(333));  // nonary 410, digit minus
    assert_eq!(encode_char(']'), Some(-364));
    assert_eq!(encode_char('?'), None);
}

// ============================================================================
// Syllable configuration layer (generated by setun70's build.rs)
// ============================================================================

use crate::syllables::info_by_value;

#[test]
fn test_syllable_info_table() {
    let i = info_by_value(-351).unwrap(); // LST
    assert!(i.is_op_syllable);
    assert_eq!(i.op_group, 0);
    assert_eq!(i.op_code, -13);
    assert_eq!(i.mnemonic, "LST");

    let i = info_by_value(-126).unwrap(); // LOADG1
    assert!(i.is_op_syllable);
    assert_eq!(i.op_group, 1);
    assert_eq!(i.op_code, 11);
    assert_eq!(i.mnemonic, "LOADG1");

    let i = info_by_value(-9).unwrap(); // MACRO
    assert!(i.is_op_syllable);
    assert_eq!(i.op_group, -1);
    assert_eq!(i.mnemonic, "MACRO @-27");

    let i = info_by_value(-181).unwrap(); // Ref1 h[0]@20
    assert!(!i.is_op_syllable);
    assert_eq!(i.mnemonic, "Ref1 h[0]@20");

    for v in -364..=364i16 {
        let i = info_by_value(v).unwrap();
        assert!(!i.mnemonic.is_empty());
        if i.is_op_syllable {
            assert!((-13..=13).contains(&i.op_code));
            assert!((-1..=1).contains(&i.op_group));
        }
    }
}

#[test]
fn test_format_syllable_table_driven() {
    assert_eq!(format_syllable(&op_syllable(Trit::Z, -13)), "LST");
    assert_eq!(format_syllable(&addr_syllable(Trit::N, Trit::Z, 20)),
               "Ref1 h[0]@20");
    assert_eq!(format_syllable(&op_syllable(Trit::N, 0)), "MACRO @-27");
}


// ============================================================================
// Teletype echo program
// ============================================================================

use crate::io::Teletype;

/// Echo: read a character from the teletype keyboard (group -1, dev 1)
/// and print it back on the teletype printer (group 0, dev 2).
///
/// The interrupt machinery is the wait mechanism: LOADU1 triggers the
/// read, and the v-request enters IR; the vectored dispatcher (a single
/// C=T at (13,-13), jumping to the address stored in the vector cell
/// m[13, w]) routes the read completion to COPYG1; EXCHP/RETNMC restore
/// the macro context. The write completion routes to a no-op handler.
#[test]
fn test_teletype_echo_program() {
    let mut m = Machine::new();
    m.power_on();
    m.regs.set_mode(Mode::Macro);
    m.regs.set_ch(0);
    m.regs.set_ca(-40);
    m.regs.set_ph(1);
    m.regs.set_h(Trit::Z, 0);
    // u[0] (group 0) preset: PASS to device 2 (printer).
    m.regs.u[1].set_trit(0, Trit::P);
    m.regs.u[1].set_field(2, 3, 2);

    // Data: m[0,10] = u[-1] syllable (TAKE, dev 1); m[0,13] = idle target.
    m.mem.write(0, 10, Tryte::from_i16(26)).unwrap();
    m.mem.write(0, 13, Tryte::from_i16(-38)).unwrap();
    // Program: arm the read once, then idle. The interrupt handlers do
    // the echo (a window-consuming race is impossible: COPYG1 and LOADG2
    // run back to back inside one handler).
    m.mem.write(0, -40, addr_syllable(Trit::N, Trit::Z, 10)).unwrap();
    m.mem.write(0, -39, op_syllable(Trit::P, 5)).unwrap();   // LOADU1
    m.mem.write(0, -38, op_syllable(Trit::Z, -1)).unwrap();  // T=C (idle)
    m.mem.write(0, -37, addr_syllable(Trit::N, Trit::Z, 13)).unwrap();
    m.mem.write(0, -36, op_syllable(Trit::Z, 1)).unwrap();   // C=T (idle)

    // ROM page 13: vectors, dispatcher, handlers.
    let s13 = |a: i8| (a - -40) as usize;
    let mut img = crate::memory::Page::zeroed();
    img.syllables[s13(-39)] = Tryte::from_i16(-30); // read-completion vector
    img.syllables[s13(-38)] = Tryte::from_i16(-20); // write-completion vector
    img.syllables[s13(-13)] = op_syllable(Trit::Z, 1);   // C=T: dispatch by t
    // Read handler: EXCHP; COPYG1 (char into the macro window);
    // LOADG2 (window -> g[0] -> printer); one push (pa balance); RETNMC.
    img.syllables[s13(-30)] = op_syllable(Trit::P, -3);  // EXCHP
    img.syllables[s13(-29)] = op_syllable(Trit::P, -13); // COPYG1
    img.syllables[s13(-28)] = op_syllable(Trit::P, 12);  // LOADG2
    img.syllables[s13(-27)] = addr_syllable(Trit::N, Trit::Z, 10); // balance push
    img.syllables[s13(-26)] = op_syllable(Trit::P, 0);   // RETNMC
    // Write handler: EXCHP; RETNMC.
    img.syllables[s13(-20)] = op_syllable(Trit::P, -3);  // EXCHP
    img.syllables[s13(-19)] = op_syllable(Trit::P, 0);   // RETNMC
    m.mem.load_rom_page(13, img).unwrap();

    // Teletype: keyboard at (-1, 1), printer at (0, 2).
    let tty = std::rc::Rc::new(std::cell::RefCell::new(Teletype::new("А")));
    let printer = tty.clone();
    m.io.attach(-1, 1, tty);
    m.io.attach(0, 2, printer.clone());

    let outcome = m.run(500);
    assert_eq!(outcome, RunOutcome::StepLimitExceeded); // idle after the input
    assert_eq!(printer.borrow().output, "А");
}

#[test]
fn test_teletype_space_and_greeting() {
    use crate::io::{syllable_to_row, Teletype};
    let mut tty = Teletype::new("Я СЕТУНЬ 70\n");
    // drain the input through the device and re-feed it to the printer half
    let mut rows = Vec::new();
    while let Some(b) = crate::io::ByteDevice::read(&mut tty) {
        rows.push(b);
    }
    assert_eq!(rows.len(), 12); // 9 visible + 2 spaces + CR
    let mut out = Teletype::new("");
    for b in rows {
        crate::io::ByteDevice::write(&mut out, b);
    }
    assert_eq!(out.output, "Я СЕТУНЬ 70\n");
    // Пропуск (330 = 270) is the space
    let row = syllable_to_row(&Tryte::from_i16(270));
    let mut t2 = Teletype::new("");
    crate::io::ByteDevice::write(&mut t2, row);
    assert_eq!(t2.output, " ");
}


#[test]
fn test_teletype_alphabet_filter() {
    use crate::charset::{encode_teletype, is_representable};

    // The alphabet boundary: uppercase only, table signs, space, newline.
    assert_eq!(encode_teletype('А'), Some(121));
    assert_eq!(encode_teletype('7'), Some(351));
    assert_eq!(encode_teletype('!'), Some(-10)); // 0ZZ
    assert_eq!(encode_teletype(' '), Some(270)); // Пропуск
    assert_eq!(encode_teletype('\n'), Some(283)); // Возврат каретки
    assert!(!is_representable('а'));   // lowercase Cyrillic: absent
    assert!(!is_representable('a'));   // lowercase Latin: absent
    assert!(!is_representable('~'));
    assert!(!is_representable('{'));
    assert!(!is_representable('ё'));

    // The device drops everything outside the alphabet at input time.
    let mut tty = Teletype::new("УРА, Заработало! заведут баркас и через 42 к-а-к! ЭТО ТОЧНО... ~{}q\n");
    let mut rows = Vec::new();
    while let Some(b) = crate::io::ByteDevice::read(&mut tty) {
        rows.push(b);
    }
    let mut out = Teletype::new("");
    for b in rows {
        crate::io::ByteDevice::write(&mut out, b);
    }
    // lowercase words vanish, their spaces stay; ~{}q are ignored
    assert_eq!(out.output, "УРА, З!     42 --! ЭТО ТОЧНО... \n");
}
