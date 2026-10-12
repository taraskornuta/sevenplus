//! The Win7 calculator state machine, shared by all four modes.
//!
//! Standard and Statistics use immediate execution (`2 + 3 * 4 = 20`); Scientific and
//! Programmer honour operator precedence and parentheses (`2 + 3 * 4 = 14`).

use crate::number::{CalcError, CalcResult, Num, group};
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

    fn mask(self) -> u128 {
        (1u128 << self.bits()) - 1
    }

    /// Truncates `v` to this word size and sign-extends it back.
    pub fn wrap(self, v: i128) -> i128 {
        let u = (v as u128) & self.mask();
        if u >> (self.bits() - 1) & 1 == 1 {
            u as i128 - (1i128 << self.bits())
        } else {
            u as i128
        }
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
        }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cmd {
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

/// Number currently being typed.
#[derive(Debug, Clone, Default)]
struct Entry {
    neg: bool,
    mant: String,
    /// Exponent typed after `Exp`: (negative, digits).
    exp: Option<(bool, String)>,
}

#[derive(Debug, Clone)]
pub struct Calculator {
    pub mode: Mode,
    pub angle: Angle,
    pub base: Base,
    pub word: WordSize,
    pub inv: bool,
    pub fe: bool,
    pub grouping: bool,
    memory: Option<Num>,
    history: Vec<HistoryItem>,
    stats: Vec<Num>,
    entry: Option<Entry>,
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
            grouping: false,
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

    /// Current integer value in programmer mode, as raw bits.
    pub fn bits(&self) -> u64 {
        (self.word.wrap(self.value().to_i128()) as u128 & self.word.mask()) as u64
    }

    pub fn format_num(&self, n: &Num) -> String {
        if self.mode == Mode::Programmer {
            return self.format_int(self.word.wrap(n.to_i128()));
        }
        n.format(self.max_digits(), self.fe, self.grouping)
    }

    fn format_int(&self, v: i128) -> String {
        let (s, n, sep) = match self.base {
            Base::Dec => (v.to_string(), 3, ','),
            b => {
                let u = (v as u128) & self.word.mask();
                let s = match b {
                    Base::Hex => format!("{u:X}"),
                    Base::Oct => format!("{u:o}"),
                    _ => format!("{u:b}"),
                };
                (s, if b == Base::Oct { 3 } else { 4 }, ' ')
            }
        };
        if !self.grouping {
            return s;
        }
        match s.strip_prefix('-') {
            Some(d) => format!("-{}", group(d, n, sep)),
            None => group(&s, n, sep),
        }
    }

    /// Main display text.
    pub fn display(&self) -> String {
        if let Some(e) = self.error {
            return e.to_string();
        }
        let Some(entry) = &self.entry else {
            return self.format_num(&self.value);
        };
        let mut s = String::new();
        if entry.neg {
            s.push('-');
        }
        let (int, frac) = match entry.mant.split_once('.') {
            Some((i, f)) => (i, Some(f)),
            None => (entry.mant.as_str(), None),
        };
        if self.grouping {
            let (n, sep) = match (self.mode, self.base) {
                (Mode::Programmer, Base::Hex | Base::Bin) => (4, ' '),
                (Mode::Programmer, Base::Oct) => (3, ' '),
                _ => (3, ','),
            };
            s.push_str(&group(int, n, sep));
        } else {
            s.push_str(int);
        }
        if let Some(f) = frac {
            s.push('.');
            s.push_str(f);
        }
        if let Some((neg, digits)) = &entry.exp {
            if frac.is_none() {
                s.push('.');
            }
            s.push_str(if *neg { "e-" } else { "e+" });
            s.push_str(if digits.is_empty() { "0" } else { digits });
        }
        s
    }

    /// Expression line shown above the display, e.g. `2 + sqrt(9) *`.
    pub fn expression(&self) -> String {
        let mut parts = self.tokens.clone();
        if let Some(t) = &self.operand_text {
            parts.push(t.clone());
        }
        parts.join(" ").replace("( ", "(")
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

    pub fn set_mode(&mut self, mode: Mode) {
        if self.mode == mode {
            return;
        }
        self.commit();
        self.reset_expression();
        self.error = None;
        self.repeat = None;
        self.mode = mode;
        if mode == Mode::Programmer {
            self.value = Num::from_i128(self.word.wrap(self.value.to_i128()));
            if let Some(m) = &self.memory {
                self.memory = Some(Num::from_i128(self.word.wrap(m.to_i128())));
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
        self.value = Num::from_i128(word.wrap(self.value.to_i128()));
    }

    /// Replaces the display value (history recall, paste, unit conversion…).
    pub fn load_value(&mut self, n: Num) {
        self.error = None;
        self.entry = None;
        self.operand_text = None;
        self.last_was_op = false;
        self.value = if self.mode == Mode::Programmer {
            Num::from_i128(self.word.wrap(n.to_i128()))
        } else {
            n
        };
    }

    // ---- input ---------------------------------------------------------------------

    pub fn press(&mut self, cmd: Cmd) {
        if self.error.is_some() {
            match cmd {
                Cmd::Clear | Cmd::ClearEntry => {}
                Cmd::Digit(_) | Cmd::Point => self.press(Cmd::Clear),
                _ => return,
            }
        }
        if let Err(e) = self.dispatch(cmd) {
            self.error = Some(e);
            self.entry = None;
            self.reset_expression();
        }
    }

    fn dispatch(&mut self, cmd: Cmd) -> Result<(), CalcError> {
        match cmd {
            Cmd::Digit(d) => self.digit(d),
            Cmd::Point => self.point(),
            Cmd::Negate => self.negate()?,
            Cmd::Back => self.back(),
            Cmd::ClearEntry => {
                self.error = None;
                self.entry = None;
                self.operand_text = None;
                self.value = Num::zero();
            }
            Cmd::Clear => {
                let keep = (self.memory.take(), std::mem::take(&mut self.history));
                let stats = std::mem::take(&mut self.stats);
                *self = Calculator {
                    mode: self.mode,
                    angle: self.angle,
                    base: self.base,
                    word: self.word,
                    fe: self.fe,
                    grouping: self.grouping,
                    memory: keep.0,
                    history: keep.1,
                    stats,
                    ..Calculator::new()
                };
            }
            Cmd::Op(op) => self.binary(op)?,
            Cmd::Equals => self.equals()?,
            Cmd::Percent => self.percent()?,
            Cmd::Func(f) => self.func(f)?,
            Cmd::Pi => {
                let pi = Num::pi();
                let v = if self.inv {
                    pi.mul(&Num::from_i64(2))?
                } else {
                    pi
                };
                self.load_value(v);
                self.inv = false;
            }
            Cmd::OpenParen => {
                self.entry = None;
                self.operand_text = None;
                self.last_was_op = false;
                self.levels.push(Vec::new());
                self.paren_marks.push(self.tokens.len());
                self.tokens.push("(".into());
            }
            Cmd::CloseParen => self.close_paren()?,
            Cmd::Exp => self.exp(),
            Cmd::FE => {
                self.commit();
                self.fe = !self.fe;
            }
            Cmd::Inv => self.inv = !self.inv,
            Cmd::MemClear => self.memory = None,
            Cmd::MemRecall => {
                if let Some(m) = self.memory.clone() {
                    self.load_value(m);
                }
            }
            Cmd::MemStore => {
                self.commit();
                self.memory = Some(self.value.clone());
            }
            Cmd::MemAdd | Cmd::MemSub => {
                self.commit();
                let m = self.memory.clone().unwrap_or_else(Num::zero);
                let r = if cmd == Cmd::MemAdd {
                    self.apply(&m, BinOp::Add, &self.value)?
                } else {
                    self.apply(&m, BinOp::Sub, &self.value)?
                };
                self.memory = Some(r);
            }
            Cmd::StatAdd => {
                self.commit();
                self.stats.push(self.value.clone());
                self.value = Num::zero();
                self.operand_text = None;
            }
            Cmd::StatClear => self.stats.clear(),
            Cmd::Stat(f) => {
                let v = self.stat(f)?;
                self.load_value(v);
            }
            Cmd::ToggleBit(i) => {
                if self.mode == Mode::Programmer && i < self.word.bits() {
                    let v = self.word.wrap(self.value().to_i128() ^ (1i128 << i));
                    self.load_value(Num::from_i128(v));
                }
            }
        }
        Ok(())
    }

    fn entry_value(&self, e: &Entry) -> Num {
        let n = if self.mode == Mode::Programmer {
            let u = u128::from_str_radix(&e.mant, self.base.radix()).unwrap_or(0);
            Num::from_i128(self.word.wrap(u as i128))
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
                Num::from_i128(self.word.wrap(v.to_i128()))
            } else {
                v
            };
            self.operand_text = None;
        }
    }

    fn start_entry(&mut self) -> &mut Entry {
        if self.entry.is_none() {
            self.operand_text = None;
            self.last_was_op = false;
            self.entry = Some(Entry {
                mant: "0".into(),
                ..Entry::default()
            });
        }
        self.entry.as_mut().unwrap()
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
        let c = char::from_digit(d as u32, radix)
            .unwrap()
            .to_ascii_uppercase();
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
            u128::from_str_radix(&candidate, radix).is_ok_and(|v| v <= limit)
        } else {
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
            let s = self.value.format(32, false, false);
            let (mant, exp) = match s.split_once('e') {
                Some((m, x)) => (m.trim_end_matches('.').to_string(), Some(x.to_string())),
                None => (s, None),
            };
            let neg = mant.starts_with('-');
            self.entry = Some(Entry {
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
        let e = self.entry.as_mut().unwrap();
        if e.exp.is_none() {
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
        let v = if self.mode == Mode::Programmer {
            Num::from_i128(self.word.wrap(-self.value.to_i128()))
        } else {
            self.value.neg()
        };
        self.value = v;
        self.operand_text = Some(format!("negate({text})"));
        self.last_was_op = false;
        Ok(())
    }

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

    fn binary(&mut self, op: BinOp) -> Result<(), CalcError> {
        if self.last_was_op {
            // Pressing another operator replaces the pending one.
            if let Some(last) = self.levels.last_mut().and_then(|l| l.last_mut()) {
                last.1 = op;
                *self.tokens.last_mut().unwrap() = op.symbol().into();
                return Ok(());
            }
        }
        self.commit();
        let text = self.operand_text();
        let mut v = self.value.clone();
        let immediate = !self.precedence();
        loop {
            let top = self.levels.last_mut().unwrap();
            match top.last() {
                Some((_, pop)) if immediate || pop.prec() >= op.prec() => {
                    let (lhs, pop) = top.pop().unwrap();
                    v = self.apply(&lhs, pop, &v)?;
                }
                _ => break,
            }
        }
        self.levels.last_mut().unwrap().push((v.clone(), op));
        self.value = v;
        self.operand_text = None;
        self.tokens.push(text);
        self.tokens.push(op.symbol().into());
        self.last_was_op = true;
        Ok(())
    }

    /// Reduces the innermost level completely, returning the last applied operation.
    fn reduce_level(&mut self, mut v: Num) -> Result<(Num, Option<(BinOp, Num)>), CalcError> {
        let mut last = None;
        while let Some((lhs, op)) = self.levels.last_mut().unwrap().pop() {
            last = Some((op, v.clone()));
            v = self.apply(&lhs, op, &v)?;
        }
        Ok((v, last))
    }

    fn close_paren(&mut self) -> Result<(), CalcError> {
        if self.paren_marks.is_empty() {
            return Ok(());
        }
        self.commit();
        let text = self.operand_text();
        let (v, _) = self.reduce_level(self.value.clone())?;
        self.levels.pop();
        let mark = self.paren_marks.pop().unwrap();
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
            if let Some((op, rhs)) = self.repeat.clone() {
                let expr = format!(
                    "{} {} {}",
                    self.operand_text(),
                    op.symbol(),
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
        self.history.push(HistoryItem {
            expr,
            result: self.value.clone(),
        });
    }

    fn percent(&mut self) -> Result<(), CalcError> {
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
        self.commit();
        let text = self.operand_text();
        self.value = self.apply_func(f, &self.value)?;
        self.operand_text = Some(format!("{}({text})", f.name(self.angle)));
        self.last_was_op = false;
        self.inv = false;
        Ok(())
    }

    fn apply(&self, a: &Num, op: BinOp, b: &Num) -> CalcResult {
        if self.mode == Mode::Programmer {
            return self
                .apply_int(a.to_i128(), op, b.to_i128())
                .map(Num::from_i128);
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

    fn apply_int(&self, a: i128, op: BinOp, b: i128) -> Result<i128, CalcError> {
        let shift = b.clamp(0, 127) as u32;
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
            BinOp::Div => a.wrapping_div(b),
            BinOp::Mod => a.wrapping_rem(b),
            BinOp::And => a & b,
            BinOp::Or => a | b,
            BinOp::Xor => a ^ b,
            BinOp::Lsh => a.checked_shl(shift).unwrap_or(0),
            BinOp::Rsh => a >> shift,
            BinOp::Pow | BinOp::Root => return Err(CalcError::InvalidInput),
        };
        Ok(self.word.wrap(r))
    }

    fn to_radians(&self, x: &Num) -> CalcResult {
        match self.angle {
            Angle::Radians => Ok(x.clone()),
            Angle::Degrees => x.mul(&Num::pi())?.div(&Num::from_i64(180)),
            Angle::Gradians => x.mul(&Num::pi())?.div(&Num::from_i64(200)),
        }
    }

    fn radians_to_angle(&self, x: Num) -> CalcResult {
        match self.angle {
            Angle::Radians => Ok(x),
            Angle::Degrees => x.mul(&Num::from_i64(180))?.div(&Num::pi()),
            Angle::Gradians => x.mul(&Num::from_i64(200))?.div(&Num::pi()),
        }
    }

    /// sin/cos/tan, exact at multiples of a right angle in degrees and gradians.
    fn trig(&self, f: Func, x: &Num) -> CalcResult {
        let quarter = match self.angle {
            Angle::Degrees => Some(90),
            Angle::Gradians => Some(100),
            Angle::Radians => None,
        };
        if let Some(q) = quarter {
            let k = x.div(&Num::from_i64(q))?;
            if k.is_int() {
                let k = k.to_i128().rem_euclid(4) as usize;
                let one = Num::from_i64(1);
                return match f {
                    Func::Sin => Ok([Num::zero(), one.clone(), Num::zero(), one.neg()][k].clone()),
                    Func::Cos => Ok([one.clone(), Num::zero(), one.neg(), Num::zero()][k].clone()),
                    _ if k % 2 == 1 => Err(CalcError::InvalidInput),
                    _ => Ok(Num::zero()),
                };
            }
        }
        let r = self.to_radians(x)?;
        match f {
            Func::Sin => r.sin(),
            Func::Cos => r.cos(),
            _ => r.tan(),
        }
    }

    fn apply_func(&self, f: Func, x: &Num) -> CalcResult {
        let one = Num::from_i64(1);
        if self.mode == Mode::Programmer {
            let v = x.to_i128();
            let bits = self.word.bits();
            let u = (v as u128) & self.word.mask();
            let r = match f {
                Func::Not => !v,
                Func::RoL => ((u << 1) | (u >> (bits - 1))) as i128,
                Func::RoR => ((u >> 1) | ((u & 1) << (bits - 1))) as i128,
                _ => return Err(CalcError::InvalidInput),
            };
            return Ok(Num::from_i128(self.word.wrap(r)));
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
            Func::Asin => self.radians_to_angle(x.asin()?),
            Func::Acos => self.radians_to_angle(x.acos()?),
            Func::Atan => self.radians_to_angle(x.atan()?),
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
            Func::Not | Func::RoL | Func::RoR => Err(CalcError::InvalidInput),
        }
    }

    fn stat(&self, f: StatFn) -> CalcResult {
        let n = Num::from_i64(self.stats.len() as i64);
        let sum = self.stats.iter().try_fold(Num::zero(), |a, x| a.add(x))?;
        let sum_sq = self
            .stats
            .iter()
            .try_fold(Num::zero(), |a, x| a.add(&x.mul(x)?))?;
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
                    n.sub(&Num::from_i64(1))?
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
