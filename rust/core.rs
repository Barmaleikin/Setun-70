use std::fmt;
use crate::util::wrap_value;

pub const VAL_HALF_RANGE: i16 = 364;
pub const VAL_LOW_BOUNDARY: i16 = -VAL_HALF_RANGE;
pub const VAL_HIGH_BOUNDARY: i16 = VAL_HALF_RANGE;
pub const VAL_FULL_RANGE: usize = (VAL_HALF_RANGE as isize * 2 + 1) as usize;

pub const PAGE_HALF_RANGE: i32 = 40;
pub const PAGE_FULL_RANGE: usize = (PAGE_HALF_RANGE as isize * 2 + 1) as usize;

#[derive(Clone)]
pub struct AnValue {
    pub val_binary: i16,
    pub val_trinary: i16,

    pub txt_binary: String,
    pub txt_trinary: String,
    pub txt_nonary: String,
    pub txt_decimal: String,

    pub is_zero: bool,
    pub is_positive: bool,
    pub is_even: bool,
}

impl Default for AnValue {
    fn default() -> Self {
        Self {
            val_binary: VAL_LOW_BOUNDARY,
            val_trinary: 0,
            txt_binary: "0".into(),
            txt_trinary: "0".into(),
            txt_nonary: "None".into(),
            txt_decimal: "None".into(),
            is_zero: true,
            is_positive: false,
            is_even: false,
        }
    }
}

impl fmt::Display for AnValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.txt_trinary)
    }
}

#[derive(Default)]
pub struct Context {
    pub m_exception_rised: bool,
    pub programm_pos: usize,
    pub programm_page: usize,
    pub stack_page: usize,
    pub stack_pos: usize,
    pub opcode: u8,
}

pub enum MachineMode {
    Basic,
    System,
    User,
}

pub struct VM {
    pub values: Vec<AnValue>,
    pub stack: Vec<usize>,

    pub reg_c: usize,
    pub part_c1: usize,
    pub part_ca: usize,
    pub part_ch: usize,

    pub reg_k: usize,
    pub part_k1: usize,
    pub part_ka: usize,

    pub ptr_p: usize,
    pub ptr_top: usize,
    pub ptr_t: usize,
    pub ptr_subtop: usize,

    pub reg_e: usize,
    pub reg_r: usize,
    pub reg_y: usize,

    pub reg_h1: usize,
    pub reg_h2: usize,
    pub reg_h3: usize,

    pub m_state_running: bool,
    pub context: Context,

    pub ram_page: Vec<i32>,
    pub rom_page: Vec<i32>,
    pub page_ram: [Option<Vec<i32>>; 9],
    pub page_rom: [Option<Vec<i32>>; 18],
}

impl VM {
    pub fn new() -> Self {
        let mut values = vec![AnValue::default(); VAL_FULL_RANGE];

        for v in VAL_LOW_BOUNDARY..=VAL_HIGH_BOUNDARY {
            let idx = Self::val_index(v);
            let is_zero = v == 0;
            let is_positive = v > 0;
            let is_even = v % 2 == 0;

            values[idx].val_binary = v;
            values[idx].val_trinary = 0;

            values[idx].txt_binary = crate::util::to_binary_string(v);
            values[idx].txt_trinary = crate::util::to_balanced_ternary(v);
            values[idx].txt_nonary = crate::util::to_balanced_nonary(v);
            values[idx].txt_decimal = v.to_string();

            values[idx].is_zero = is_zero;
            values[idx].is_positive = is_positive;
            values[idx].is_even = is_even;
        }

        let center = Self::val_index(0);

        VM {
            values,
            stack: Vec::with_capacity(64),
            reg_c: center,
            part_c1: center,
            part_ca: center,
            part_ch: center,
            reg_k: center,
            part_k1: center,
            part_ka: center,
            ptr_p: center,
            ptr_top: center,
            ptr_t: center,
            ptr_subtop: center,
            reg_e: center,
            reg_r: center,
            reg_y: center,
            reg_h1: center,
            reg_h2: center,
            reg_h3: center,
            m_state_running: false,
            context: Context::default(),
            ram_page: vec![0; PAGE_FULL_RANGE],
            rom_page: vec![0; PAGE_FULL_RANGE],
            page_ram: Default::default(),
            page_rom: Default::default(),
        }
    }

    pub fn val_index(v: i16) -> usize {
        (v as isize + VAL_HALF_RANGE as isize) as usize
    }

    pub fn wrap_value(v: i32) -> i16 {
        wrap_value(v)
    }

    pub fn push(&mut self, v: i16) {
        let idx = VM::val_index(v);
        self.stack.push(idx);
    }

    pub fn peek(&self) -> Option<&AnValue> {
        self.stack.last().and_then(|&idx| self.values.get(idx))
    }

    pub fn pop(&mut self) -> Option<AnValue> {
        self.stack.pop().and_then(|idx| self.values.get(idx).cloned())
    }

    pub fn execute(&mut self, ko: usize) -> Result<(), ()> {
        match self.context.opcode {
            0 => crate::ops::basic::BasicOpExecutor::exec(self, ko),
            1 => crate::ops::system::SystemOpExecutor::exec(self, ko),
            2 => crate::ops::r#macro::MacroOpExecutor::exec(self, ko),
            _ => {
                self.context.m_exception_rised = true;
                println!(" - [UNDEFINED]");
                return Err(());
            }
        }
    }

    pub fn fetch(&mut self) -> usize {
        self.context.opcode = 0;
        0
    }

    pub fn process(&mut self, _ko: usize) {
        if self.context.m_exception_rised {
            self.context.m_exception_rised = false;
        }
    }

    pub fn show_registers(&self) {
        use std::usize;
        let pairs = [
            ("  c : ", self.reg_c), ("  c1: ", self.part_c1), ("  ca: ", self.part_ca), ("  ch: ", self.part_ch),
            ("  k : ", self.reg_k), ("  k1: ", self.part_k1), ("  ka: ", self.part_ka), (" ", usize::MAX),
            ("  p : ", self.ptr_p), ("  T : ", self.ptr_top), ("  t : ", self.ptr_t), ("  S : ", self.ptr_subtop),
            ("  e : ", self.reg_e), ("  R : ", self.reg_r), ("  Y : ", self.reg_y), (" ", usize::MAX),
            ("  H1: ", self.reg_h1), ("  H2: ", self.reg_h2), ("  H3: ", self.reg_h3),
        ];

        let items: Vec<(String, String)> = pairs.iter()
            .map(|&(lbl, idx)| {
                if idx == usize::MAX {
                    (" ".to_string(), " ".to_string())
                } else {
                    (lbl.to_string(), self.values[idx].txt_trinary.clone())
                }
            })
            .collect();

        println!();
        crate::util::print_columns(&items, 4, 12);
        println!();
    }
}
