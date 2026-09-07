//! Syllable disassembler, page dumps, and state tracing (stage 7).
//!
//! Notation (ours — the historical notation is not documented):
//!   operational syllables print as their §4 mnemonics: `LST`, `S+T`,
//!   `COPYG1`, ...; a macro syllable prints as `MACRO @ka`;
//!   address syllables print as `Ref<len> h[<k2>]@<ka>` — a reference of
//!   `len` syllables whose senior sits at `ka` on page h[k2].
//! Operand length uses the k1+2 decoding assumption (see README).

use analemma::trit::Trit;
use analemma::tryte::Tryte;
use crate::machine::Machine;
use crate::registers::OpGroup;

/// BASIC mnemonics B1..B27 (ko −13..=+13).
pub const BAS_NAMES: [&str; 27] = [
    "LST", "COT", "XNN", "E-1", "E=0", "E+1", "T-E", "E=T", "T+E",
    "CLT", "CET", "CGT", "T=C", "R=T", "C=T", "T=W", "YFT", "W=S",
    "SMT", "Y=T", "SAT", "S-T", "TDN", "S+T", "LBT", "L*T", "LHT",
];

/// SPEC mnemonics S1..S27 (ko −13..=+13).
pub const SPEC_NAMES: [&str; 27] = [
    "COPYG1", "COPYG2", "COPYG3", "COPYF1", "COPYF2", "COPYF3",
    "LOADQ1", "LOADQ2", "LOADQ3", "COPYP", "EXCHP", "LOADP",
    "COPYMC", "RETNMC", "LOADMC", "LOADH1", "LOADH2", "LOADH3",
    "LOADU1", "LOADU2", "LOADU3", "LOADF1", "LOADF2", "LOADF3",
    "LOADG1", "LOADG2", "LOADG3",
];

/// A decoded syllable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Syllable {
    /// k1 = k2 = 0.
    Op { group: OpGroup, code: i8 },
    /// Address syllable (a reference): len = k1 + 2 syllables whose senior
    /// sits at ka on page h[k2].
    Addr { len: u8, k2: Trit, ka: i8 },
}

fn field(t: &Tryte, from: usize, to: usize) -> i8 {
    let mut v: i8 = 0;
    for i in from..=to {
        v = v * 3 + t.trits[i].to_i8();
    }
    v
}

/// Decodes a syllable tryte.
pub fn decode(t: &Tryte) -> Syllable {
    if t.trits[0] == Trit::Z && t.trits[1] == Trit::Z {
        Syllable::Op {
            group: OpGroup::from_trit(t.trits[2]),
            code: field(t, 3, 5),
        }
    } else {
        Syllable::Addr {
            len: (t.trits[0].to_i8() + 2) as u8,
            k2: t.trits[1],
            ka: field(t, 2, 5),
        }
    }
}

fn trit_char(t: Trit) -> char {
    match t {
        Trit::N => '-',
        Trit::Z => '0',
        Trit::P => '+',
    }
}

/// Formats a syllable for listings and traces via the emulator's syllable
/// configuration layer (`syllables::info_by_value`) — no run-time
/// decoding; the decoder below is the fallback.
pub fn format_syllable(t: &Tryte) -> String {
    if let Some(info) = crate::syllables::info_by_value(t.to_i16()) {
        return info.mnemonic.to_string();
    }
    match decode(t) {
        Syllable::Op { group, code } => {
            let idx = (code + 13) as usize;
            let name = match group {
                OpGroup::Basic => BAS_NAMES[idx],
                OpGroup::Spec => SPEC_NAMES[idx],
                OpGroup::Macro => return format!("MACRO @{}", field(t, 2, 5)),
            };
            name.to_string()
        }
        Syllable::Addr { len, k2, ka } => {
            format!("Ref{} h[{}]@{}", len, trit_char(k2), ka)
        }
    }
}

fn trits_string(t: &Tryte) -> String {
    t.trits.iter().map(|&x| match x {
        Trit::N => 'N',
        Trit::Z => 'Z',
        Trit::P => 'P',
    }).collect()
}

/// (trits, nonary) strings via the analemma table when the core is built
/// at 6 trits; local fallbacks otherwise.
fn table_strings(t: &Tryte) -> (String, String) {
    match analemma::table::entry_by_value(t.to_i16() as i32) {
        Some(e) => (e.str_trits.to_string(), e.str_nonary.to_string()),
        None => (trits_string(t), String::new()),
    }
}

/// Dumps one page: address, value, trits, nonary, decoded syllable.
pub fn dump_page(m: &Machine, page: i8) -> String {
    let mut out = format!("--- page {} ({})\n", page,
        if crate::memory::Memory1::is_ram(page) { "RAM" } else { "ROM" });
    for addr in crate::memory::ADDR_MIN..=crate::memory::ADDR_MAX {
        if let Some(t) = m.mem.read(page, addr) {
            let (tr, no) = table_strings(&t);
            let name = match crate::syllables::info_by_value(t.to_i16()) {
                Some(info) => info.mnemonic.to_string(),
                None => format_syllable(&t),
            };
            out.push_str(&format!(
                "{:>4} {:>5} {:<6} {:<3} {}\n",
                addr, t.to_i16(), tr, no, name
            ));
        }
    }
    out
}

/// One-line machine state for tracing.
pub fn trace_state(m: &Machine) -> String {
    let r = &m.regs;
    let v: String = r.v.trits.iter().map(|&t| if t == Trit::P { '1' } else { '.' }).collect();
    let k = m.mem.read(r.ch(), r.ca())
        .map(|t| format_syllable(&t))
        .unwrap_or_else(|| "??".to_string());
    let t = m.stack_t().map(|w| w.to_i64().to_string()).unwrap_or("—".to_string());
    let s = m.stack_s().map(|w| w.to_i64().to_string()).unwrap_or("—".to_string());
    format!(
        "mode={:?} ({},{}) next={} | p=({},{}) T={} S={} | R={} Y={} e={} w={} v={}",
        r.mode(), r.ch(), r.ca(), k,
        r.ph(), r.pa(), t, s,
        r.r.to_i64(), r.y.to_i64(), r.e.to_i16(),
        r.w_value(), v,
    )
}


// ============================================================================
// Assembly: text -> syllable (inverse of format_syllable)
// ============================================================================

/// Assembly error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AsmError {
    /// Not a mnemonic, not a Ref, not MACRO, not a value literal.
    Unknown(String),
    /// Malformed Ref/MACRO/value syntax.
    BadForm(String),
    /// The syllable cannot be represented (e.g. Ref2 h[0]@.. — the
    /// k1 = 0 & k2 = 0 pattern is reserved for operational syllables).
    NotRepresentable(String),
}

fn op_tryte(group: Trit, ko: i8) -> Tryte {
    let mut t = [Trit::Z; 6];
    t[2] = group;
    // ko field trits[3..5], MSB at 3
    let mut v = ko;
    for i in (3..=5).rev() {
        let rem = v.rem_euclid(3) as i8;
        t[i] = Trit::from_i8(if rem == 2 { -1 } else { rem }).unwrap();
        v = (v - (if rem == 2 { -1 } else { rem })) / 3;
    }
    Tryte { trits: t }
}

/// Assembles one syllable line.
///
/// Grammar:
///   `=V`               — a data syllable with the balanced value V (−364..=364)
///   `<BAS mnemonic>`   — e.g. `S+T`, `LST`
///   `<SPEC mnemonic>`  — e.g. `LOADG3`, `RETNMC`
///   `MACRO @ka`        — ka must lie in −40..=−14 (its senior trit is k3 = −1)
///   `Ref<len> h[<sym>]@<ka>` — sym ∈ {`-`, `0`, `+`}, len ∈ {1, 2, 3};
///                        `Ref2 h[0]@..` is rejected (reserved pattern)
pub fn assemble_syllable(src: &str) -> Result<Tryte, AsmError> {
    let src = src.trim();
    if let Some(rest) = src.strip_prefix('=') {
        let v: i16 = rest.trim().parse()
            .map_err(|_| AsmError::BadForm(src.to_string()))?;
        if !(-364..=364).contains(&v) {
            return Err(AsmError::BadForm(src.to_string()));
        }
        return Ok(Tryte::from_i16(v));
    }
    if let Some(rest) = src.strip_prefix("MACRO") {
        let ka: i8 = rest.trim().trim_start_matches('@').trim().parse()
            .map_err(|_| AsmError::BadForm(src.to_string()))?;
        if !(-40..=-14).contains(&ka) {
            return Err(AsmError::NotRepresentable(format!(
                "macro ka {} outside -40..=-14 (its senior trit is k3 = -1)", ka)));
        }
        let mut t = [Trit::Z; 6];
        t[2] = Trit::N;
        // place ka in trits[2..5] with trits[2] fixed to N
        let mut v = ka;
        for i in (3..=5).rev() {
            let rem = v.rem_euclid(3) as i8;
            let tr = if rem == 2 { -1 } else { rem };
            t[i] = Trit::from_i8(tr).unwrap();
            v = (v - tr) / 3;
        }
        return Ok(Tryte { trits: t });
    }
    if src.starts_with("Ref") {
        // Ref<len> h[<sym>]@<ka>
        let body = &src[3..];
        let len: u8 = body.chars().next()
            .and_then(|c| c.to_digit(10)).map(|d| d as u8)
            .ok_or_else(|| AsmError::BadForm(src.to_string()))?;
        if !(1..=3).contains(&len) {
            return Err(AsmError::BadForm(src.to_string()));
        }
        let rest = &body[1..];
        let rest = rest.trim_start().strip_prefix("h[").ok_or_else(|| AsmError::BadForm(src.to_string()))?;
        let (sym, rest) = rest.split_at(1.min(rest.len()));
        let k2 = match sym {
            "-" => Trit::N,
            "0" => Trit::Z,
            "+" => Trit::P,
            _ => return Err(AsmError::BadForm(src.to_string())),
        };
        let ka_str = rest.strip_prefix("]@").ok_or_else(|| AsmError::BadForm(src.to_string()))?;
        let ka: i8 = ka_str.trim().parse().map_err(|_| AsmError::BadForm(src.to_string()))?;
        if !(-40..=40).contains(&ka) {
            return Err(AsmError::BadForm(src.to_string()));
        }
        if len == 2 && k2 == Trit::Z {
            return Err(AsmError::NotRepresentable(format!(
                "{}: k1 = 0 & k2 = 0 is the operational-syllable pattern", src)));
        }
        let mut t = [Trit::Z; 6];
        t[0] = Trit::from_i8(len as i8 - 2).unwrap();
        t[1] = k2;
        let mut v = ka;
        for i in (2..=5).rev() {
            let rem = v.rem_euclid(3) as i8;
            let tr = if rem == 2 { -1 } else { rem };
            t[i] = Trit::from_i8(tr).unwrap();
            v = (v - tr) / 3;
        }
        return Ok(Tryte { trits: t });
    }
    for (i, name) in BAS_NAMES.iter().enumerate() {
        if *name == src {
            return Ok(op_tryte(Trit::Z, i as i8 - 13));
        }
    }
    for (i, name) in SPEC_NAMES.iter().enumerate() {
        if *name == src {
            return Ok(op_tryte(Trit::P, i as i8 - 13));
        }
    }
    Err(AsmError::Unknown(src.to_string()))
}

/// Assembles a sequence of syllables (e.g. a program in textual POLIZ).
pub fn assemble(srcs: &[&str]) -> Result<Vec<Tryte>, AsmError> {
    srcs.iter().map(|s| assemble_syllable(s)).collect()
}
