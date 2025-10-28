use std::io::{self, Write};
use std::fmt;

const VAL_HALF_RANGE: i16 = 364;
const VAL_LOW_BOUNDARY: i16 = -VAL_HALF_RANGE;
const VAL_HIGH_BOUNDARY: i16 = VAL_HALF_RANGE;
const VAL_FULL_RANGE: usize = (VAL_HALF_RANGE as isize * 2 + 1) as usize;

const PAGE_HALF_RANGE: i32 = 40;
const PAGE_FULL_RANGE: usize = (PAGE_HALF_RANGE as isize * 2 + 1) as usize;

#[derive(Clone)]
struct AnValue {
    val_binary: i16,
    val_trinary: i16,

    txt_binary: String,
    txt_trinary: String,
    txt_nonary: String,
    txt_decimal: String,

    is_zero: bool,
    is_positive: bool,
    is_even: bool,
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
struct Context {
    m_exception_rised: bool,
    programm_pos: usize,
    programm_page: usize,
    stack_page: usize,
    stack_pos: usize,
    opcode: u8,
}

enum MachineMode {
    BASIC,
    SYSTEM,
    USER,
}

struct VM {
    values: Vec<AnValue>,
    stack: Vec<usize>,

    reg_c: usize,
    part_c1: usize,
    part_ca: usize,
    part_ch: usize,

    reg_k: usize,
    part_k1: usize,
    part_ka: usize,

    ptr_p: usize,
    ptr_top: usize,
    ptr_t: usize,
    ptr_subtop: usize,

    reg_e: usize,
    reg_r: usize,
    reg_y: usize,

    m_state_running: bool,
    context: Context,

    ram_page: Vec<i32>,
    rom_page: Vec<i32>,
    page_ram: [Option<Vec<i32>>; 9],
    page_rom: [Option<Vec<i32>>; 18],
}

impl VM {
    fn new() -> Self {
        let mut values = vec![AnValue::default(); VAL_FULL_RANGE];

        for v in VAL_LOW_BOUNDARY..=VAL_HIGH_BOUNDARY {
            let idx = Self::val_index(v);
            let is_zero = v == 0;
            let is_positive = v > 0;
            let is_even = v % 2 == 0;

            values[idx].val_binary = v;
            values[idx].val_trinary = 0;

            let s = v.to_string();
            values[idx].txt_binary = s.clone();
            values[idx].txt_trinary = s.clone();
            values[idx].txt_nonary = s.clone();
            values[idx].txt_decimal = s;

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
            ptr_op: center,
            ptr_t: center,
            ptr_subtop: center,
            reg_e: center,
            reg_r: center,
            reg_y: center,
            m_state_running: false,
            context: Context::default(),
            ram_page: vec![0; PAGE_FULL_RANGE],
            rom_page: vec![0; PAGE_FULL_RANGE],
            page_ram: Default::default(),
            page_rom: Default::default(),
        }
    }

    fn val_index(v: i16) -> usize {
        (v as isize + VAL_HALF_RANGE as isize) as usize
    }

    fn wrap_value(v: i32) -> i16 {
        let n = VAL_FULL_RANGE as i32;
        let shifted = v + VAL_HALF_RANGE as i32;
        let wrapped = ((shifted % n) + n) % n;
        (wrapped - VAL_HALF_RANGE as i32) as i16
    }

    fn push(&mut self, v: i16) {
        let idx = VM::val_index(v);
        self.stack.push(idx);
    }

    fn peek(&self) -> Option<&AnValue> {
        self.stack.last().and_then(|&idx| self.values.get(idx))
    }

    fn pop(&mut self) -> Option<AnValue> {
        self.stack.pop().and_then(|idx| self.values.get(idx).cloned())
    }

    fn op_basic(&mut self, ko: usize) {
        #[allow(non_camel_case_types)]
        enum OpBasic {
            lst = 0, cot, xnn, dow, brt, hmp, t_sub_e, e_setto_t, t_add_d,
            c_lessthan_t, c_equalto_t, c_greaterthan_t, jsr, r_setto_t, c_setto_t, t_setto_w, yft, w_setto_s,
            smt, y_setto_t, sat, s_sub_t, tdn, s_add_t, lbt, l_mul_t, lht
        }

        let name_basic = [
            "LST","COT","XNN","DOW","BRT","JMP","T-E","E=T","T+E",
            "CLT","CET","CGT","JSR","R=T","C=T","T=W","YFT","W=S",
            "SMT","Y=T","SAT","S-T","TDN","S+T","LBT","L*T","LHT"
        ];

        match ko {
            x if x == OpBasic::yft as usize => {
                self.reg_y = Self::val_index(0);
            }
            x if x == OpBasic::y_setto_t as usize => {
                if let Some(top) = self.pop() {
                    let idx = Self::val_index(top.val_binary);
                    self.reg_y = idx;
                }
            }
            x if x == OpBasic::s_sub_t as usize => {
                let a = self.pop().map(|v| v.val_binary).unwrap_or(0);
                let b = self.pop().map(|v| v.val_binary).unwrap_or(0);
                let result = VM::wrap_value(a as i32 - b as i32);
                self.push(result);
            }
            x if x == OpBasic::tdn as usize => {
                if let Some(top) = self.pop() {
                    let temp = -top.val_binary;
                    self.push(temp);
                }
            }
            x if x == OpBasic::s_add_t as usize => {
                let a = self.pop().map(|v| v.val_binary).unwrap_or(0);
                let b = self.pop().map(|v| v.val_binary).unwrap_or(0);
                let result = VM::wrap_value(a as i32 + b as i32);
                self.push(result);
            }
            _ => { /* not implemented */ }
        }

        let name = name_basic.get(ko).unwrap_or(&"UNKNOWN");
        println!(" - [BASIC] : {}", name);
    }

    fn op_macro(&self, ko: usize) {
        let name_macro = [
            "MACRO 1","MACRO 2","MACRO 3","MACRO 4","MACRO 5","MACRO 6","MACRO 7","MACRO 8","MACRO 9",
            "MACRO 10","MACRO 11","MACRO 12","MACRO 13","MACRO 14","MACRO 15","MACRO 16","MACRO 17","MACRO 18",
            "MACRO 19","MACRO 20","MACRO 21","MACRO 22","MACRO 23","MACRO 24","MACRO 25","MACRO 26","MACRO 27"
        ];
        println!(" - [MACRO] : {}", name_macro.get(ko).unwrap_or(&"UNKNOWN"));
    }

    fn op_system(&self, ko: usize) {
        let name_system = [
            "COPYG1","COPYG2","COPYG3","COPYF1","COPYF2","COPYF3","LOADQ1","LOADQ2","LOADQ3",
            "COPYP","EXCHP","LOADP","COPYMC","RETNMC","LOADMC","LOADH1","LOADH2","LOADH3",
            "LOADU1","LOADU2","LOADU3","LOADF1","LOADF2","LOADF3","LOADG1","LOADG2","LOADG3"
        ];
        println!(" - [SYSTEM] : {}", name_system.get(ko).unwrap_or(&"UNKNOWN"));
    }

    fn process(&mut self, _ko: usize) {
        if self.context.m_exception_rised {
            self.context.m_exception_rised = false;
        }
    }

    fn execute(&mut self, ko: usize) -> Result<(), ()> {
        match self.context.opcode {
            0 => { self.op_basic(ko); }
            1 => { self.op_system(ko); }
            2 => { self.op_macro(ko); }
            _ => {
                self.context.m_exception_rised = true;
                println!(" - [UNDEFINED]");
                return Err(());
            }
        }
        Ok(())
    }

    fn fetch(&mut self) -> usize {
        self.context.opcode = 0;
        0
    }

    fn show_registers(&self) {
        println!();
        println!("  c : {}", self.values[self.reg_c].txt_trinary);
        println!("  c1: {}", self.values[self.part_c1].txt_trinary);
        println!("  ca: {}", self.values[self.part_ca].txt_trinary);
        println!("  ch: {}", self.values[self.part_ch].txt_trinary);
        println!();
        println!("  k : {}", self.values[self.reg_k].txt_trinary);
        println!("  k1: {}", self.values[self.part_k1].txt_trinary);
        println!("  ka: {}", self.values[self.part_ka].txt_trinary);
        println!();
        println!("  p : {}", self.values[self.ptr_p].txt_trinary);
        println!("  T : {}", self.values[self.ptr_top].txt_trinary);
        println!("  t : {}", self.values[self.ptr_t].txt_trinary);
        println!("  S : {}", self.values[self.ptr_subtop].txt_trinary);
        println!();
        println!("  e : {}", self.values[self.reg_e].txt_trinary);
        println!("  R : {}", self.values[self.reg_r].txt_trinary);
        println!("  Y : {}", self.values[self.reg_y].txt_trinary);
        println!();
    }
}

fn read_line_trimmed() -> Option<String> {
    let mut input = String::new();
    match io::stdin().read_line(&mut input) {
        Ok(0) => None,
        Ok(_) => Some(input.trim().to_string()),
        Err(_) => None,
    }
}

fn showhelp() {
    println!(" - Available commands:");
    println!("   [G] Run program");
    println!("   [H] Show command reference");
    println!("   [M] Show memory dump");
    println!("   [R] Show registers");
    println!("   [S] Step");
    println!("   [X] Exit");
    print!("> ");
    let _ = io::stdout().flush();
}

fn main() {
    let mut vm = VM::new();

    println!(" - No arguments provided");
    println!(" - Entering interactive debug mode");
    showhelp();
    
    loop {
        let input = match read_line_trimmed() {
            Some(s) => s,
            None => break,
        };

        match input.as_str() {
            "G" => {
                println!("{}", input);
                vm.m_state_running = true;
                while vm.m_state_running {
                    let ko = vm.fetch();
                    let _ = vm.execute(ko);
                    vm.process(0);
                    vm.m_state_running = false;
                }
                print!("> ");
                let _ = io::stdout().flush();
            }
            "H" => {
                showhelp();
                let _ = io::stdout().flush();
            }
            "M" => {
                println!("{}", input);
                println!(" RAM page center values:");
                for i in 0..5 {
                    let v_idx = VM::val_index(i - 2);
                    print!("{} ", vm.values[v_idx].val_binary);
                }
                println!();
                print!("> ");
                let _ = io::stdout().flush();
            }
            "R" => {
                vm.show_registers();
                print!("> ");
                let _ = io::stdout().flush();
            }
            "S" => {
                println!();
                let ko = vm.fetch();
                let _ = vm.execute(ko);
                vm.process(0);
                println!();
                print!("> ");
                let _ = io::stdout().flush();
            }
            "X" => break,
            other => {
                println!("Unrecognized command: {}", other);
                print!("> ");
                let _ = io::stdout().flush();
            }
        }
    }
}
