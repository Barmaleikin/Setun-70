//! # setun70
//!
//! Emulator of the Setun-70 ternary computer, written directly from the
//! algorithmic description («Структура и алгоритм функционирования малой
//! вычислительной машины "Сетунь-70"», rotaprint 27-VT (417), MSU, 1970).
//!
//! Architecture essentials (§2): syllable-addressable level-1 memory
//! (27 pages × 81 trytes), stack machine executing reverse Polish notation,
//! two-level operation set (27 basic + 27 privileged + 27 macro),
//! 9 RAM pages and 18 ROM pages, indirect addressing via three pointer
//! registers, interrupts.
//!
//! Current stage: the complete processor per the description — level-1
//! memory (RAM/ROM), level-2 drum, the register file, the power-on
//! sequence, the stack window (T/S/t), CYCLE with address-syllable push
//! (REFSYL), the BASIC group B1..B27, the MACRO dispatcher, the IR
//! interrupt entry (page-13 vector table), and the SPEC group S1..S27
//! (I/O buffers per §6, macro context, stack-pointer ops, drum exchange),
//! the peripheral layer (`io`: byte devices behind g/u/a, §6 tape rows),
//! and the `run` loop. WATCH (clock) and ATTENT (panel button) requests
//! can be raised by writing the v register directly. `asm` provides both
//! directions: disassembly (`format_syllable`, `dump_page`, `trace_state`)
//! and assembly (`assemble_syllable`: `=V` data, mnemonics, `MACRO @ka`,
//! `Ref<len> h[<sym>]@<ka>`).
//!
//! Documented decoding assumptions (OCR-damaged spots of the rotaprint):
//! operand length = k1 + 2 (N→1, Z→2, P→3); B1/B2/B3 reconstructed as
//! described in `machine.rs`.

#[path = "syllable_table_generated.rs"]
pub mod syllables;

pub mod asm;
pub mod charset;
pub mod io;
pub mod machine;
pub mod memory;
pub mod registers;

#[cfg(test)]
mod tests;
