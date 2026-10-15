//! Property tests (§10.1, NFR-010): no key sequence panics, and arithmetic identities
//! hold exactly.

mod common;

use calc_core::{Angle, Base, BinOp, Calculator, Func, Key, Mode, Num, StatFn, WordSize};
use proptest::prelude::*;

const FUNCS: [Func; 31] = [
    Func::Sqrt,
    Func::Reciprocal,
    Func::Square,
    Func::Cube,
    Func::CubeRoot,
    Func::Pow10,
    Func::ExpE,
    Func::Ln,
    Func::Log,
    Func::Sin,
    Func::Cos,
    Func::Tan,
    Func::Asin,
    Func::Acos,
    Func::Atan,
    Func::Sinh,
    Func::Cosh,
    Func::Tanh,
    Func::Asinh,
    Func::Acosh,
    Func::Atanh,
    Func::Fact,
    Func::Int,
    Func::Frac,
    Func::Dms,
    Func::Degrees,
    Func::Not,
    Func::RoL,
    Func::RoR,
    Func::ByteFlip,
    Func::WordFlip,
];

const OPS: [BinOp; 12] = [
    BinOp::Add,
    BinOp::Sub,
    BinOp::Mul,
    BinOp::Div,
    BinOp::Mod,
    BinOp::Pow,
    BinOp::Root,
    BinOp::And,
    BinOp::Or,
    BinOp::Xor,
    BinOp::Lsh,
    BinOp::Rsh,
];

const STATS: [StatFn; 6] = [
    StatFn::Mean,
    StatFn::MeanSq,
    StatFn::Sum,
    StatFn::SumSq,
    StatFn::StdDevN,
    StatFn::StdDevN1,
];

/// One user action: a key or a mode/radix/word/angle change.
#[derive(Debug, Clone)]
enum Action {
    Key(Key),
    Mode(Mode),
    Base(Base),
    Word(WordSize),
    Angle(Angle),
}

fn key() -> impl Strategy<Value = Key> {
    prop_oneof![
        8 => (0u8..16).prop_map(Key::Digit),
        1 => (0usize..OPS.len()).prop_map(|i| Key::Op(OPS[i])),
        1 => (0usize..FUNCS.len()).prop_map(|i| Key::Func(FUNCS[i])),
        1 => (0usize..STATS.len()).prop_map(|i| Key::Stat(STATS[i])),
        1 => (0u32..70).prop_map(Key::ToggleBit),
        2 => prop::sample::select(vec![
            Key::Point,
            Key::Negate,
            Key::Back,
            Key::ClearEntry,
            Key::Clear,
            Key::Equals,
            Key::Percent,
            Key::Pi,
            Key::OpenParen,
            Key::CloseParen,
            Key::Exp,
            Key::FE,
            Key::Inv,
            Key::MemClear,
            Key::MemRecall,
            Key::MemStore,
            Key::MemAdd,
            Key::MemSub,
            Key::StatAdd,
            Key::StatClear,
        ]),
    ]
}

fn action() -> impl Strategy<Value = Action> {
    prop_oneof![
        20 => key().prop_map(Action::Key),
        1 => prop::sample::select(vec![
            Mode::Standard,
            Mode::Scientific,
            Mode::Programmer,
            Mode::Statistics
        ])
        .prop_map(Action::Mode),
        1 => prop::sample::select(vec![Base::Hex, Base::Dec, Base::Oct, Base::Bin])
            .prop_map(Action::Base),
        1 => prop::sample::select(vec![
            WordSize::Qword,
            WordSize::Dword,
            WordSize::Word,
            WordSize::Byte
        ])
        .prop_map(Action::Word),
        1 => prop::sample::select(vec![Angle::Degrees, Angle::Radians, Angle::Gradians])
            .prop_map(Action::Angle),
    ]
}

/// A decimal literal of up to 12 digits with an optional fraction.
fn decimal() -> impl Strategy<Value = String> {
    (any::<bool>(), 0u64..1_000_000_000_000, 0u32..8).prop_map(|(neg, m, scale)| {
        let s = m.to_string();
        let s = if scale as usize >= s.len() {
            format!("0.{}{s}", "0".repeat(scale as usize - s.len()))
        } else if scale > 0 {
            let (i, f) = s.split_at(s.len() - scale as usize);
            format!("{i}.{f}")
        } else {
            s
        };
        if neg { format!("-{s}") } else { s }
    })
}

fn num(s: &str) -> Num {
    Num::parse(s).expect("valid literal")
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 512, ..ProptestConfig::default() })]

    /// NFR-010: arbitrary input never panics, and the display is never empty.
    #[test]
    fn no_sequence_panics(actions in prop::collection::vec(action(), 0..80)) {
        let mut c = Calculator::new();
        for a in actions {
            match a {
                Action::Key(k) => {
                    let s = c.press(k);
                    prop_assert!(!s.text(true, &calc_format::Locale::EN_US).is_empty());
                }
                Action::Mode(m) => c.set_mode(m),
                Action::Base(b) => c.set_base(b),
                Action::Word(w) => c.set_word(w),
                Action::Angle(a) => c.angle = a,
            }
        }
        prop_assert!(!c.display().is_empty());
        let _ = c.expression();
    }

    /// `a + b - b = a` exactly.
    #[test]
    fn add_then_subtract(a in decimal(), b in decimal()) {
        let (x, y) = (num(&a), num(&b));
        let r = x.add(&y).unwrap().sub(&y).unwrap();
        prop_assert_eq!(r.compare(&x), std::cmp::Ordering::Equal);
    }

    /// `a × b ÷ b = a` exactly for non-zero b.
    #[test]
    fn multiply_then_divide(a in decimal(), b in decimal()) {
        let (x, y) = (num(&a), num(&b));
        prop_assume!(!y.is_zero());
        let r = x.mul(&y).unwrap().div(&y).unwrap();
        prop_assert_eq!(r.compare(&x), std::cmp::Ordering::Equal);
    }

    /// Typing a number and pressing `=` shows it back unchanged.
    #[test]
    fn entry_round_trips(a in decimal()) {
        let mut c = common::calc(Mode::Scientific);
        let (neg, digits) = match a.strip_prefix('-') {
            Some(d) => (true, d),
            None => (false, a.as_str()),
        };
        common::feed(&mut c, digits);
        if neg {
            c.apply_key(Key::Negate);
        }
        c.apply_key(Key::Equals);
        let shown = num(&c.display());
        prop_assert_eq!(shown.compare(&num(&a)), std::cmp::Ordering::Equal);
    }

    /// Standard mode evaluates left to right: `a + b * c = (a + b) * c`.
    #[test]
    fn standard_is_left_to_right(a in 0u32..10_000, b in 0u32..10_000, k in 0u32..10_000) {
        let mut c = common::calc(Mode::Standard);
        common::feed(&mut c, &format!("{a}+{b}*{k}="));
        prop_assert_eq!(c.display(), ((a as u64 + b as u64) * k as u64).to_string());
    }

    /// Programmer mode wraps like the matching two's-complement integer type.
    #[test]
    fn programmer_wraps_like_i32(a in (i32::MIN + 1)..=i32::MAX, b in (i32::MIN + 1)..=i32::MAX) {
        let mut c = common::calc(Mode::Programmer);
        c.set_word(WordSize::Dword);
        common::feed(&mut c, &a.unsigned_abs().to_string());
        if a < 0 {
            c.apply_key(Key::Negate);
        }
        c.apply_key(Key::Op(BinOp::Mul));
        common::feed(&mut c, &b.unsigned_abs().to_string());
        if b < 0 {
            c.apply_key(Key::Negate);
        }
        c.apply_key(Key::Equals);
        prop_assert_eq!(c.display(), a.wrapping_mul(b).to_string());
    }
}
