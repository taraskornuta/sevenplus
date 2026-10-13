//! Key-sequence notation shared by the golden, property and performance tests.
//!
//! A sequence is typed characters (mapped exactly like the keyboard, see
//! `calc_core::keys`) plus `{NAME}` tokens for keys without a character. Spaces outside
//! braces are ignored.

#![allow(dead_code)]

use calc_core::keys::key_for_char;
use calc_core::{Angle, Base, BinOp, Calculator, Func, Key, Mode, StatFn, WordSize};

pub fn mode(name: &str) -> Mode {
    match name.trim().to_ascii_lowercase().as_str() {
        "standard" | "std" => Mode::Standard,
        "scientific" | "sci" => Mode::Scientific,
        "programmer" | "prg" => Mode::Programmer,
        "statistics" | "sta" => Mode::Statistics,
        m => panic!("unknown mode {m:?}"),
    }
}

/// Applies a `{NAME}` token. Returns false for an unknown name.
fn named(c: &mut Calculator, name: &str) -> bool {
    let key = match name {
        "C" => Key::Clear,
        "CE" => Key::ClearEntry,
        "BS" => Key::Back,
        "NEG" => Key::Negate,
        "EQ" => Key::Equals,
        "PCT" => Key::Percent,
        "MS" => Key::MemStore,
        "MR" => Key::MemRecall,
        "MC" => Key::MemClear,
        "M+" => Key::MemAdd,
        "M-" => Key::MemSub,
        "INV" => Key::Inv,
        "FE" => Key::FE,
        "EXP" => Key::Exp,
        "PI" => Key::Pi,
        "MOD" => Key::Op(BinOp::Mod),
        "ROOT" => Key::Op(BinOp::Root),
        "SINH" => Key::Func(Func::Sinh),
        "COSH" => Key::Func(Func::Cosh),
        "TANH" => Key::Func(Func::Tanh),
        "ASINH" => Key::Func(Func::Asinh),
        "ACOSH" => Key::Func(Func::Acosh),
        "ATANH" => Key::Func(Func::Atanh),
        "CBRT" => Key::Func(Func::CubeRoot),
        "EXPE" => Key::Func(Func::ExpE),
        "ADD" => Key::StatAdd,
        "CAD" => Key::StatClear,
        "MEAN" => Key::Stat(StatFn::Mean),
        "MEANSQ" => Key::Stat(StatFn::MeanSq),
        "SUM" => Key::Stat(StatFn::Sum),
        "SUMSQ" => Key::Stat(StatFn::SumSq),
        "SDN" => Key::Stat(StatFn::StdDevN),
        "SDN1" => Key::Stat(StatFn::StdDevN1),
        "DEG" => return set(|c| c.angle = Angle::Degrees, c),
        "RAD" => return set(|c| c.angle = Angle::Radians, c),
        "GRAD" => return set(|c| c.angle = Angle::Gradians, c),
        "HEX" => return set(|c| c.set_base(Base::Hex), c),
        "DEC" => return set(|c| c.set_base(Base::Dec), c),
        "OCT" => return set(|c| c.set_base(Base::Oct), c),
        "BIN" => return set(|c| c.set_base(Base::Bin), c),
        "QWORD" => return set(|c| c.set_word(WordSize::Qword), c),
        "DWORD" => return set(|c| c.set_word(WordSize::Dword), c),
        "WORD" => return set(|c| c.set_word(WordSize::Word), c),
        "BYTE" => return set(|c| c.set_word(WordSize::Byte), c),
        "STANDARD" | "SCIENTIFIC" | "PROGRAMMER" | "STATISTICS" => {
            c.set_mode(mode(name));
            return true;
        }
        n => match n.strip_prefix("BIT").and_then(|b| b.parse().ok()) {
            Some(b) => Key::ToggleBit(b),
            None => return false,
        },
    };
    c.apply_key(key);
    true
}

fn set(f: impl FnOnce(&mut Calculator), c: &mut Calculator) -> bool {
    f(c);
    true
}

/// Feeds a key sequence; panics on unknown keys.
pub fn feed(c: &mut Calculator, keys: &str) {
    let mut chars = keys.chars();
    while let Some(ch) = chars.next() {
        match ch {
            ' ' => {}
            '{' => {
                let name: String = chars.by_ref().take_while(|&c| c != '}').collect();
                assert!(named(c, &name), "unknown key {{{name}}}");
            }
            _ => {
                let key = key_for_char(ch, c.mode, c.inv)
                    .unwrap_or_else(|| panic!("no key for {ch:?} in {:?}", c.mode));
                c.apply_key(key);
            }
        }
    }
}

pub fn calc(m: Mode) -> Calculator {
    let mut c = Calculator::new();
    c.set_mode(m);
    c
}
