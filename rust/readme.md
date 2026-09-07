# setun70

Emulator of the Setun-70 ternary computer, written directly from the
primary source: «Структура и алгоритм функционирования малой
вычислительной машины "Сетунь-70"» (rotaprint 27-VT (417), MSU, 1970).

Depends on the architecture-neutral core `analemma` (path dependency
`../analemma-core`, default width 6 trits).

## Implemented (stage 1)

- **Level-1 memory** (`memory`): 27 pages × 81 syllables; RAM pages
  −4..=4 (read/write), ROM pages −13..=−5 and 5..=13 (read-only, loadable
  images); senior-first 1..=3 syllable access matching the address-syllable
  semantics; whole-page copy (exchange primitive).
- **Register file** (`registers`): c (mode c1, program page ch, syllable
  address ca), p (stack page ph, window pa), R, Y, v, e, k (k1/k2/k3/ka/ko
  with the operational-syllable decode), w (interrupt causes per §5.2),
  hf, q/g/u/a (I/O placeholders), start; MSB-first field accessors.
- **Machine** (`machine`): power-on sequence (START block), the stack
  window aliases T/S/t as views over `m[ph, 3pa−1:3pa+1]`, instruction
  fetch `k := m[ch, ca]`.

## Implemented (stage 2)

- The CYCLE loop: fetch `k := m[ch, ca]`, dispatch operational vs address
  syllables, INCRC with the v[9] page-exhausted request.
- REFSYL push (T := 0, senior-first copy of 1..=3 syllables from h[k2]).
- The whole BASIC group B1..B27 with the 36-trit double-word helpers.
- Traps matching §5.2 (stack overflow/exhaustion, operand crossing a
  page boundary, forbidden syllable) and FINISH for MACRO in interrupt mode.

## Implemented (stage 3)

- The MACRO dispatcher (§4): state save via the c-area rotation, mode
  switch to macro, entry at page −13 (OCR-literal reading; the macro
  address comes from ka). MACRO in interrupt mode stops the machine.
- The IR interrupt entry: same rotation, mode := interrupt, program
  continues at (13, −13); the vector for cause w is the syllable m[13, w]
  (page 13 holds the vector table at addresses −40..−28).
- Polling of the `v` register in user/macro mode (§4 CYCLE); all §5.2
  conditions (overflows, forbidden syllables, operand crossing a page)
  enter IR inside `step()`.

## Implemented (stage 4)

- The whole SPEC group S1..S27:
  - COPYG/LOADG (S1–S3, S25–S27): g-buffer transfer with the §6 tape
    correspondence (position i ↔ element 7−i; punch = ±1 by the sign
    track); exact for single-sign syllables by design of §6.
  - COPYF/LOADF/LOADQ (S4–S9): page-granular exchange with the level-2
    drum (3 kinds × 6561 pages); SUIT raises its v-request at once.
  - COPYP/EXCHP/LOADP (S10–S12): stack-pointer save/restore/exchange.
  - COPYMC/RETNMC/LOADMC (S13–S15): macro context juggling on c.
  - LOADH (S16–S18), LOADU (S19–S21).

## Implemented (stage 5)

- Peripheral layer (`io.rs`): byte devices (tape reader, punch, or any
  `ByteDevice`) attached to (group, number); `Machine::service_io()`
  emulates the INOUT process — PASS/TAKE by u[i,1], device number from
  u[i,3:4], v[i+3] on completion, wait-at-tape-end on TAKE.
- The §6 syllable ↔ tape-row codec (`syllable_to_row` / `row_to_syllable`).
- The `run(max_steps)` loop (step + I/O service until FINISH, trap, or
  budget exhaustion) and an end-to-end POLIZ program test
  (push 6, push 7, S+T, W=S store, idle loop).

## Firmware (stage 6)

`test_power_on_minimal_firmware` exercises the real power-on path
(START block: interrupt mode, program at ROM page 12):

- **Part 1** — the pure power-on: with h reset to 0 every operand fetch
  would read empty RAM, so the first syllables may only use stack ops and
  `T=C` constants; a MACRO syllable in interrupt mode stops the machine
  (FINISH per §5.3).
- **Part 2** — the firmware body with h primed: push 6, push 7 from ROM,
  `S+T`, `W=S` into RAM, FINISH. The reset values of h are not stated in
  the description; the historical machine either had them wired or the
  program equipment synthesised them (bootstrap via `T=C` and arithmetic).
  This is flagged as an open question for the programming manuals.

## Debugging (stage 7)

- `asm.rs`: syllable decoder and formatter. Mnemonics per §4 (BAS_NAMES /
  SPEC_NAMES, 27+27, completeness-tested). Address syllables print as
  `Ref<len> h[<k2>]@<ka>` — "Ref" after REFSYL, the reference syllable of
  the description.
- `dump_page(&Machine, page)`: address, value, trits (`ZPZZPN`), nonary
  (via the analemma table), decoded syllable; ROM pages marked.
- `trace_state(&Machine)`: one line — mode, (ch, ca), decoded k, (ph, pa),
  T/S values, R/Y/e/w, the v bitmap.
- `run_with(max_steps, hook)`: the run loop with a per-step callback for
  tracing.

## Running

```bash
cargo run --release                      # the machine greets: prints "Я СЕТУНЬ 70"
                                         # (teletype echo program, §6.1 charset)
cargo run --release tape.bin 5000        # attach tape.bin to group -1, dev 1

# Teletype echo: the machine reads the string from the teletype keyboard
# and prints it back (Сетунь-70 charset, §6.1):
cargo run --release -- echo "УРА, Заработало! заведут баркас и через 42 к-а-к! ЭТО ТОЧНО... ~{}q"
# prints: УРА, З!     42 --! ЭТО ТОЧНО...
cargo run --release -- echo            # prompts for a line
```

The demo firmware (ROM page 12) computes 6 + 7 and stores 13 at m[0, 20],
then stops via FINISH; every executed step is traced with `trace_state`.

## Assembly (stage 8)

`asm::assemble_syllable` is the exact inverse of `format_syllable`:
`=V` (data), the 54 mnemonics, `MACRO @ka` (ka ∈ −40..=−14), and
`Ref<len> h[<sym>]@<ka>` (sym ∈ {`-`, `0`, `+`}; `Ref2 h[0]@..` is
rejected — the k1 = 0 ∧ k2 = 0 pattern is reserved). `asm::assemble`
assembles a whole program; `test_firmware_written_as_text` runs a ROM
firmware composed from assembly text end to end.

## Not yet implemented

- WATCH (periodic clock) and ATTENT (panel button) as auto-raised
  requests — write the v register directly for now.
- STOP as an external stop signal.
- Character-level decoding of §6 codes (register letters etc.) — only
  the raw row codec exists.

## Decoding assumptions (OCR-damaged spots)

- Operand length is encoded as `k1 + 2` (N→1, Z→2, P→3 syllables).
- B1 "LST" is a long shift of (S,Y) by t trits; B2 "COT" compares
  |S| against 3^17/2; B3 "XNN" normalises (T,Y) with e := e + i.
  All three are flagged in `machine.rs` for verification against the
  programming manuals.
- LOADU (S19–S21) copies the senior syllable's low 4 trits into u
  positionally ("u[l, 1:4] := t" read as a trit copy, not a value
  conversion).

## Conventions

- Trit indices are 0-based (the description is 1-based).
- In any trit field `trits[from..=to]` the most significant trit is
  `from` (§3.2); ascending syllable addresses go senior → junior.
