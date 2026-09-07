//! setun70 — runnable entry point.
//!
//! Usage:
//!   cargo run --release [input-tape] [max-steps]
//!
//! Without arguments it boots the demo firmware (ROM page 12): reads 6 and 7,
//! stores their sum at m[0, 20], and stops via FINISH. Every executed step is
//! traced. If an input tape file is given, its bytes become the group -1
//! reader device (device 1) — attach your own firmware to consume it.

use std::cell::RefCell;
use std::rc::Rc;

use analemma::trit::Trit;
use setun70::asm::{assemble, trace_state};
use setun70::io::TapeReader;
use setun70::machine::Machine;

fn main() {
    let argv: Vec<String> = std::env::args().collect();
    if argv.get(1).map(|s| s.as_str()) == Some("echo") {
        run_echo(argv.get(2).map(|s| s.as_str()));
        return;
    }
    if argv.len() == 1 {
        // The machine greets on a bare launch.
        run_echo(Some("Я СЕТУНЬ 70"));
        return;
    }
    let mut args = std::env::args().skip(1);
    let tape_path = args.next();
    let max_steps: u64 = args.next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1000);

    let mut m = Machine::new();
    m.power_on();
    // h[0] := 12: the demo firmware lives on ROM page 12.
    m.regs.set_h(Trit::Z, 12);

    // Firmware (assembly text, stage 8 syntax):
    //   data:    m[12,10]=6, m[12,11]=7, m[12,12]=Ref1 h[-]@20
    //   program: Ref1 h[0]@10 ; Ref1 h[0]@11 ; S+T ; Ref1 h[0]@12 ; W=S ; MACRO @-27
    let slot = |a: i8| (a - -40) as usize;
    let mut img = setun70::memory::Page::zeroed();
    img.syllables[slot(10)] = setun70::asm::assemble_syllable("=6").unwrap();
    img.syllables[slot(11)] = setun70::asm::assemble_syllable("=7").unwrap();
    img.syllables[slot(12)] = setun70::asm::assemble_syllable("Ref1 h[-]@20").unwrap();
    let program = assemble(&[
        "Ref1 h[0]@10",
        "Ref1 h[0]@11",
        "S+T",
        "Ref1 h[0]@12",
        "W=S",
        "MACRO @-27", // FINISH (interrupt mode)
    ]).unwrap();
    for (i, t) in program.iter().enumerate() {
        img.syllables[slot(-40 + i as i8)] = *t;
    }
    m.mem.load_rom_page(12, img).unwrap();

    if let Some(path) = tape_path {
        let data = std::fs::read(&path).unwrap_or_else(|e| {
            eprintln!("cannot read tape file {}: {}", path, e);
            std::process::exit(2);
        });
        eprintln!("tape attached: {} bytes -> group -1, device 1", data.len());
        m.io.attach(-1, 1, Rc::new(RefCell::new(TapeReader::new(data))));
    }

    println!("--- power on: mode=Interrupt (ch,ca)=(12,-40), h[0]=12");
    let outcome = m.run_with(max_steps, |mach, step| {
        println!("[{:>4}] {}", step, trace_state(mach));
    });
    println!("--- outcome: {:?}", outcome);
    println!("--- m[0,20] = {:?} (expect 13)",
        m.mem.read(0, 20).map(|t| t.to_i16()));
}

/// Teletype echo demo: the machine arms the keyboard read once, idles,
/// and the interrupt handlers echo every character to the printer
/// (COPYG1 + LOADG2 run back to back inside the read-completion handler,
/// so the window is consumed before the next interrupt can overwrite it).
fn run_echo(input_arg: Option<&str>) {
    use setun70::io::Teletype;
    use setun70::machine::Machine;
    use analemma::trit::Trit;

    let input = match input_arg {
        Some(s) => s.to_string(),
        None => {
            eprint!("type a line for the teletype: ");
            use std::io::Write;
            std::io::stderr().flush().unwrap();
            let mut line = String::new();
            std::io::stdin().read_line(&mut line).unwrap();
            line.trim_end().to_string()
        }
    };

    let mut m = Machine::new();
    m.power_on();
    m.regs.set_mode(setun70::registers::Mode::Macro);
    m.regs.set_ch(0);
    m.regs.set_ca(-40);
    m.regs.set_ph(1);
    m.regs.set_h(Trit::Z, 0);
    m.regs.u[1].set_trit(0, Trit::P);  // u[0]: PASS
    m.regs.u[1].set_field(2, 3, 2);    //       device 2

    let write_p0 = |m: &mut Machine, addr: i8, v: i16| {
        m.mem.write(0, addr, analemma::tryte::Tryte::from_i16(v)).unwrap();
    };
    write_p0(&mut m, 10, 26);    // u[-1] = (TAKE, dev 1)
    write_p0(&mut m, 13, -38);   // idle target

    let prog = setun70::asm::assemble(&[
        "Ref1 h[0]@10",
        "LOADU1",
        "T=C",
        "Ref1 h[0]@13",
        "C=T",
    ]).unwrap();
    for (i, t) in prog.iter().enumerate() {
        m.mem.write(0, -40 + i as i8, *t).unwrap();
    }

    let mut img = setun70::memory::Page::zeroed();
    let w13 = |img: &mut setun70::memory::Page, addr: i8, t: analemma::tryte::Tryte| {
        img.syllables[(addr - -40) as usize] = t;
    };
    let one = |s: &str| setun70::asm::assemble(&[s]).unwrap()[0];
    w13(&mut img, -39, analemma::tryte::Tryte::from_i16(-30)); // read vector
    w13(&mut img, -38, analemma::tryte::Tryte::from_i16(-20)); // write vector
    w13(&mut img, -13, one("C=T"));        // dispatcher
    w13(&mut img, -30, one("EXCHP"));
    w13(&mut img, -29, one("COPYG1"));
    w13(&mut img, -28, one("LOADG2"));
    w13(&mut img, -27, one("Ref1 h[0]@10")); // pa balance inside the handler
    w13(&mut img, -26, one("RETNMC"));
    w13(&mut img, -20, one("EXCHP"));
    w13(&mut img, -19, one("RETNMC"));
    m.mem.load_rom_page(13, img).unwrap();

    let tty = Rc::new(RefCell::new(Teletype::new(&format!("{}\n", input))));
    let printer = tty.clone();
    m.io.attach(-1, 1, tty);
    m.io.attach(0, 2, printer.clone());

    let _ = m.run(2_000_000); // the input runs out; the machine idles on
    print!("{}", printer.borrow().output);
}
