//! The Win7 calculator state machine, shared by all four modes (ENG-008).
//!
//! Standard and Statistics use immediate execution (`2 + 3 * 4 = 20`); Scientific and
//! Programmer honour operator precedence and parentheses (`2 + 3 * 4 = 14`).
//! Programmer mode computes with 64-bit integers masked to the word size (ENG-005).

use crate::number::{CalcError, CalcResult, Num};
use calc_format::{EntryText, FormatSettings, Locale, Notation, Value};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Standard,
    Scientific,
    Programmer,
    Statistics,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Angle {
    #[default]
    Degrees,
    Radians,
    Gradians,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Base {
    Hex,
    #[default]
    Dec,
    Oct,
    Bin,
}

impl Base {
    pub fn radix(self) -> u32 {
        match self {
            Base::Hex => 16,
            Base::Dec => 10,
            Base::Oct => 8,
            Base::Bin => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WordSize {
    #[default]
    Qword,
    Dword,
    Word,
    Byte,
}

impl WordSize {
    pub fn bits(self) -> u32 {
        match self {
            WordSize::Qword => 64,
            WordSize::Dword => 32,
            WordSize::Word => 16,
            WordSize::Byte => 8,
        }
    }

    pub fn mask(self) -> u64 {
        u64::MAX >> (64 - self.bits())
    }

    /// Truncates `v` to this word size and sign-extends it back (two's complement).
    pub fn wrap(self, v: i64) -> i64 {
        let shift = 64 - self.bits();
        (v << shift) >> shift
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Root,
    And,
    Or,
    Xor,
    Lsh,
    Rsh,
}

impl BinOp {
    /// Win7 precedence table (CalcEngine `rgbPrec`).
    fn prec(self) -> u8 {
        match self {
            BinOp::Or | BinOp::Xor => 0,
            BinOp::And => 1,
            BinOp::Add | BinOp::Sub => 2,
            BinOp::Mul | BinOp::Div | BinOp::Mod | BinOp::Lsh | BinOp::Rsh => 3,
            BinOp::Pow | BinOp::Root => 4,
        }
    }

    pub fn symbol(self) -> &'static str {
        match self {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::Mod => "Mod",
            BinOp::Pow => "^",
            BinOp::Root => "yroot",
            BinOp::And => "And",
            BinOp::Or => "Or",
            BinOp::Xor => "Xor",
            BinOp::Lsh => "Lsh",
            BinOp::Rsh => "Rsh",
        }
    }

    /// The C-language operator, used in Programmer mode's expression line.
    pub fn c_symbol(self) -> &'static str {
        match self {
            BinOp::Mod => "%",
            BinOp::And => "&",
            BinOp::Or => "|",
            BinOp::Xor => "^",
            BinOp::Lsh => "<<",
            BinOp::Rsh => ">>",
            op => op.symbol(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Func {
    Sqrt,
    Reciprocal,
    Square,
    Cube,
    CubeRoot,
    Pow10,
    ExpE,
    Ln,
    Log,
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    Sinh,
    Cosh,
    Tanh,
    Asinh,
    Acosh,
    Atanh,
    Fact,
    Int,
    Frac,
    Dms,
    Degrees,
    Not,
    RoL,
    RoR,
    /// Reverses the byte order within the word size (endianness swap).
    ByteFlip,
    /// Reverses the order of the 16-bit units within the word size.
    WordFlip,
}

impl Func {
    fn name(self, angle: Angle) -> String {
        let a = match angle {
            Angle::Degrees => "d",
            Angle::Radians => "r",
            Angle::Gradians => "g",
        };
        match self {
            Func::Sqrt => "sqrt".into(),
            Func::Reciprocal => "reciproc".into(),
            Func::Square => "sqr".into(),
            Func::Cube => "cube".into(),
            Func::CubeRoot => "cuberoot".into(),
            Func::Pow10 => "powten".into(),
            Func::ExpE => "powe".into(),
            Func::Ln => "ln".into(),
            Func::Log => "log".into(),
            Func::Sin => format!("sin{a}"),
            Func::Cos => format!("cos{a}"),
            Func::Tan => format!("tan{a}"),
            Func::Asin => format!("asin{a}"),
            Func::Acos => format!("acos{a}"),
            Func::Atan => format!("atan{a}"),
            Func::Sinh => "sinh".into(),
            Func::Cosh => "cosh".into(),
            Func::Tanh => "tanh".into(),
            Func::Asinh => "asinh".into(),
            Func::Acosh => "acosh".into(),
            Func::Atanh => "atanh".into(),
            Func::Fact => "fact".into(),
            Func::Int => "Int".into(),
            Func::Frac => "frac".into(),
            Func::Dms => "dms".into(),
            Func::Degrees => "degrees".into(),
            Func::Not => "NOT".into(),
            Func::RoL => "RoL".into(),
            Func::RoR => "RoR".into(),
            Func::ByteFlip => "bswap".into(),
            Func::WordFlip => "wswap".into(),
        }
    }

    /// Functions available in Programmer mode.
    fn integer(self) -> bool {
        matches!(
            self,
            Func::Not | Func::RoL | Func::RoR | Func::ByteFlip | Func::WordFlip
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatFn {
    Mean,
    MeanSq,
    Sum,
    SumSq,
    StdDevN,
    StdDevN1,
}

/// A calculator key: every input the engine understands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Digit(u8),
    Point,
    Negate,
    Back,
    ClearEntry,
    Clear,
    Op(BinOp),
    Equals,
    Percent,
    Func(Func),
    Pi,
    OpenParen,
    CloseParen,
    Exp,
    FE,
    Inv,
    MemClear,
    MemRecall,
    MemStore,
    MemAdd,
    MemSub,
    StatAdd,
    StatClear,
    Stat(StatFn),
    ToggleBit(u32),
}

#[derive(Debug, Clone)]
pub struct HistoryItem {
    pub expr: String,
    pub result: Num,
}

/// Everything the display shows, before locale-specific formatting (ENG-008, ENG-010).
#[derive(Debug, Clone, PartialEq)]
pub struct DisplayState {
    pub mode: Mode,
    /// The current number: the typed entry's value or the last result.
    pub value: Value,
    /// The number being typed, if any, kept as text.
    pub entry: Option<EntryText>,
    /// Pending expression with `.` as decimal point, e.g. `12 + sqrt(9) -`.
    pub expression: String,
    pub error: Option<CalcError>,
    pub memory: bool,
    pub paren_depth: usize,
    pub inv: bool,
    /// Digits, notation and radix for this mode; the UI adds grouping.
    pub format: FormatSettings,
}

impl DisplayState {
    /// The main display text.
    pub fn text(&self, grouping: bool, locale: &Locale) -> String {
        if let Some(e) = self.error {
            return e.to_string();
        }
        let s = FormatSettings {
            grouping,
            ..self.format
        };
        match &self.entry {
            Some(e) => calc_format::format_entry(e, &s, locale),
            None => calc_format::format(&self.value, &s, locale),
        }
    }
}

/// Digits requested from a number for display: enough to round 32 digits correctly.
const DECIMAL_DIGITS: usize = 40;

#[derive(Debug, Clone)]
pub struct Calculator {
    pub mode: Mode,
    pub angle: Angle,
    pub base: Base,
    pub word: WordSize,
    pub inv: bool,
    pub fe: bool,
    memory: Option<Num>,
    history: Vec<HistoryItem>,
    stats: Vec<Num>,
    entry: Option<EntryText>,
    value: Num,
    /// Expression text of `value` when it came from a function, e.g. `sqrt(9)`.
    operand_text: Option<String>,
    /// Pending `(lhs, op)` pairs; one inner vec per open parenthesis level.
    levels: Vec<Vec<(Num, BinOp)>>,
    tokens: Vec<String>,
    paren_marks: Vec<usize>,
    last_was_op: bool,
    repeat: Option<(BinOp, Num)>,
    error: Option<CalcError>,
}

impl Default for Calculator {
    fn default() -> Self {
        Self::new()
    }
}

impl Calculator {
    pub fn new() -> Self {
        Calculator {
            mode: Mode::Standard,
            angle: Angle::Degrees,
            base: Base::Dec,
            word: WordSize::Qword,
            inv: false,
            fe: false,
            memory: None,
            history: Vec::new(),
            stats: Vec::new(),
            entry: None,
            value: Num::zero(),
            operand_text: None,
            levels: vec![Vec::new()],
            tokens: Vec::new(),
            paren_marks: Vec::new(),
            last_was_op: false,
            repeat: None,
            error: None,
        }
    }

    // ---- queries -------------------------------------------------------------------

    /// Significant digits shown: Win7 shows 32 in Scientific and 16 elsewhere.
    fn max_digits(&self) -> usize {
        if self.mode == Mode::Scientific {
            32
        } else {
            16
        }
    }

    fn precedence(&self) -> bool {
        matches!(self.mode, Mode::Scientific | Mode::Programmer)
    }

    /// Formatting settings of the current mode, without grouping.
    pub fn format_settings(&self) -> FormatSettings {
        FormatSettings {
            max_digits: self.max_digits(),
            notation: if self.fe {
                Notation::Exponential
            } else {
                Notation::Auto
            },
            grouping: false,
            radix: if self.mode == Mode::Programmer {
                self.base.radix()
            } else {
                10
            },
        }
    }

    pub fn value(&self) -> Num {
        self.entry
            .as_ref()
            .map_or_else(|| self.value.clone(), |e| self.entry_value(e))
    }

    pub fn error(&self) -> Option<CalcError> {
        self.error
    }

    pub fn has_memory(&self) -> bool {
        self.memory.is_some()
    }

    pub fn paren_depth(&self) -> usize {
        self.paren_marks.len()
    }

    pub fn history(&self) -> &[HistoryItem] {
        &self.history
    }

    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    pub fn stats(&self) -> &[Num] {
        &self.stats
    }

    pub fn stat_delete(&mut self, i: usize) {
        if i < self.stats.len() {
            self.stats.remove(i);
        }
    }

    /// Programmer-mode value as a sign-extended 64-bit integer.
    fn int(&self, n: &Num) -> i64 {
        self.word.wrap(n.to_i128() as i64)
    }

    /// Current integer value in programmer mode, as raw bits.
    pub fn bits(&self) -> u64 {
        (self.int(&self.value()) as u64) & self.word.mask()
    }

    /// A number as a display value in the current mode.
    pub fn to_value(&self, n: &Num) -> Value {
        if self.mode == Mode::Programmer {
            Value::Int {
                bits: self.int(n) as u64,
                word: self.word.bits(),
            }
        } else {
            Value::Dec(n.to_decimal(DECIMAL_DIGITS))
        }
    }

    /// Canonical text of a number (no grouping, `.` decimal point), as used in the
    /// expression line and history.
    pub fn format_num(&self, n: &Num) -> String {
        calc_format::format(&self.to_value(n), &self.format_settings(), &Locale::EN_US)
    }

    /// Everything the display needs (ENG-008).
    pub fn state(&self) -> DisplayState {
        DisplayState {
            mode: self.mode,
            value: self.to_value(&self.value()),
            entry: self.entry.clone(),
            expression: self.expression(),
            error: self.error,
            memory: self.has_memory(),
            paren_depth: self.paren_depth(),
            inv: self.inv,
            format: self.format_settings(),
        }
    }

    /// Main display text with default formatting (no grouping, en-US).
    pub fn display(&self) -> String {
        self.state().text(false, &Locale::EN_US)
    }

    /// Expression line shown above the display, e.g. `2 + sqrt(9) *`.
    pub fn expression(&self) -> String {
        let mut parts = self.tokens.clone();
        if let Some(t) = &self.operand_text {
            parts.push(t.clone());
        }
        parts.join(" ").replace("( ", "(")
    }

    /// Whether a key does anything in the current state (disabled buttons, PRG-004,
    /// ENG-006).
    pub fn enabled(&self, key: Key) -> bool {
        if self.error.is_some() {
            return matches!(key, Key::Digit(_) | Key::Clear | Key::ClearEntry);
        }
        match self.mode {
            Mode::Programmer => match key {
                Key::Digit(n) => (n as u32) < self.base.radix(),
                Key::Point | Key::Percent | Key::Pi | Key::Exp | Key::FE | Key::Inv => false,
                Key::Stat(_) | Key::StatAdd | Key::StatClear => false,
                Key::Op(op) => !matches!(op, BinOp::Pow | BinOp::Root),
                // A flip needs at least two bytes or two 16-bit units to swap.
                Key::Func(Func::ByteFlip) => self.word.bits() >= 16,
                Key::Func(Func::WordFlip) => self.word.bits() >= 32,
                Key::Func(f) => f.integer(),
                Key::CloseParen => self.paren_depth() > 0,
                _ => true,
            },
            mode => match key {
                Key::Digit(n) => n < 10,
                Key::ToggleBit(_) => false,
                Key::Func(f) => !f.integer(),
                Key::Op(op) => {
                    matches!(op, BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div)
                        || mode == Mode::Scientific
                }
                Key::OpenParen | Key::Exp | Key::Inv | Key::FE | Key::Pi => {
                    mode == Mode::Scientific
                }
                Key::CloseParen => self.paren_depth() > 0,
                Key::Stat(_) | Key::StatAdd | Key::StatClear => mode == Mode::Statistics,
                _ => true,
            },
        }
    }

    // ---- settings ------------------------------------------------------------------

    fn reset_expression(&mut self) {
        self.levels = vec![Vec::new()];
        self.tokens.clear();
        self.paren_marks.clear();
        self.operand_text = None;
        self.last_was_op = false;
        self.inv = false;
    }

    /// Switches mode. History is cleared (HIS-004); memory is kept, truncated to an
    /// integer in Programmer mode (MEM-003).
    pub fn set_mode(&mut self, mode: Mode) {
        if self.mode == mode {
            return;
        }
        self.commit();
        self.reset_expression();
        self.error = None;
        self.repeat = None;
        self.history.clear();
        self.mode = mode;
        if mode == Mode::Programmer {
            self.value = Num::from_i64(self.int(&self.value));
            if let Some(m) = &self.memory {
                self.memory = Some(Num::from_i64(self.int(m)));
            }
        }
    }

    pub fn set_base(&mut self, base: Base) {
        self.commit();
        self.base = base;
    }

    pub fn set_word(&mut self, word: WordSize) {
        self.commit();
        self.word = word;
        if self.mode == Mode::Programmer {
            self.value = Num::from_i64(self.int(&self.value));
        }
    }

    /// Replaces the display value (history recall, paste, unit conversion…).
    pub fn load_value(&mut self, n: Num) {
        self.error = None;
        self.entry = None;
        self.operand_text = None;
        self.last_was_op = false;
        self.value = if self.mode == Mode::Programmer {
            Num::from_i64(self.int(&n))
        } else {
            n
        };
    }

    // ---- input ---------------------------------------------------------------------

    /// Applies one key and returns the new display state (ENG-008).
    pub fn press(&mut self, key: Key) -> DisplayState {
        self.apply_key(key);
        self.state()
    }

    /// Applies one key without building a [`DisplayState`].
    pub fn apply_key(&mut self, key: Key) {
        if self.error.is_some() {
            // In an error state only C, CE and digits are accepted (ENG-006).
            match key {
                Key::Clear | Key::ClearEntry => {}
                Key::Digit(_) => self.apply_key(Key::Clear),
                _ => return,
            }
        }
        if let Err(e) = self.dispatch(key) {
            self.error = Some(e);
            self.entry = None;
            self.reset_expression();
        }
    }

    fn dispatch(&mut self, key: Key) -> Result<(), CalcError> {
        match key {
            Key::Digit(d) => self.digit(d),
            Key::Point => self.point(),
            Key::Negate => self.negate()?,
            Key::Back => self.back(),
            Key::ClearEntry => {
                self.error = None;
                self.entry = None;
                self.operand_text = None;
                self.value = Num::zero();
            }
            Key::Clear => {
                let memory = self.memory.take();
                let history = std::mem::take(&mut self.history);
                let stats = std::mem::take(&mut self.stats);
                *self = Calculator {
                    mode: self.mode,
                    angle: self.angle,
                    base: self.base,
                    word: self.word,
                    fe: self.fe,
                    memory,
                    history,
                    stats,
                    ..Calculator::new()
                };
            }
            Key::Op(op) => self.binary(op)?,
            Key::Equals => self.equals()?,
            Key::Percent => self.percent()?,
            Key::Func(f) => self.func(f)?,
            Key::Pi => {
                if self.mode == Mode::Programmer {
                    return Ok(());
                }
                let pi = Num::pi();
                let v = if self.inv {
                    pi.mul(&Num::from_i64(2))?
                } else {
                    pi
                };
                self.load_value(v);
                self.inv = false;
            }
            Key::OpenParen => {
                if !self.precedence() {
                    return Ok(());
                }
                self.entry = None;
                self.operand_text = None;
                self.last_was_op = false;
                self.levels.push(Vec::new());
                self.paren_marks.push(self.tokens.len());
                self.tokens.push("(".into());
            }
            Key::CloseParen => self.close_paren()?,
            Key::Exp => self.exp(),
            Key::FE => {
                if self.mode != Mode::Programmer {
                    self.commit();
                    self.fe = !self.fe;
                }
            }
            Key::Inv => self.inv = !self.inv,
            Key::MemClear => self.memory = None,
            Key::MemRecall => {
                if let Some(m) = self.memory.clone() {
                    self.load_value(m);
                }
            }
            Key::MemStore => {
                self.commit();
                self.memory = Some(self.value.clone());
            }
            Key::MemAdd | Key::MemSub => {
                self.commit();
                let m = self.memory.clone().unwrap_or_else(Num::zero);
                let op = if key == Key::MemAdd {
                    BinOp::Add
                } else {
                    BinOp::Sub
                };
                self.memory = Some(self.apply(&m, op, &self.value)?);
            }
            Key::StatAdd => {
                if self.mode == Mode::Statistics {
                    self.commit();
                    self.stats.push(self.value.clone());
                    self.value = Num::zero();
                    self.operand_text = None;
                }
            }
            Key::StatClear => self.stats.clear(),
            Key::Stat(f) => {
                if self.mode == Mode::Statistics {
                    let v = self.stat(f)?;
                    self.load_value(v);
                }
            }
            Key::ToggleBit(i) => {
                if self.mode == Mode::Programmer && i < self.word.bits() {
                    let v = self.int(&self.value()) ^ (1i64 << i);
                    self.load_value(Num::from_i64(v));
                }
            }
        }
        Ok(())
    }

    fn entry_value(&self, e: &EntryText) -> Num {
        let n = if self.mode == Mode::Programmer {
            let u = u64::from_str_radix(&e.mant, self.base.radix()).unwrap_or(0);
            Num::from_i64(self.word.wrap(u as i64))
        } else {
            let mut s = e.mant.clone();
            if let Some((neg, d)) = &e.exp {
                s = format!(
                    "{s}e{}{}",
                    if *neg { "-" } else { "" },
                    if d.is_empty() { "0" } else { d }
                );
            }
            Num::parse(&s).unwrap_or_else(Num::zero)
        };
        if e.neg { n.neg() } else { n }
    }

    /// Moves a typed entry into `value`.
    fn commit(&mut self) {
        if let Some(e) = self.entry.take() {
            let v = self.entry_value(&e);
            self.value = if self.mode == Mode::Programmer {
                Num::from_i64(self.int(&v))
            } else {
                v
            };
            self.operand_text = None;
        }
    }

    fn start_entry(&mut self) -> &mut EntryText {
        if self.entry.is_none() {
            self.operand_text = None;
            self.last_was_op = false;
        }
        self.entry.get_or_insert_with(|| EntryText {
            mant: "0".into(),
            ..EntryText::default()
        })
    }

    fn digit(&mut self, d: u8) {
        let radix = if self.mode == Mode::Programmer {
            self.base.radix()
        } else {
            10
        };
        if d as u32 >= radix {
            return;
        }
        let max = self.max_digits();
        let mode = self.mode;
        let word = self.word;
        let base = self.base;
        let e = self.start_entry();
        let Some(c) = char::from_digit(d as u32, radix) else {
            return;
        };
        let c = c.to_ascii_uppercase();
        if let Some((_, digits)) = &mut e.exp {
            if digits.len() < 4 {
                digits.push(c);
            }
            return;
        }
        let candidate = if e.mant == "0" {
            c.to_string()
        } else {
            format!("{}{c}", e.mant)
        };
        let fits = if mode == Mode::Programmer {
            let limit = if base == Base::Dec {
                word.mask() >> 1
            } else {
                word.mask()
            };
            u64::from_str_radix(&candidate, radix).is_ok_and(|v| v <= limit)
        } else {
            // STD-008: further digits beyond the display width are ignored.
            let count = candidate
                .trim_start_matches(['0', '.'])
                .chars()
                .filter(|c| *c != '.')
                .count();
            count <= max
        };
        if fits {
            e.mant = candidate;
        }
    }

    fn point(&mut self) {
        if self.mode == Mode::Programmer {
            return;
        }
        let e = self.start_entry();
        if e.exp.is_none() && !e.mant.contains('.') {
            e.mant.push('.');
        }
    }

    fn exp(&mut self) {
        if self.mode != Mode::Scientific {
            return;
        }
        if self.entry.is_none() {
            let s = self.format_num(&self.value);
            let (mant, exp) = match s.split_once('e') {
                Some((m, x)) => (m.trim_end_matches('.').to_string(), Some(x.to_string())),
                None => (s, None),
            };
            let neg = mant.starts_with('-');
            self.entry = Some(EntryText {
                neg,
                mant: mant.trim_start_matches('-').to_string(),
                exp: exp.map(|x| {
                    (
                        x.starts_with('-'),
                        x.trim_start_matches(['+', '-']).to_string(),
                    )
                }),
            });
            self.operand_text = None;
        }
        if let Some(e) = self.entry.as_mut()
            && e.exp.is_none()
        {
            e.exp = Some((false, String::new()));
        }
    }

    fn negate(&mut self) -> Result<(), CalcError> {
        if self.mode != Mode::Programmer
            && let Some(e) = &mut self.entry
        {
            match &mut e.exp {
                Some((neg, _)) => *neg = !*neg,
                None => e.neg = !e.neg,
            }
            return Ok(());
        }
        self.commit();
        let text = self.operand_text();
        self.value = if self.mode == Mode::Programmer {
            Num::from_i64(self.word.wrap(self.int(&self.value).wrapping_neg()))
        } else {
            self.value.neg()
        };
        self.operand_text = Some(format!("negate({text})"));
        self.last_was_op = false;
        Ok(())
    }

    /// STD-007: deletes the last typed digit; no effect on a computed result.
    fn back(&mut self) {
        let Some(e) = &mut self.entry else { return };
        match &mut e.exp {
            Some((_, d)) if !d.is_empty() => {
                d.pop();
            }
            Some(_) => e.exp = None,
            None => {
                e.mant.pop();
                if e.mant.is_empty() {
                    e.mant = "0".into();
                    e.neg = false;
                }
            }
        }
    }

    fn operand_text(&self) -> String {
        self.operand_text
            .clone()
            .unwrap_or_else(|| self.format_num(&self.value))
    }

    /// Operator text in the expression line: C syntax in Programmer mode.
    fn op_text(&self, op: BinOp) -> &'static str {
        if self.mode == Mode::Programmer {
            op.c_symbol()
        } else {
            op.symbol()
        }
    }

    fn binary(&mut self, op: BinOp) -> Result<(), CalcError> {
        if !self.enabled(Key::Op(op)) {
            return Ok(());
        }
        if self.last_was_op {
            // Pressing another operator replaces the pending one (STD-002).
            if let Some(last) = self.levels.last_mut().and_then(|l| l.last_mut()) {
                last.1 = op;
                let text = self.op_text(op);
                if let Some(t) = self.tokens.last_mut() {
                    *t = text.into();
                }
                return Ok(());
            }
        }
        self.commit();
        let text = self.operand_text();
        let mut v = self.value.clone();
        let immediate = !self.precedence();
        while let Some(top) = self.levels.last_mut() {
            match top.last() {
                Some((_, pop)) if immediate || pop.prec() >= op.prec() => {
                    let Some((lhs, pop)) = top.pop() else { break };
                    v = self.apply(&lhs, pop, &v)?;
                }
                _ => break,
            }
        }
        if let Some(top) = self.levels.last_mut() {
            top.push((v.clone(), op));
        }
        self.value = v;
        self.operand_text = None;
        self.tokens.push(text);
        self.tokens.push(self.op_text(op).into());
        self.last_was_op = true;
        Ok(())
    }

    /// Reduces the innermost level completely, returning the last applied operation.
    fn reduce_level(&mut self, mut v: Num) -> Result<(Num, Option<(BinOp, Num)>), CalcError> {
        let mut last = None;
        while let Some((lhs, op)) = self.levels.last_mut().and_then(|l| l.pop()) {
            last = Some((op, v.clone()));
            v = self.apply(&lhs, op, &v)?;
        }
        Ok((v, last))
    }

    fn close_paren(&mut self) -> Result<(), CalcError> {
        let Some(&mark) = self.paren_marks.last() else {
            return Ok(());
        };
        self.commit();
        let text = self.operand_text();
        let (v, _) = self.reduce_level(self.value.clone())?;
        self.levels.pop();
        self.paren_marks.pop();
        let inner: Vec<String> = self.tokens.drain(mark..).skip(1).collect();
        let body = if inner.is_empty() {
            text
        } else {
            format!("{} {text}", inner.join(" "))
        };
        self.value = v;
        self.operand_text = Some(format!("({body})"));
        self.last_was_op = false;
        Ok(())
    }

    fn equals(&mut self) -> Result<(), CalcError> {
        self.commit();
        let pending = self.levels.iter().any(|l| !l.is_empty());
        if !pending {
            // STD-003: repeated `=` repeats the last operation.
            if let Some((op, rhs)) = self.repeat.clone() {
                let expr = format!(
                    "{} {} {}",
                    self.operand_text(),
                    self.op_text(op),
                    self.format_num(&rhs)
                );
                self.value = self.apply(&self.value, op, &rhs)?;
                self.push_history(expr);
            } else if let Some(t) = self.operand_text.clone() {
                self.push_history(t);
            }
            self.reset_expression();
            return Ok(());
        }
        let mut expr = self.tokens.clone();
        expr.push(self.operand_text());
        expr.extend(std::iter::repeat_n(")".to_string(), self.paren_marks.len()));
        let mut v = self.value.clone();
        let mut last = None;
        while !self.levels.is_empty() {
            let (r, l) = self.reduce_level(v)?;
            v = r;
            last = l.or(last);
            self.levels.pop();
        }
        self.levels.push(Vec::new());
        self.value = v;
        self.repeat = last;
        self.push_history(expr.join(" ").replace("( ", "(").replace(" )", ")"));
        self.reset_expression();
        Ok(())
    }

    fn push_history(&mut self, expr: String) {
        if matches!(self.mode, Mode::Standard | Mode::Scientific) {
            self.history.push(HistoryItem {
                expr,
                result: self.value.clone(),
            });
        }
    }

    /// STD-004: `left × entry / 100`; 0 without a pending operator.
    fn percent(&mut self) -> Result<(), CalcError> {
        if self.mode == Mode::Programmer {
            return Ok(());
        }
        self.commit();
        let lhs = self
            .levels
            .last()
            .and_then(|l| l.last())
            .map_or_else(Num::zero, |(n, _)| n.clone());
        self.value = lhs.mul(&self.value)?.div(&Num::from_i64(100))?;
        self.operand_text = Some(self.format_num(&self.value));
        self.last_was_op = false;
        Ok(())
    }

    fn func(&mut self, f: Func) -> Result<(), CalcError> {
        if !self.enabled(Key::Func(f)) {
            return Ok(());
        }
        self.commit();
        let text = self.operand_text();
        self.value = self.apply_func(f, &self.value)?;
        self.operand_text = Some(if self.mode == Mode::Programmer && f == Func::Not {
            // C syntax: ~x, parenthesized when the operand is an expression.
            if text.contains(' ') && !text.starts_with('(') {
                format!("~({text})")
            } else {
                format!("~{text}")
            }
        } else {
            format!("{}({text})", f.name(self.angle))
        });
        self.last_was_op = false;
        self.inv = false;
        Ok(())
    }

    fn apply(&self, a: &Num, op: BinOp, b: &Num) -> CalcResult {
        if self.mode == Mode::Programmer {
            return self
                .apply_int(self.int(a), op, self.int(b))
                .map(Num::from_i64);
        }
        match op {
            BinOp::Add => a.add(b),
            BinOp::Sub => a.sub(b),
            BinOp::Mul => a.mul(b),
            BinOp::Div => a.div(b),
            BinOp::Mod => a.rem(b),
            BinOp::Pow => a.pow(b),
            BinOp::Root => a.root(b),
            _ => Err(CalcError::InvalidInput),
        }
    }

    /// Fixed-width integer arithmetic, wrapping at the word size (PRG-003, PRG-006).
    fn apply_int(&self, a: i64, op: BinOp, b: i64) -> Result<i64, CalcError> {
        let shift = b.clamp(0, 64) as u32;
        let r = match op {
            BinOp::Add => a.wrapping_add(b),
            BinOp::Sub => a.wrapping_sub(b),
            BinOp::Mul => a.wrapping_mul(b),
            BinOp::Div | BinOp::Mod if b == 0 => {
                return Err(if a == 0 {
                    CalcError::Undefined
                } else {
                    CalcError::DivideByZero
                });
            }
            // Integer division truncates toward zero.
            BinOp::Div => a.wrapping_div(b),
            BinOp::Mod => a.wrapping_rem(b),
            BinOp::And => a & b,
            BinOp::Or => a | b,
            BinOp::Xor => a ^ b,
            BinOp::Lsh => a.checked_shl(shift).unwrap_or(0),
            BinOp::Rsh => a.checked_shr(shift).unwrap_or(if a < 0 { -1 } else { 0 }),
            BinOp::Pow | BinOp::Root => return Err(CalcError::InvalidInput),
        };
        Ok(self.word.wrap(r))
    }

    fn to_radians(&self, x: &Num) -> CalcResult {
        // Reduce whole turns first: exact for exact angles, and fast for huge ones.
        let turn = |t: i64| x.rem(&Num::from_i64(t));
        match self.angle {
            Angle::Radians => Ok(x.clone()),
            Angle::Degrees => turn(360)?.mul(&Num::pi())?.div(&Num::from_i64(180)),
            Angle::Gradians => turn(400)?.mul(&Num::pi())?.div(&Num::from_i64(200)),
        }
    }

    fn radians_to_angle(&self, x: Num) -> CalcResult {
        match self.angle {
            Angle::Radians => Ok(x),
            Angle::Degrees => x.mul(&Num::from_i64(180))?.div(&Num::pi()),
            Angle::Gradians => x.mul(&Num::from_i64(200))?.div(&Num::pi()),
        }
    }

    /// Exact degrees for an exact angle in degrees or gradians, reduced to [0, 360).
    fn exact_degrees(&self, x: &Num) -> Option<i64> {
        let deg = match self.angle {
            Angle::Degrees => x.clone(),
            Angle::Gradians => x
                .mul(&Num::from_i64(9))
                .ok()?
                .div(&Num::from_i64(10))
                .ok()?,
            Angle::Radians => return None,
        };
        if !deg.is_exact() || !deg.is_int() {
            return None;
        }
        // Reduce modulo 360 without overflowing for huge integers.
        let r = deg.rem(&Num::from_i64(360)).ok()?.to_i128() as i64;
        Some(r.rem_euclid(360))
    }

    /// sin/cos/tan with exact results where the value is rational (ENG-004).
    fn trig(&self, f: Func, x: &Num) -> CalcResult {
        if let Some(d) = self.exact_degrees(x) {
            let half = || Num::one().div(&Num::from_i64(2));
            let sin_of = |d: i64| -> Option<CalcResult> {
                Some(match d {
                    0 | 180 => Ok(Num::zero()),
                    90 => Ok(Num::one()),
                    270 => Ok(Num::one().neg()),
                    30 | 150 => half(),
                    210 | 330 => half().map(|h| h.neg()),
                    _ => return None,
                })
            };
            let exact = match f {
                Func::Sin => sin_of(d),
                Func::Cos => sin_of((d + 90) % 360),
                _ => match d % 180 {
                    0 => Some(Ok(Num::zero())),
                    45 => Some(Ok(Num::one())),
                    135 => Some(Ok(Num::one().neg())),
                    90 => Some(Err(CalcError::InvalidInput)),
                    _ => None,
                },
            };
            if let Some(r) = exact {
                return r;
            }
        }
        let r = match self.angle {
            Angle::Radians => x.reduce_turns()?,
            _ => self.to_radians(x)?,
        };
        match f {
            Func::Sin => r.sin(),
            Func::Cos => r.cos(),
            _ => r.tan(),
        }
    }

    /// Inverse trig with exact angles for the rational values of [`Self::trig`].
    fn inverse_trig(&self, f: Func, x: &Num) -> CalcResult {
        if self.angle != Angle::Radians && x.is_exact() {
            let two = Num::from_i64(2);
            let key = x.mul(&two)?; // 2x: exact table lookup on halves
            let k = key.is_int().then(|| key.to_i128());
            let deg = match (f, k) {
                (Func::Asin, Some(-2)) => Some(-90),
                (Func::Asin, Some(-1)) => Some(-30),
                (Func::Asin, Some(0)) => Some(0),
                (Func::Asin, Some(1)) => Some(30),
                (Func::Asin, Some(2)) => Some(90),
                (Func::Acos, Some(-2)) => Some(180),
                (Func::Acos, Some(-1)) => Some(120),
                (Func::Acos, Some(0)) => Some(90),
                (Func::Acos, Some(1)) => Some(60),
                (Func::Acos, Some(2)) => Some(0),
                (Func::Atan, Some(-2)) => Some(-45),
                (Func::Atan, Some(0)) => Some(0),
                (Func::Atan, Some(2)) => Some(45),
                _ => None,
            };
            if let Some(d) = deg {
                let d = Num::from_i64(d);
                return match self.angle {
                    Angle::Gradians => d.mul(&Num::from_i64(10))?.div(&Num::from_i64(9)),
                    _ => Ok(d),
                };
            }
        }
        let r = match f {
            Func::Asin => x.asin()?,
            Func::Acos => x.acos()?,
            _ => x.atan()?,
        };
        self.radians_to_angle(r)
    }

    fn apply_func(&self, f: Func, x: &Num) -> CalcResult {
        let one = Num::one();
        if self.mode == Mode::Programmer {
            let v = self.int(x);
            let bits = self.word.bits();
            let u = (v as u64) & self.word.mask();
            let r = match f {
                Func::Not => !v,
                Func::RoL => ((u << 1) | (u >> (bits - 1))) as i64,
                Func::RoR => ((u >> 1) | ((u & 1) << (bits - 1))) as i64,
                Func::ByteFlip => (u.swap_bytes() >> (64 - bits)) as i64,
                Func::WordFlip => (0..bits / 16)
                    .fold(0u64, |acc, i| (acc << 16) | ((u >> (16 * i)) & 0xFFFF))
                    as i64,
                _ => return Err(CalcError::InvalidInput),
            };
            return Ok(Num::from_i64(self.word.wrap(r)));
        }
        match f {
            Func::Sqrt => x.sqrt(),
            Func::Reciprocal => one.div(x),
            Func::Square => x.mul(x),
            Func::Cube => x.mul(x)?.mul(x),
            Func::CubeRoot => x.cbrt(),
            Func::Pow10 => Num::from_i64(10).pow(x),
            Func::ExpE => x.exp(),
            Func::Ln => x.ln(),
            Func::Log => x.log10(),
            Func::Sin | Func::Cos | Func::Tan => self.trig(f, x),
            Func::Asin | Func::Acos | Func::Atan => self.inverse_trig(f, x),
            Func::Sinh => x.sinh(),
            Func::Cosh => x.cosh(),
            Func::Tanh => x.tanh(),
            Func::Asinh => x.asinh(),
            Func::Acosh => x.acosh(),
            Func::Atanh => x.atanh(),
            Func::Fact => x.factorial(),
            Func::Int => Ok(x.int()),
            Func::Frac => Ok(x.frac()),
            Func::Dms => {
                // Decimal degrees → d.mmss
                let sixty = Num::from_i64(60);
                let d = x.int();
                let m_full = x.sub(&d)?.mul(&sixty)?;
                let m = m_full.int();
                let s = m_full.sub(&m)?.mul(&sixty)?;
                d.add(&m.div(&Num::from_i64(100))?)?
                    .add(&s.div(&Num::from_i64(10000))?)
            }
            Func::Degrees => {
                // d.mmss → decimal degrees
                let d = x.int();
                let rest = x.sub(&d)?.mul(&Num::from_i64(100))?;
                let m = rest.int();
                let s = rest.sub(&m)?.mul(&Num::from_i64(100))?;
                d.add(&m.div(&Num::from_i64(60))?)?
                    .add(&s.div(&Num::from_i64(3600))?)
            }
            Func::Not | Func::RoL | Func::RoR | Func::ByteFlip | Func::WordFlip => {
                Err(CalcError::InvalidInput)
            }
        }
    }

    fn stat(&self, f: StatFn) -> CalcResult {
        let n = Num::from_i64(self.stats.len() as i64);
        let sum = self.stats.iter().try_fold(Num::zero(), |a, x| a.add(x))?;
        let sum_sq = self
            .stats
            .iter()
            .try_fold(Num::zero(), |a, x| a.add(&x.mul(x)?))?;
        // STA-004: functions over an empty list report an error instead of crashing.
        let need = |k: usize| {
            if self.stats.len() < k {
                Err(CalcError::InvalidInput)
            } else {
                Ok(())
            }
        };
        match f {
            StatFn::Sum => Ok(sum),
            StatFn::SumSq => Ok(sum_sq),
            StatFn::Mean => {
                need(1)?;
                sum.div(&n)
            }
            StatFn::MeanSq => {
                need(1)?;
                sum_sq.div(&n)
            }
            StatFn::StdDevN | StatFn::StdDevN1 => {
                need(if f == StatFn::StdDevN { 1 } else { 2 })?;
                let mean = sum.div(&n)?;
                let ss = self.stats.iter().try_fold(Num::zero(), |a, x| {
                    a.add(&x.sub(&mean)?.mul(&x.sub(&mean)?)?)
                })?;
                let d = if f == StatFn::StdDevN {
                    n
                } else {
                    n.sub(&Num::one())?
                };
                let var = ss.div(&d)?;
                if var.compare(&Num::zero()) == Ordering::Less {
                    Ok(Num::zero())
                } else {
                    var.sqrt()
                }
            }
        }
    }
}
