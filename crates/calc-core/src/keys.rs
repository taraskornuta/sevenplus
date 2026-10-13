//! Win7 calculator keyboard map for printable characters (also used for paste, KEY-007).

use crate::calculator::{BinOp, Func, Key, Mode};

/// Maps a typed character to a calculator command. `inv` selects the inverse function
/// for keys whose button has an Inv variant.
pub fn key_for_char(c: char, mode: Mode, inv: bool) -> Option<Key> {
    use Func::*;
    let f = |a: Func, b: Func| Some(Key::Func(if inv { b } else { a }));
    if mode == Mode::Programmer {
        if let Some(d) = c.to_digit(16) {
            return Some(Key::Digit(d as u8));
        }
        return match c {
            '+' => Some(Key::Op(BinOp::Add)),
            '-' => Some(Key::Op(BinOp::Sub)),
            '*' => Some(Key::Op(BinOp::Mul)),
            '/' => Some(Key::Op(BinOp::Div)),
            '%' => Some(Key::Op(BinOp::Mod)),
            '&' => Some(Key::Op(BinOp::And)),
            '|' => Some(Key::Op(BinOp::Or)),
            '^' => Some(Key::Op(BinOp::Xor)),
            '<' => Some(Key::Op(BinOp::Lsh)),
            '>' => Some(Key::Op(BinOp::Rsh)),
            '~' => Some(Key::Func(Not)),
            'j' | 'J' => Some(Key::Func(RoL)),
            'k' | 'K' => Some(Key::Func(RoR)),
            '(' => Some(Key::OpenParen),
            ')' => Some(Key::CloseParen),
            '=' | '\n' => Some(Key::Equals),
            _ => None,
        };
    }
    if let Some(d) = c.to_digit(10) {
        return Some(Key::Digit(d as u8));
    }
    let common = match c {
        '.' | ',' => Some(Key::Point),
        '+' => Some(Key::Op(BinOp::Add)),
        '-' => Some(Key::Op(BinOp::Sub)),
        '*' => Some(Key::Op(BinOp::Mul)),
        '/' => Some(Key::Op(BinOp::Div)),
        '=' | '\n' => Some(Key::Equals),
        '@' => Some(Key::Func(Sqrt)),
        'r' | 'R' => Some(Key::Func(Reciprocal)),
        _ => None,
    };
    if common.is_some() {
        return common;
    }
    match mode {
        Mode::Standard => match c {
            '%' => Some(Key::Percent),
            _ => None,
        },
        Mode::Statistics => match c {
            'a' | 'A' => Some(Key::StatAdd),
            _ => None,
        },
        _ => match c {
            '(' => Some(Key::OpenParen),
            ')' => Some(Key::CloseParen),
            '^' | 'y' | 'Y' => Some(Key::Op(if inv { BinOp::Root } else { BinOp::Pow })),
            '%' | 'd' | 'D' => Some(Key::Op(BinOp::Mod)),
            'q' | 'Q' => f(Square, Sqrt),
            '#' => f(Cube, CubeRoot),
            '!' => Some(Key::Func(Fact)),
            ';' => f(Int, Frac),
            's' | 'S' => f(Sin, Asin),
            'o' | 'O' => f(Cos, Acos),
            't' | 'T' => f(Tan, Atan),
            'n' | 'N' => f(Ln, ExpE),
            'l' | 'L' => Some(Key::Func(Log)),
            'g' | 'G' => Some(Key::Func(Pow10)),
            'm' | 'M' => f(Dms, Degrees),
            'p' | 'P' => Some(Key::Pi),
            'v' | 'V' => Some(Key::FE),
            'x' | 'X' | 'e' | 'E' => Some(Key::Exp),
            'i' | 'I' => Some(Key::Inv),
            _ => None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calculator::{Angle, Base, Calculator, StatFn, WordSize};

    fn calc(mode: Mode) -> Calculator {
        let mut c = Calculator::new();
        c.set_mode(mode);
        c
    }

    fn feed(c: &mut Calculator, keys: &str) {
        for ch in keys.chars() {
            if ch == ' ' {
                continue;
            }
            let key = key_for_char(ch, c.mode, c.inv).unwrap_or_else(|| panic!("no key {ch:?}"));
            c.press(key);
        }
    }

    fn run(mode: Mode, keys: &str) -> String {
        let mut c = calc(mode);
        feed(&mut c, keys);
        c.display()
    }

    #[test]
    fn standard_is_immediate() {
        assert_eq!(run(Mode::Standard, "2+3*4="), "20");
        assert_eq!(run(Mode::Standard, "2+3*"), "5");
        assert_eq!(run(Mode::Standard, "0.1+0.2="), "0.3");
        assert_eq!(run(Mode::Standard, "1/3="), "0.3333333333333333");
    }

    #[test]
    fn scientific_has_precedence() {
        assert_eq!(run(Mode::Scientific, "2+3*4="), "14");
        assert_eq!(run(Mode::Scientific, "(2+3)*4="), "20");
        assert_eq!(run(Mode::Scientific, "2^10="), "1024");
        assert_eq!(run(Mode::Scientific, "2+3^2*2="), "20");
        assert_eq!(
            run(Mode::Scientific, "1/3="),
            "0.33333333333333333333333333333333"
        );
        assert_eq!(run(Mode::Scientific, "(2+3"), "3");
        assert_eq!(run(Mode::Scientific, "(2+3="), "5");
    }

    #[test]
    fn repeat_equals_and_operator_replacement() {
        assert_eq!(run(Mode::Standard, "2+3=="), "8");
        assert_eq!(run(Mode::Standard, "2+="), "4");
        assert_eq!(run(Mode::Standard, "6+-2="), "4");
    }

    #[test]
    fn percent() {
        assert_eq!(run(Mode::Standard, "5+10%"), "0.5");
        assert_eq!(run(Mode::Standard, "5+10%="), "5.5");
        assert_eq!(run(Mode::Standard, "200*5%="), "2000");
    }

    #[test]
    fn functions_and_expression_text() {
        let mut c = calc(Mode::Standard);
        feed(&mut c, "5+9@");
        assert_eq!(c.display(), "3");
        assert_eq!(c.expression(), "5 + sqrt(9)");
        feed(&mut c, "=");
        assert_eq!(c.display(), "8");
        assert_eq!(c.history().last().unwrap().expr, "5 + sqrt(9)");

        let mut c = calc(Mode::Scientific);
        feed(&mut c, "(2+3)*");
        assert_eq!(c.expression(), "(2 + 3) *");
        assert_eq!(c.display(), "5");
    }

    #[test]
    fn trig_in_all_angle_units() {
        assert_eq!(run(Mode::Scientific, "30s"), "0.5");
        assert_eq!(run(Mode::Scientific, "180s"), "0");
        assert_eq!(run(Mode::Scientific, "90t"), "Invalid input");
        assert_eq!(run(Mode::Scientific, "0.5is"), "30");
        let mut c = calc(Mode::Scientific);
        c.angle = Angle::Radians;
        feed(&mut c, "ps");
        assert_eq!(c.display(), "0");
        c.angle = Angle::Gradians;
        feed(&mut c, "100s");
        assert_eq!(c.display(), "1");
    }

    #[test]
    fn scientific_misc() {
        assert_eq!(run(Mode::Scientific, "5!"), "120");
        assert_eq!(run(Mode::Scientific, "8iy3="), "2");
        assert_eq!(run(Mode::Scientific, "100l"), "2");
        assert_eq!(
            run(Mode::Scientific, "1in"),
            "2.7182818284590452353602874713527"
        );
        assert_eq!(run(Mode::Scientific, "12.75;"), "12");
        assert_eq!(run(Mode::Scientific, "12.75i;"), "0.75");
        assert_eq!(run(Mode::Scientific, "10.5m"), "10.3");
        assert_eq!(run(Mode::Scientific, "10.3im"), "10.5");
        assert_eq!(run(Mode::Scientific, "17d5="), "2");
        assert_eq!(run(Mode::Scientific, "1x5"), "1.e+5");
        assert_eq!(run(Mode::Scientific, "1x5+1="), "100001");
        assert_eq!(run(Mode::Scientific, "1234v"), "1.234e+3");
    }

    #[test]
    fn errors() {
        assert_eq!(run(Mode::Standard, "5/0="), "Cannot divide by zero");
        assert_eq!(run(Mode::Standard, "0/0="), "Result is undefined");
        assert_eq!(run(Mode::Standard, "5/0=7"), "7");
        assert_eq!(run(Mode::Standard, "0r"), "Cannot divide by zero");
    }

    #[test]
    fn entry_editing() {
        let mut c = calc(Mode::Standard);
        feed(&mut c, "123.45");
        c.press(Key::Back);
        c.press(Key::Negate);
        assert_eq!(c.display(), "-123.4");
        let mut c = calc(Mode::Standard);
        feed(&mut c, "1234567.25");
        assert_eq!(
            c.state().text(true, &calc_format::Locale::EN_US),
            "1,234,567.25"
        );
        assert_eq!(
            run(Mode::Standard, "12345678901234567890"),
            "1234567890123456"
        );
    }

    #[test]
    fn memory() {
        let mut c = calc(Mode::Standard);
        feed(&mut c, "5");
        c.press(Key::MemStore);
        feed(&mut c, "3");
        c.press(Key::MemAdd);
        c.press(Key::Clear);
        c.press(Key::MemRecall);
        assert_eq!(c.display(), "8");
        assert!(c.has_memory());
        c.press(Key::MemClear);
        assert!(!c.has_memory());
    }

    #[test]
    fn programmer() {
        let mut c = calc(Mode::Programmer);
        c.set_base(Base::Hex);
        feed(&mut c, "ff+1=");
        assert_eq!(c.display(), "100");
        c.set_word(WordSize::Byte);
        assert_eq!(c.display(), "0");
        feed(&mut c, "7f+1=");
        assert_eq!(c.display(), "80");
        c.set_base(Base::Dec);
        assert_eq!(c.display(), "-128");
        c.set_base(Base::Bin);
        feed(&mut c, "1j");
        assert_eq!(c.display(), "10");
        c.set_word(WordSize::Qword);
        c.set_base(Base::Dec);
        assert_eq!(run(Mode::Programmer, "7/2="), "3");
        assert_eq!(run(Mode::Programmer, "6&3|8="), "10");
        assert_eq!(run(Mode::Programmer, "1<4="), "16");
        assert_eq!(run(Mode::Programmer, "0~"), "-1");
        let mut c = calc(Mode::Programmer);
        c.press(Key::ToggleBit(3));
        assert_eq!(c.display(), "8");
        assert_eq!(c.bits(), 8);
    }

    #[test]
    fn statistics() {
        let mut c = calc(Mode::Statistics);
        for k in ["2a", "4a", "4a", "4a", "5a", "5a", "7a", "9a"] {
            feed(&mut c, k);
        }
        c.press(Key::Stat(StatFn::Mean));
        assert_eq!(c.display(), "5");
        c.press(Key::Stat(StatFn::StdDevN));
        assert_eq!(c.display(), "2");
        c.press(Key::Stat(StatFn::Sum));
        assert_eq!(c.display(), "40");
        c.press(Key::Stat(StatFn::SumSq));
        assert_eq!(c.display(), "232");
        c.press(Key::Stat(StatFn::StdDevN1));
        assert_eq!(c.display(), "2.138089935299395");
        c.press(Key::StatClear);
        c.press(Key::Stat(StatFn::Mean));
        assert_eq!(c.display(), "Invalid input");
    }
}
