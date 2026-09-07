//! Punched-tape loader (stage 12).
//!
//! Tape format: §6 rows, one row per syllable, every syllable single-sign
//! (the §6.1 alphabet satisfies this; mixed-sign syllables are not
//! tape-representable — the description resolves this by restricting the
//! admissible codes).
//!
//! The loader is a straight-line program on page 0 (from -40). It
//! initiates reading once: LOADU1 copies the exchange-mode syllable into
//! u[-1] and sets the sync trigger a[-1]. Each read-completion interrupt
//! (handler [EXCHP, COPYG1, RETNMC] on ROM page 13) copies the byte from
//! the g buffer into the stack window; COPYG1 sets the sync trigger again
//! (a[l] := 1), so the next byte is requested automatically. The loader
//! stores the window senior syllable at the next fixed address (W=S) and
//! finally jumps to the loaded code (C=T). Fixed address syllables are
//! len-1 and never touch the tape, so their mixed signs are harmless.
//!
//! Page-0 layout (RAM):
//!   loader program:   -40..-9  (32 syllables)
//!   u[-1] cell:        10 = 26 (TAKE, dev 1)      [written by the loader]
//!   jump-target cell:  11 = 12 (loaded start)     [written by the loader]
//!   loaded code:       12..18
//!   loader constants:  19=26, 20=12, 21=269 (Ref1 h[0]@10), 22=-136 (Ref1 h[0]@11)
//!   loaded data:       28=addr syllable, 30=4, 31=9, 37=13
//!   store scratch:     35, 36  (the loaded demo stores 13 at m[0,36])

use analemma::tryte::Tryte;
use crate::asm::assemble;

/// Where the loaded code lands and where the demo stores its result.
pub const LOADED_START: i8 = 12;
pub const RESULT_ADDR: i8 = 36;

/// Destination addresses of the tape bytes (fixed in the loader's Ref1 syllables).
/// All are mono-sign where it matters — only the TAPE syllables must be
/// single-sign; these fixed values never reach the tape themselves.
pub const LOAD_ADDRS: [i8; 11] = [12, 13, 14, 15, 16, 17, 18, 30, 31, 28, 37];

/// The loader program (assembled), to be written at page 0 from -40.
pub fn loader_program() -> Vec<Tryte> {
    let mut src: Vec<String> = vec![
        "Ref1 h[0]@19".into(),   // 26
        "Ref1 h[0]@21".into(),   // addr syllable for m[0,10]
        "W=S".into(),            // m[0,10] := 26  (u[-1] = TAKE, dev 1)
        "Ref1 h[0]@20".into(),   // 12
        "Ref1 h[0]@22".into(),   // addr syllable for m[0,11]
        "W=S".into(),            // m[0,11] := 12  (jump target)
        "Ref3 h[0]@10".into(),   // push the u syllable
        "LOADU1".into(),         // u[-1] := t; set the sync trigger (request byte 0)
    ];
    for &a in &LOAD_ADDRS {
        src.push(format!("Ref1 h[0]@{a}"));
        src.push("W=S".into());
    }
    src.push("Ref3 h[0]@11".into()); // push 12
    src.push("C=T".into());          // jump to the loaded program
    let refs: Vec<&str> = src.iter().map(|s| s.as_str()).collect();
    assemble(&refs).unwrap()
}

/// Loader constant cells: (address, value).
pub fn loader_constants() -> [(i8, i16); 4] {
    [(19, 26), (20, 12), (21, 269), (22, -136)]
}

/// The read-completion vector / dispatcher / handler on ROM page 13.
pub fn rom13_vectors(page13: &mut crate::memory::Page) {
    let s = |a: i8| (a - -40) as usize;
    page13.syllables[s(-39)] = Tryte::from_i16(-30); // read vector
    page13.syllables[s(-13)] = assemble(&["C=T"]).unwrap()[0]; // dispatcher
    page13.syllables[s(-30)] = assemble(&["EXCHP"]).unwrap()[0];
    page13.syllables[s(-29)] = assemble(&["COPYG1"]).unwrap()[0];
    page13.syllables[s(-28)] = assemble(&["RETNMC"]).unwrap()[0];
}

/// A sample tape: the "4 + 9" demo in loaded form. Computes 13 and stores
/// it at m[0,36], then idles in a loop. All 11 syllables are single-sign.
///
/// Order matches LOAD_ADDRS: code at 12..18, data at 30, 31, 28, 37.
pub fn sample_tape() -> Vec<Tryte> {
    assemble(&[
        "Ref3 h[0]@30", // 12: push data (4)
        "Ref3 h[0]@31", // 13: push data (9)
        "S+T",          // 14
        "Ref3 h[0]@28", // 15: push the W=S address syllable (cell 28)
        "W=S",          // 16: store 13 at m[0,36] (and 0 at 35)
        "Ref3 h[0]@37", // 17: push the idle target (13)
        "C=T",          // 18: ca := 13 -> loop 13..18 forever
        "=4",           // 30
        "=9",           // 31
        "=39",          // 28: Ref2 h[+]@36 (h[P] = 0), len 2
        "=13",          // 37
    ]).unwrap()
}
