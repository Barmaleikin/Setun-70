use std::io::{self, Write};
use std::cmp::max;

pub fn wrap_value(v: i32) -> i16 {
    const VAL_HALF_RANGE_I32: i32 = crate::core::VAL_HALF_RANGE as i32;
    let n = crate::core::VAL_FULL_RANGE as i32;
    let shifted = v + VAL_HALF_RANGE_I32;
    let wrapped = ((shifted % n) + n) % n;
    (wrapped - VAL_HALF_RANGE_I32) as i16
}

pub fn read_line_trimmed() -> Option<String> {
    let mut input = String::new();
    match io::stdin().read_line(&mut input) {
        Ok(0) => None,
        Ok(_) => Some(input.trim().to_string()),
        Err(_) => None,
    }
}

pub fn print_columns(items: &[(String, String)], columns: usize, gap: usize) {
    if columns == 0 { return; }
    let n = items.len();
    let rows = (n + columns - 1) / columns;

    let mut grid: Vec<Vec<Option<usize>>> = vec![vec![None; rows]; columns];
    for i in 0..n {
        let col = i % columns;
        let row = i / columns;
        grid[col][row] = Some(i);
    }

    let mut col_widths = vec![0usize; columns];
    for c in 0..columns {
        let mut w = 0;
        for r in 0..rows {
            if let Some(i) = grid[c][r] {
                let (ref h, ref v) = items[i];
                w = max(w, h.len() + v.len());
            }
        }
        col_widths[c] = w;
    }

    for r in 0..rows {
        for c in 0..columns {
            match grid[c][r] {
                Some(i) => {
                    let (ref h, ref v) = items[i];
                    print!("{}{}", h, v);
                    let printed = h.len() + v.len();
                    if col_widths[c] > printed {
                        print!("{}", " ".repeat(col_widths[c] - printed));
                    }
                    if c + 1 < columns {
                        print!("{}", " ".repeat(gap));
                    }
                }
                None => {
                    if c + 1 < columns {
                        print!("{}", " ".repeat(col_widths[c] + gap));
                    }
                }
            }
        }
        println!();
    }
    let _ = std::io::stdout().flush();
}

pub fn to_binary_string(v: i16) -> String {
    if v == 0 { return "0".to_string(); }
    let mut n = v;
    let negative = n < 0;
    if negative { n = -n; }
    let mut s = String::new();
    while n > 0 {
        s.push(if n & 1 == 1 { '1' } else { '0' });
        n = (n as u16 >> 1) as i16;
    }
    if negative { s.push('-'); }
    s.chars().rev().collect()
}

pub fn to_balanced_ternary(mut v: i16) -> String {
    if v == 0 { return "O".to_string(); } // 0 => O

    let mut digits: Vec<i8> = Vec::new();
    let mut n = v as i32;
    while n != 0 {
        let mut rem = (n % 3) as i8;
        n /= 3;
        if rem == 2 {
            rem = -1;
            n += 1;
        } else if rem == -2 {
            rem = 1;
            n -= 1;
        } else if rem == -1 || rem == 0 || rem == 1 {
        } else if rem == -0 {
            rem = 0;
        }
        digits.push(rem);
    }

    digits.reverse();
    digits.into_iter().map(|d| match d {
        -1 => 'N',
         0 => 'O',
         1 => 'P',
         _ => '?'
    }).collect()
}

pub fn to_balanced_nonary(mut v: i16) -> String {
    if v == 0 { return "0".to_string(); }

    let mut digits: Vec<i8> = Vec::new();
    let mut n = v as i32;
    while n != 0 {
        let mut rem = (n % 9) as i8;
        n /= 9;
        if rem > 4 {
            rem -= 9;
            n += 1;
        } else if rem < -4 {
            rem += 9;
            n -= 1;
        }
        digits.push(rem);
    }
    digits.reverse();

    let symbols = ['W','X','Y','Z','0','1','2','3','4']; // -4,-3,-2,-1,,0,1,2,3,4
    digits.into_iter().map(|d| {
        let idx = (d + 4) as usize;
        symbols.get(idx).cloned().unwrap_or('?')
    }).collect()
}
