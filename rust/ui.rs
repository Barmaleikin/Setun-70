use crate::core::VM;
use crate::util::{read_line_trimmed};
use std::io::Write;

pub fn show_help() {
    println!(" - Available commands:");
    println!("");
    println!("   [G] Run program");
    println!("   [H] Show command reference");
    println!("   [M] Show memory dump");
    println!("   [R] Show registers");
    println!("   [S] Step");
    println!("   [X] Exit");
    println!("");
    print!("> ");
    let _ = std::io::stdout().flush();
}

pub fn interactive(mut vm: VM) {
    println!("");
    println!(" Welcome to Setun 70 computer emulator");
    println!("");
    println!(" - No arguments provided");
    println!(" - Entering interactive debug mode");
    show_help();

    loop {
        let input = match read_line_trimmed() {
            Some(s) => s,
            None => break,
        };

        match input.as_str() {
            "G" | "g" => {
                println!("{}", input);
                vm.m_state_running = true;
                while vm.m_state_running {
                    let ko = vm.fetch();
                    let _ = vm.execute(ko);
                    vm.process(0);
                    vm.m_state_running = false;
                }
                print!("> ");
                let _ = std::io::stdout().flush();
            }
            "H" | "h" => {
                show_help();
                let _ = std::io::stdout().flush();
            }
            "M" | "m" => {
                println!("{}", input);
                println!(" RAM page center values:");
                for i in 0..5 {
                    let v_idx = VM::val_index(i - 2);
                    print!("{} ", vm.values[v_idx].val_binary);
                }
                println!();
                print!("> ");
                let _ = std::io::stdout().flush();
            }
            "R" | "r" => {
                vm.show_registers();
                print!("> ");
                let _ = std::io::stdout().flush();
            }
            "S" | "s" => {
                println!();
                let ko = vm.fetch();
                let _ = vm.execute(ko);
                vm.process(0);
                println!();
                print!("> ");
                let _ = std::io::stdout().flush();
            }
            "X" | "x" => break,
            other => {
                println!("Unrecognized command: {}", other);
                print!("> ");
                let _ = std::io::stdout().flush();
            }
        }
    }
}
