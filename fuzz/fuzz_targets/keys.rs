//! NFR-010: random key sequences must never panic the engine.
//!
//! Run with `cargo +nightly fuzz run keys -- -max_total_time=3600`.

#![no_main]

use calc_core::{Angle, Base, BinOp, Calculator, Func, Key, Mode, StatFn, WordSize};
use calc_format::Locale;
use libfuzzer_sys::fuzz_target;

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

const FUNCS: [Func; 29] = [
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
];

const KEYS: [Key; 20] = [
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
];

const STATS: [StatFn; 6] = [
    StatFn::Mean,
    StatFn::MeanSq,
    StatFn::Sum,
    StatFn::SumSq,
    StatFn::StdDevN,
    StatFn::StdDevN1,
];

fuzz_target!(|data: &[u8]| {
    let mut c = Calculator::new();
    // Two bytes per action: a selector and an argument.
    for pair in data.chunks_exact(2).take(400) {
        let (sel, arg) = (pair[0], pair[1] as usize);
        match sel % 16 {
            0..=6 => {
                c.press(Key::Digit((arg % 16) as u8));
            }
            7 => {
                c.press(Key::Op(OPS[arg % OPS.len()]));
            }
            8 => {
                c.press(Key::Func(FUNCS[arg % FUNCS.len()]));
            }
            9 | 10 => {
                c.press(KEYS[arg % KEYS.len()]);
            }
            11 => {
                c.press(Key::Stat(STATS[arg % STATS.len()]));
            }
            12 => {
                c.press(Key::ToggleBit((arg % 70) as u32));
            }
            13 => c.set_mode(
                [Mode::Standard, Mode::Scientific, Mode::Programmer, Mode::Statistics][arg % 4],
            ),
            14 => c.set_base([Base::Hex, Base::Dec, Base::Oct, Base::Bin][arg % 4]),
            _ => {
                c.set_word([WordSize::Qword, WordSize::Dword, WordSize::Word, WordSize::Byte][arg % 4]);
                c.angle = [Angle::Degrees, Angle::Radians, Angle::Gradians][arg % 3];
            }
        }
        let s = c.state();
        assert!(!s.text(true, &Locale::EN_US).is_empty());
    }
    let _ = c.expression();
});
