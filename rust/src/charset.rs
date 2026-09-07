//! Character layer of Setun-70 (description §6.1).
//!
//! The internal 6-trit syllable is the character code; this module is the
//! configuration layer mapping codes to printed signs, controls and back.
//! Codes are 3-digit balanced nonary over the §6.1 alphabet
//! W=-4, X=-3, Y=-2, Z=-1, 0..=4 — the same glyph order the analemma
//! table uses for its nonary strings.
//!
//! Every §6.1 sign code is single-sign (all nonzero trits share a sign),
//! so all of them pass the §6 tape-row representation exactly — that is
//! how the historical codes were chosen. `-` (digit minus, nonary 410 =
//! 333) and the hyphen ZWW both print as '-'; `encode_char` maps '-'
//! to the digit minus.

/// Decodes a character code to a printable sign; None for control codes
/// and unassigned codes (see `control_name`).
pub fn decode_char(code: i16) -> Option<char> {
    match code {
        -364 => Some(']'),  // WWW
        -363 => Some(','),  // WWX
        -361 => Some('.'),  // WWZ
        -355 => Some('↑'),  // WXW
        -354 => Some('('),  // WXX
        -352 => Some(')'),  // WXZ
        -351 => Some('×'),  // WX0
        -337 => Some(';'),  // WZW
        -336 => Some('['),  // WZX
        -334 => Some('⎵'),  // WZZ
        -333 => Some('='),  // WZ0
        -328 => Some('>'),  // W0W
        -256 => Some('_'),  // XZW
        -255 => Some('|'),  // XZX
        -121 => Some('-'),  // ZWW
        -120 => Some('%'),  // ZWX
        -118 => Some('W'),  // ZWZ
        -117 => Some('G'),  // ZW0
        -112 => Some('D'),  // ZXW
        -111 => Some(':'),  // ZXX
        -109 => Some('V'),  // ZXZ
        -108 => Some('Z'),  // ZX0
        -94 => Some('I'),  // ZZW
        -93 => Some('J'),  // ZZX
        -91 => Some('≤'),  // ZZZ
        -90 => Some('L'),  // ZZ0
        -85 => Some('≡'),  // Z0W
        -84 => Some('N'),  // Z0X
        -82 => Some('¬'),  // Z0Z
        -81 => Some('⊃'),  // Z00
        -40 => Some('R'),  // 0WW
        -39 => Some('S'),  // 0WX
        -37 => Some('∨'),  // 0WZ
        -36 => Some('U'),  // 0W0
        -31 => Some('F'),  // 0XW
        -30 => Some('<'),  // 0XX
        -28 => Some('≠'),  // 0XZ
        -27 => Some('÷'),  // 0X0
        -13 => Some('‘'),  // 0ZW
        -12 => Some('‚'),  // 0ZX
        -10 => Some('!'),  // 0ZZ
        -9 => Some('∧'),  // 0Z0
        -4 => Some('≥'),  // 00W
        -3 => Some('◊'),  // 00X
        -1 => Some('Q'),  // 00Z
        1 => Some('Я'),  // 001
        3 => Some('Ю'),  // 003
        4 => Some('Э'),  // 004
        9 => Some('Ь'),  // 010
        10 => Some('Ы'),  // 011
        12 => Some('Щ'),  // 013
        13 => Some('Ш'),  // 014
        27 => Some('Ч'),  // 030
        28 => Some('Ц'),  // 031
        30 => Some('Х'),  // 033
        31 => Some('Ф'),  // 034
        36 => Some('У'),  // 040
        37 => Some('Т'),  // 041
        39 => Some('С'),  // 043
        40 => Some('Р'),  // 044
        81 => Some('П'),  // 100
        82 => Some('О'),  // 101
        84 => Some('Н'),  // 103
        85 => Some('М'),  // 104
        90 => Some('Л'),  // 110
        91 => Some('К'),  // 111
        93 => Some('Й'),  // 113
        94 => Some('И'),  // 114
        108 => Some('З'),  // 130
        109 => Some('Ж'),  // 131
        111 => Some('Е'),  // 133
        112 => Some('Д'),  // 134
        117 => Some('Г'),  // 140
        118 => Some('В'),  // 141
        120 => Some('Б'),  // 143
        121 => Some('А'),  // 144
        255 => Some('*'),  // 313
        256 => Some('¯'),  // 314
        328 => Some('/'),  // 404
        333 => Some('-'),  // 410
        334 => Some('+'),  // 411
        336 => Some('9'),  // 413
        337 => Some('8'),  // 414
        351 => Some('7'),  // 430
        352 => Some('6'),  // 431
        354 => Some('5'),  // 433
        355 => Some('4'),  // 434
        360 => Some('3'),  // 440
        361 => Some('2'),  // 441
        363 => Some('1'),  // 443
        364 => Some('0'),  // 444
        _ => None,
    }
}

/// Names of the non-printing control codes (§6.1).
pub fn control_name(code: i16) -> Option<&'static str> {
    match code {
        270 => Some("Пропуск"),  // 330
        271 => Some("Нижний регистр"),  // 331
        273 => Some("Табулятор"),  // 333
        274 => Some("Верхний регистр"),  // 334
        279 => Some("Черный цвет"),  // 340
        280 => Some("Красный цвет"),  // 341
        282 => Some("Перевод строки"),  // 343
        283 => Some("Возврат каретки"),  // 344
        _ => None,
    }
}

/// Encodes a printable sign back to its code ('-' -> 333, the digit minus).
pub fn encode_char(c: char) -> Option<i16> {
    match c {
        ']' => Some(-364),
        ',' => Some(-363),
        '.' => Some(-361),
        '↑' => Some(-355),
        '(' => Some(-354),
        ')' => Some(-352),
        '×' => Some(-351),
        ';' => Some(-337),
        '[' => Some(-336),
        '⎵' => Some(-334),
        '=' => Some(-333),
        '>' => Some(-328),
        '_' => Some(-256),
        '|' => Some(-255),
        '%' => Some(-120),
        'W' => Some(-118),
        'G' => Some(-117),
        'D' => Some(-112),
        ':' => Some(-111),
        'V' => Some(-109),
        'Z' => Some(-108),
        'I' => Some(-94),
        'J' => Some(-93),
        '≤' => Some(-91),
        'L' => Some(-90),
        '≡' => Some(-85),
        'N' => Some(-84),
        '¬' => Some(-82),
        '⊃' => Some(-81),
        'R' => Some(-40),
        'S' => Some(-39),
        '∨' => Some(-37),
        'U' => Some(-36),
        'F' => Some(-31),
        '<' => Some(-30),
        '≠' => Some(-28),
        '÷' => Some(-27),
        '‘' => Some(-13),
        '‚' => Some(-12),
        '!' => Some(-10),
        '∧' => Some(-9),
        '≥' => Some(-4),
        '◊' => Some(-3),
        'Q' => Some(-1),
        'Я' => Some(1),
        'Ю' => Some(3),
        'Э' => Some(4),
        'Ь' => Some(9),
        'Ы' => Some(10),
        'Щ' => Some(12),
        'Ш' => Some(13),
        'Ч' => Some(27),
        'Ц' => Some(28),
        'Х' => Some(30),
        'Ф' => Some(31),
        'У' => Some(36),
        'Т' => Some(37),
        'С' => Some(39),
        'Р' => Some(40),
        'П' => Some(81),
        'О' => Some(82),
        'Н' => Some(84),
        'М' => Some(85),
        'Л' => Some(90),
        'К' => Some(91),
        'Й' => Some(93),
        'И' => Some(94),
        'З' => Some(108),
        'Ж' => Some(109),
        'Е' => Some(111),
        'Д' => Some(112),
        'Г' => Some(117),
        'В' => Some(118),
        'Б' => Some(120),
        'А' => Some(121),
        '*' => Some(255),
        '¯' => Some(256),
        '/' => Some(328),
        '-' => Some(333),
        '+' => Some(334),
        '9' => Some(336),
        '8' => Some(337),
        '7' => Some(351),
        '6' => Some(352),
        '5' => Some(354),
        '4' => Some(355),
        '3' => Some(360),
        '2' => Some(361),
        '1' => Some(363),
        '0' => Some(364),
        _ => None,
    }
}

/// True for control codes (non-printing but assigned).
pub fn is_control(code: i16) -> bool {
    control_name(code).is_some()
}

/// Teletype-level encoding: the §6.1 signs plus the two conventional
/// control mappings — space as Пропуск (nonary 330 = 270) and newline as
/// Возврат каретки (nonary 344 = 283). Everything outside the alphabet
/// (lowercase letters, signs not in the table) is NOT representable;
/// devices filter input through this function.
pub fn encode_teletype(c: char) -> Option<i16> {
    match c {
        '\n' => Some(283),
        ' ' => Some(270),
        _ => encode_char(c),
    }
}

/// True if the character can be typed/printed on the Setun-70 teletype.
pub fn is_representable(c: char) -> bool {
    encode_teletype(c).is_some()
}
