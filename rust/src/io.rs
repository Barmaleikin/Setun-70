//! Peripheral layer: byte devices behind the g/u/a registers (§4 INOUT).
//!
//! A device is addressed by (group, number): group ∈ {−1, 0, 1} (the
//! index of INOUT(i)), number from u[i, 3:4] (2 trits, −4..=4). The byte
//! level is the §6 punched-tape row: bit i (0-based) is the punch at tape
//! position i+1; bit 6 is the sign track.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// A byte-at-a-time device (tape reader, punch, future teletype).
pub trait ByteDevice {
    /// Next byte, or None at end of input / when not ready.
    fn read(&mut self) -> Option<u8>;
    /// Accept a byte.
    fn write(&mut self, byte: u8);
}

/// Tape reader over an in-memory byte string.
pub struct TapeReader {
    data: Vec<u8>,
    pos: usize,
}

impl TapeReader {
    pub fn new(data: Vec<u8>) -> Self {
        TapeReader { data, pos: 0 }
    }
}

impl ByteDevice for TapeReader {
    fn read(&mut self) -> Option<u8> {
        let b = self.data.get(self.pos).copied();
        if b.is_some() {
            self.pos += 1;
        }
        b
    }
    fn write(&mut self, _byte: u8) {}
}

/// Tape punch collecting output bytes.
#[derive(Default)]
pub struct TapePunch {
    pub out: Vec<u8>,
}

impl ByteDevice for TapePunch {
    fn read(&mut self) -> Option<u8> {
        None
    }
    fn write(&mut self, byte: u8) {
        self.out.push(byte);
    }
}

/// Device bus: (group, number) -> shared device handle.
#[derive(Default)]
pub struct IoBus {
    devices: HashMap<(i8, i8), Rc<RefCell<dyn ByteDevice>>>,
}

impl IoBus {
    pub fn attach(&mut self, group: i8, number: i8, device: Rc<RefCell<dyn ByteDevice>>) {
        self.devices.insert((group, number), device);
    }

    pub(crate) fn get(&self, group: i8, number: i8) -> Option<Rc<RefCell<dyn ByteDevice>>> {
        self.devices.get(&(group, number)).cloned()
    }
}

/// §6: syllable -> tape row. Position i+1 (bit i) punches for syllable
/// element 7−(i+1) = 6−i... i.e. tape position p (1-based) maps to
/// element 7−p; a punch marks ±1 by the sign track (bit 6): set when the
/// syllable has no negative trits.
pub fn syllable_to_row(t: &analemma::tryte::Tryte) -> u8 {
    let mut row = 0u8;
    for p in 1..=6usize {
        let el = 7 - p; // 1-based element index
        if t.trits[el - 1] != analemma::trit::Trit::Z {
            row |= 1 << (p - 1);
        }
    }
    let has_neg = t.trits.iter().any(|&x| x == analemma::trit::Trit::N);
    if !has_neg {
        row |= 1 << 6;
    }
    row
}

/// §6: tape row -> syllable (inverse of `syllable_to_row`).
pub fn row_to_syllable(row: u8) -> analemma::tryte::Tryte {
    use analemma::trit::Trit;
    let sign = if row & (1 << 6) != 0 { Trit::P } else { Trit::N };
    let mut trits = [Trit::Z; 6];
    for p in 1..=6usize {
        if row & (1 << (p - 1)) != 0 {
            trits[6 - p] = sign; // element 7−p (1-based) -> trits[6−p]
        }
    }
    analemma::tryte::Tryte { trits }
}


/// Teletype: a character-level device over the §6.1 charset layer.
///
/// Input characters are encoded to syllable codes and presented as §6
/// tape rows; output rows are decoded back to characters (CR/LF become
/// '\n'). Unencodable input characters are skipped.
pub struct Teletype {
    input: Vec<char>,
    pos: usize,
    pub output: String,
}

impl Teletype {
    /// Only characters present in the §6.1 alphabet (via
    /// `charset::encode_teletype`) enter the input queue; everything
    /// else is ignored silently.
    pub fn new(input: &str) -> Self {
        let input: Vec<char> = input.chars()
            .filter(|&c| crate::charset::is_representable(c))
            .collect();
        Teletype { input, pos: 0, output: String::new() }
    }

    /// Type more characters (appended to the input queue, filtered).
    pub fn type_str(&mut self, s: &str) {
        self.input.extend(s.chars().filter(|&c| crate::charset::is_representable(c)));
    }
}

impl ByteDevice for Teletype {
    fn read(&mut self) -> Option<u8> {
        while let Some(&c) = self.input.get(self.pos) {
            self.pos += 1;
            if let Some(code) = crate::charset::encode_teletype(c) {
                return Some(syllable_to_row(&analemma::tryte::Tryte::from_i16(code)));
            }
        }
        None // nothing (more) to read: the group keeps waiting
    }

    fn write(&mut self, row: u8) {
        let code = row_to_syllable(row).to_i16();
        match crate::charset::decode_char(code) {
            Some(c) => self.output.push(c),
            None => match crate::charset::control_name(code) {
                Some("Возврат каретки") | Some("Перевод строки") => self.output.push('\n'),
                Some("Пропуск") => self.output.push(' '),
                _ => {} // colours, register shifts, tab: no visible effect
            },
        }
    }
}
