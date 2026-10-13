//! Calculator numbers: exact rationals where possible, high-precision floats otherwise.
//!
//! Parsing and `+ − × ÷`, `Mod`, `%`, integer powers and perfect roots stay exact
//! ([`BigRational`], ENG-002), so `1/3*3` is exactly 1 and `0.1+0.2` exactly 0.3.
//! Transcendental functions switch to a 256-bit binary float (~77 significant decimal
//! digits, well above the 40 required by ENG-001), which then stays approximate.

use astro_float::{BigFloat, Consts, Radix, RoundingMode, Sign};
use calc_format::Decimal;
use num_bigint::BigInt;
use num_integer::Integer;
use num_rational::BigRational;
use num_traits::{One, Pow, Signed, ToPrimitive, Zero};
use std::cell::RefCell;
use std::cmp::Ordering;
use std::fmt;

/// Working precision of approximate values in bits.
const P: usize = 256;
const RM: RoundingMode = RoundingMode::ToEven;
/// Win7 reports overflow at 1e10000 (≈ 2^33219), ENG-007.
const MAX_BIN_EXP: i32 = 33219;
/// Results this close to zero are treated as zero (e.g. sin(π)).
const SNAP_BIN_EXP: i32 = -200;
/// Decimal exponent bound of the display range (ENG-007).
const MAX_DEC_EXP: u32 = 10000;
/// Exact values whose numerator plus denominator exceed this many bits become
/// approximate, keeping every operation fast (ENG-009).
const MAX_EXACT_BITS: u64 = 70_000;
/// Approximate values carry this many decimal digits; the rest is binary noise.
const APPROX_DIGITS: usize = 70;

thread_local! {
    static CC: RefCell<Consts> = RefCell::new(Consts::new().expect("astro-float constants cache"));
}

fn with_cc<R>(f: impl FnOnce(&mut Consts) -> R) -> R {
    CC.with(|c| f(&mut c.borrow_mut()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalcError {
    DivideByZero,
    Undefined,
    InvalidInput,
    Overflow,
}

impl fmt::Display for CalcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            CalcError::DivideByZero => "Cannot divide by zero",
            CalcError::Undefined => "Result is undefined",
            CalcError::InvalidInput => "Invalid input",
            CalcError::Overflow => "Overflow",
        })
    }
}

pub type CalcResult = Result<Num, CalcError>;

#[derive(Clone, Debug)]
enum Repr {
    Exact(BigRational),
    Approx(BigFloat),
}

#[derive(Clone, Debug)]
pub struct Num(Repr);

fn big10(e: u32) -> BigInt {
    BigInt::from(10u32).pow(e)
}

fn ratio(n: impl Into<BigInt>) -> BigRational {
    BigRational::from_integer(n.into())
}

/// Converts a big integer to a float, keeping only its top bits so huge values convert
/// quickly.
fn int_to_float(n: &BigInt) -> BigFloat {
    let bits = n.bits();
    let keep = P as u64 + 64;
    let (top, shift) = if bits > keep {
        (n >> (bits - keep), bits - keep)
    } else {
        (n.clone(), 0)
    };
    let mut f = with_cc(|cc| BigFloat::parse(&top.to_string(), Radix::Dec, P, RM, cc));
    if shift > 0
        && let Some(e) = f.exponent()
    {
        f.set_exponent(e.saturating_add(shift.min(i32::MAX as u64) as i32));
    }
    f
}

impl Num {
    pub fn zero() -> Self {
        Num(Repr::Exact(BigRational::zero()))
    }

    pub fn one() -> Self {
        Self::from_i64(1)
    }

    pub fn from_i64(v: i64) -> Self {
        Num(Repr::Exact(ratio(v)))
    }

    pub fn from_i128(v: i128) -> Self {
        Num(Repr::Exact(ratio(v)))
    }

    pub fn from_f64(v: f64) -> Self {
        // Go through the shortest decimal representation so 0.1 stays 0.1.
        Self::parse(&format!("{v:e}")).unwrap_or_else(Self::zero)
    }

    fn ratio_of(n: i64, d: i64) -> Self {
        Num(Repr::Exact(BigRational::new(n.into(), d.into())))
    }

    /// Parses a canonical decimal number such as `-12.5`, `1.e+5` or `3e-2`. Group
    /// separators (`,`) are ignored.
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim().replace(',', "");
        let (neg, s) = match s.strip_prefix('-') {
            Some(r) => (true, r),
            None => (false, s.strip_prefix('+').unwrap_or(&s)),
        };
        let (mant, exp) = match s.split_once(['e', 'E']) {
            Some((m, e)) => (m, e.parse::<i64>().ok()?),
            None => (s, 0),
        };
        let (int, frac) = mant.split_once('.').unwrap_or((mant, ""));
        if int.is_empty() && frac.is_empty()
            || !int.chars().chain(frac.chars()).all(|c| c.is_ascii_digit())
        {
            return None;
        }
        let digits = format!("{int}{frac}");
        let m: BigInt = digits.parse().ok()?;
        let scale = exp - frac.len() as i64;
        if scale.unsigned_abs() > (MAX_DEC_EXP + 100) as u64 + digits.len() as u64 {
            return None;
        }
        let mut r = if scale >= 0 {
            ratio(m * big10(scale as u32))
        } else {
            BigRational::new(m, big10((-scale) as u32))
        };
        if neg {
            r = -r;
        }
        Some(Num(Repr::Exact(r)))
    }

    pub fn pi() -> Self {
        Num(Repr::Approx(with_cc(|cc| cc.pi(P, RM))))
    }

    pub fn is_exact(&self) -> bool {
        matches!(self.0, Repr::Exact(_))
    }

    fn float(&self) -> BigFloat {
        match &self.0 {
            Repr::Approx(f) => f.clone(),
            Repr::Exact(r) => {
                let n = int_to_float(r.numer());
                if r.denom().is_one() {
                    n
                } else {
                    n.div(&int_to_float(r.denom()), P, RM)
                }
            }
        }
    }

    pub fn is_zero(&self) -> bool {
        match &self.0 {
            Repr::Exact(r) => r.is_zero(),
            Repr::Approx(f) => f.is_zero(),
        }
    }

    pub fn is_negative(&self) -> bool {
        match &self.0 {
            Repr::Exact(r) => r.is_negative(),
            Repr::Approx(f) => f.is_negative() && !f.is_zero(),
        }
    }

    pub fn is_int(&self) -> bool {
        match &self.0 {
            Repr::Exact(r) => r.is_integer(),
            Repr::Approx(f) => f.is_zero() || f.is_int(),
        }
    }

    pub fn neg(&self) -> Self {
        Num(match &self.0 {
            Repr::Exact(r) => Repr::Exact(-r),
            Repr::Approx(f) => Repr::Approx(f.neg()),
        })
    }

    pub fn abs(&self) -> Self {
        Num(match &self.0 {
            Repr::Exact(r) => Repr::Exact(r.abs()),
            Repr::Approx(f) => Repr::Approx(f.abs()),
        })
    }

    /// Integer part (truncates toward zero).
    pub fn int(&self) -> Self {
        Num(match &self.0 {
            Repr::Exact(r) => Repr::Exact(r.trunc()),
            Repr::Approx(f) => Repr::Approx(f.int()),
        })
    }

    pub fn frac(&self) -> Self {
        Num(match &self.0 {
            Repr::Exact(r) => Repr::Exact(r.fract()),
            Repr::Approx(f) => Repr::Approx(f.fract()),
        })
    }

    pub fn floor(&self) -> Self {
        Num(match &self.0 {
            Repr::Exact(r) => Repr::Exact(r.floor()),
            Repr::Approx(f) => Repr::Approx(f.floor()),
        })
    }

    fn check_float(n: BigFloat) -> CalcResult {
        if n.is_nan() {
            return Err(CalcError::InvalidInput);
        }
        if n.is_inf() {
            return Err(CalcError::Overflow);
        }
        match n.exponent() {
            Some(e) if e > MAX_BIN_EXP => Err(CalcError::Overflow),
            Some(e) if e < SNAP_BIN_EXP - 33000 => Ok(Num::zero()),
            _ => Ok(Num(Repr::Approx(n))),
        }
    }

    /// Range-checks an exact result: |x| ≥ 1e10000 overflows, |x| < 1e-9999 becomes
    /// zero, and very large fractions become approximate.
    fn check_exact(r: BigRational) -> CalcResult {
        if r.is_zero() {
            return Ok(Num(Repr::Exact(r)));
        }
        let (nb, db) = (r.numer().bits() as i64, r.denom().bits() as i64);
        let log2 = nb - db;
        if log2 > MAX_BIN_EXP as i64 - 4 && r.abs() >= ratio(big10(MAX_DEC_EXP)) {
            return Err(CalcError::Overflow);
        }
        if log2 < -(MAX_BIN_EXP as i64) + 4
            && r.abs() < BigRational::new(BigInt::one(), big10(MAX_DEC_EXP - 1))
        {
            return Ok(Num::zero());
        }
        if (nb + db) as u64 > MAX_EXACT_BITS {
            return Self::check_float(Num(Repr::Exact(r)).float());
        }
        Ok(Num(Repr::Exact(r)))
    }

    /// Snaps tiny results (from transcendental functions) to zero.
    fn snapped(self) -> Self {
        match &self.0 {
            Repr::Approx(f) => match f.exponent() {
                Some(e) if !f.is_zero() && e < SNAP_BIN_EXP => Num::zero(),
                _ => self,
            },
            Repr::Exact(_) => self,
        }
    }

    fn approx(&self, f: impl FnOnce(&BigFloat, &mut Consts) -> BigFloat) -> CalcResult {
        let x = self.float();
        Self::check_float(with_cc(|cc| f(&x, cc))).map(Self::snapped)
    }

    pub fn add(&self, o: &Num) -> CalcResult {
        match (&self.0, &o.0) {
            (Repr::Exact(a), Repr::Exact(b)) => Self::check_exact(a + b),
            _ => Self::check_float(self.float().add(&o.float(), P, RM)),
        }
    }

    pub fn sub(&self, o: &Num) -> CalcResult {
        match (&self.0, &o.0) {
            (Repr::Exact(a), Repr::Exact(b)) => Self::check_exact(a - b),
            _ => Self::check_float(self.float().sub(&o.float(), P, RM)),
        }
    }

    pub fn mul(&self, o: &Num) -> CalcResult {
        match (&self.0, &o.0) {
            (Repr::Exact(a), Repr::Exact(b)) => Self::check_exact(a * b),
            _ => Self::check_float(self.float().mul(&o.float(), P, RM)),
        }
    }

    pub fn div(&self, o: &Num) -> CalcResult {
        if o.is_zero() {
            return Err(if self.is_zero() {
                CalcError::Undefined
            } else {
                CalcError::DivideByZero
            });
        }
        match (&self.0, &o.0) {
            (Repr::Exact(a), Repr::Exact(b)) => Self::check_exact(a / b),
            _ => Self::check_float(self.float().div(&o.float(), P, RM)),
        }
    }

    /// Truncating remainder (sign follows the dividend).
    pub fn rem(&self, o: &Num) -> CalcResult {
        if o.is_zero() {
            return Err(CalcError::Undefined);
        }
        if let (Repr::Exact(a), Repr::Exact(b)) = (&self.0, &o.0) {
            // a/b mod c/d = (a·d mod c·b) / (b·d), truncating like `%` on integers.
            let (n, d) = (a.numer() * b.denom(), b.numer() * a.denom());
            let den = a.denom() * b.denom();
            return Self::check_exact(BigRational::new(n % d, den));
        }
        let q = self.div(o)?.int();
        self.sub(&q.mul(o)?)
    }

    /// Exact `k`-th root of an exact value, if it is rational.
    fn exact_root(&self, k: u32) -> Option<Num> {
        let Repr::Exact(r) = &self.0 else {
            return None;
        };
        if (r.numer().bits() + r.denom().bits()) > 4096 {
            return None;
        }
        let root = |n: &BigInt| {
            let x = n.abs().nth_root(k);
            (Pow::pow(&x, k) == n.abs()).then_some(x)
        };
        let (n, d) = (root(r.numer())?, root(r.denom())?);
        let v = BigRational::new(n, d);
        Some(Num(Repr::Exact(if r.is_negative() { -v } else { v })))
    }

    pub fn sqrt(&self) -> CalcResult {
        if self.is_negative() {
            return Err(CalcError::InvalidInput);
        }
        if let Some(r) = self.exact_root(2) {
            return Ok(r);
        }
        self.approx(|x, _| x.sqrt(P, RM))
    }

    pub fn cbrt(&self) -> CalcResult {
        if let Some(r) = self.exact_root(3) {
            return Ok(r);
        }
        self.approx(|x, _| x.cbrt(P, RM))
    }

    pub fn pow(&self, e: &Num) -> CalcResult {
        if self.is_zero() {
            return if e.is_negative() {
                Err(CalcError::DivideByZero)
            } else if e.is_zero() {
                Err(CalcError::Undefined)
            } else {
                Ok(Num::zero())
            };
        }
        if self.is_negative() {
            if !e.is_int() {
                return Err(CalcError::InvalidInput);
            }
            let r = self.abs().pow(e)?;
            return Ok(if e.is_odd_int() { r.neg() } else { r });
        }
        if let (Repr::Exact(b), Repr::Exact(x)) = (&self.0, &e.0)
            && x.is_integer()
            && let Some(n) = x.to_integer().to_i32()
        {
            let size = (b.numer().bits() + b.denom().bits()) * n.unsigned_abs() as u64;
            if size <= MAX_EXACT_BITS {
                return Self::check_exact(Pow::pow(b, n));
            }
        }
        if e.is_int() && e.abs().compare(&Num::from_i64(1 << 20)) == Ordering::Less {
            let n = e.to_i128().unsigned_abs() as usize;
            let r = Self::check_float(self.float().powi(n, P, RM))?;
            return if e.is_negative() {
                Num::one().div(&r)
            } else {
                Ok(r)
            };
        }
        let (b, x) = (self.float(), e.float());
        Self::check_float(with_cc(|cc| b.pow(&x, P, RM, cc)))
    }

    fn is_odd_int(&self) -> bool {
        self.is_int()
            && self
                .rem(&Num::from_i64(2))
                .is_ok_and(|r| r.abs().compare(&Num::one()) == Ordering::Equal)
    }

    /// The `y`-th root of `self` (Win7 `y√x`).
    pub fn root(&self, y: &Num) -> CalcResult {
        if y.is_zero() {
            return Err(CalcError::InvalidInput);
        }
        if self.is_negative() {
            // Only odd integer roots of negative numbers are real.
            if !y.is_odd_int() {
                return Err(CalcError::InvalidInput);
            }
            return Ok(self.abs().root(y)?.neg());
        }
        if y.is_int()
            && !y.is_negative()
            && let Some(k) = y.to_i128().to_u32().filter(|k| *k <= 64)
            && let Some(r) = self.exact_root(k)
        {
            return Ok(r);
        }
        self.pow(&Num::one().div(y)?)
    }

    pub fn ln(&self) -> CalcResult {
        if self.is_negative() || self.is_zero() {
            return Err(CalcError::InvalidInput);
        }
        if let Repr::Exact(r) = &self.0
            && r.is_one()
        {
            return Ok(Num::zero());
        }
        self.approx(|x, cc| x.ln(P, RM, cc))
    }

    pub fn log10(&self) -> CalcResult {
        if self.is_negative() || self.is_zero() {
            return Err(CalcError::InvalidInput);
        }
        if let Some(k) = self.exact_power_of_ten() {
            return Ok(Num::from_i64(k));
        }
        self.approx(|x, cc| x.log10(P, RM, cc))
    }

    /// `k` when the value is exactly `10^k`.
    fn exact_power_of_ten(&self) -> Option<i64> {
        let Repr::Exact(r) = &self.0 else {
            return None;
        };
        let is_pow = |n: &BigInt| {
            let s = n.to_string();
            (s.starts_with('1') && s[1..].bytes().all(|b| b == b'0')).then(|| s.len() as i64 - 1)
        };
        match (is_pow(r.numer())?, is_pow(r.denom())?) {
            (k, 0) => Some(k),
            (0, k) => Some(-k),
            _ => None,
        }
    }

    pub fn exp(&self) -> CalcResult {
        if self.is_zero() {
            return Ok(Num::one());
        }
        self.approx(|x, cc| x.exp(P, RM, cc))
    }

    /// Reduces an exact angle in radians modulo 2π. The reduction runs at a precision
    /// that covers the whole integer part, so huge arguments keep full accuracy
    /// (ENG-003) instead of first being rounded to the working precision.
    pub fn reduce_turns(&self) -> CalcResult {
        let Repr::Exact(r) = &self.0 else {
            return Ok(self.clone());
        };
        let log2 = r.numer().bits() as i64 - r.denom().bits() as i64;
        if log2 < 64 {
            return Ok(self.clone());
        }
        let prec = (log2 as usize + P + 64).next_multiple_of(64);
        let conv =
            |n: &BigInt| with_cc(|cc| BigFloat::parse(&n.to_string(), Radix::Dec, prec, RM, cc));
        let x = conv(r.numer()).div(&conv(r.denom()), prec, RM);
        let two_pi = with_cc(|cc| cc.pi(prec, RM)).mul(&BigFloat::from_i32(2, prec), prec, RM);
        let k = x.div(&two_pi, prec, RM).floor();
        let mut red = x.sub(&k.mul(&two_pi, prec, RM), prec, RM);
        red.set_precision(P, RM)
            .map_err(|_| CalcError::InvalidInput)?;
        Self::check_float(red)
    }

    pub fn sin(&self) -> CalcResult {
        self.approx(|x, cc| x.sin(P, RM, cc))
    }

    pub fn cos(&self) -> CalcResult {
        self.approx(|x, cc| x.cos(P, RM, cc))
    }

    pub fn tan(&self) -> CalcResult {
        self.approx(|x, cc| x.tan(P, RM, cc))
    }

    fn check_unit(&self) -> Result<(), CalcError> {
        if self.abs().compare(&Num::one()) == Ordering::Greater {
            return Err(CalcError::InvalidInput);
        }
        Ok(())
    }

    pub fn asin(&self) -> CalcResult {
        self.check_unit()?;
        self.approx(|x, cc| x.asin(P, RM, cc))
    }

    pub fn acos(&self) -> CalcResult {
        self.check_unit()?;
        self.approx(|x, cc| x.acos(P, RM, cc))
    }

    pub fn atan(&self) -> CalcResult {
        self.approx(|x, cc| x.atan(P, RM, cc))
    }

    pub fn sinh(&self) -> CalcResult {
        self.approx(|x, cc| x.sinh(P, RM, cc))
    }

    pub fn cosh(&self) -> CalcResult {
        if self.is_zero() {
            return Ok(Num::one());
        }
        let x = self.float();
        Self::check_float(with_cc(|cc| x.cosh(P, RM, cc)))
    }

    pub fn tanh(&self) -> CalcResult {
        self.approx(|x, cc| x.tanh(P, RM, cc))
    }

    pub fn asinh(&self) -> CalcResult {
        self.approx(|x, cc| x.asinh(P, RM, cc))
    }

    pub fn acosh(&self) -> CalcResult {
        if self.compare(&Num::one()) == Ordering::Less {
            return Err(CalcError::InvalidInput);
        }
        self.approx(|x, cc| x.acosh(P, RM, cc))
    }

    pub fn atanh(&self) -> CalcResult {
        if self.abs().compare(&Num::one()) != Ordering::Less {
            return Err(CalcError::InvalidInput);
        }
        self.approx(|x, cc| x.atanh(P, RM, cc))
    }

    pub fn factorial(&self) -> CalcResult {
        if self.is_int() {
            if self.is_negative() {
                return Err(CalcError::InvalidInput);
            }
            // 3249! exceeds 1e10000: answer immediately instead of computing (ENG-009).
            if self.compare(&Num::from_i64(3248)) == Ordering::Greater {
                return Err(CalcError::Overflow);
            }
            let n = self.to_i128() as u32;
            let mut r = BigInt::one();
            for i in 2..=n {
                r *= i;
            }
            return Self::check_exact(ratio(r));
        }
        // Non-integers use the gamma function, like Win7 (0.5! = 0.886…).
        gamma(&self.add(&Num::one())?)
    }

    pub fn compare(&self, o: &Num) -> Ordering {
        match (&self.0, &o.0) {
            (Repr::Exact(a), Repr::Exact(b)) => a.cmp(b),
            _ => match self.float().cmp(&o.float()) {
                Some(c) if c < 0 => Ordering::Less,
                Some(c) if c > 0 => Ordering::Greater,
                _ => Ordering::Equal,
            },
        }
    }

    /// The first `sig` significant decimal digits (truncated), for display formatting.
    pub fn to_decimal(&self, sig: usize) -> Decimal {
        match &self.0 {
            Repr::Exact(r) => exact_decimal(r, sig.max(1)),
            Repr::Approx(f) => approx_decimal(f, sig.max(1)),
        }
    }

    pub fn to_f64(&self) -> f64 {
        match &self.0 {
            Repr::Exact(r) => r.to_f64().unwrap_or(f64::NAN),
            Repr::Approx(_) => {
                let d = self.to_decimal(20);
                let m: String = d.digits.iter().map(|x| (b'0' + x) as char).collect();
                let s = format!("{}0.{}e{}", if d.neg { "-" } else { "" }, m, d.exp);
                s.parse().unwrap_or(f64::NAN)
            }
        }
    }

    /// Integer value, truncated toward zero. Saturates outside the i128 range.
    pub fn to_i128(&self) -> i128 {
        let t = match &self.0 {
            Repr::Exact(r) => r.trunc().to_integer(),
            Repr::Approx(_) => {
                let d = self.int().to_decimal(45);
                if d.exp > 39 {
                    return if d.neg { i128::MIN } else { i128::MAX };
                }
                let mut v = BigInt::zero();
                for i in 0..d.exp.max(0) as usize {
                    v = v * 10 + *d.digits.get(i).unwrap_or(&0);
                }
                if d.neg { -v } else { v }
            }
        };
        t.to_i128().unwrap_or(if t.is_negative() {
            i128::MIN
        } else {
            i128::MAX
        })
    }
}

/// Decimal digits of an exact rational by long division.
fn exact_decimal(r: &BigRational, sig: usize) -> Decimal {
    if r.is_zero() {
        return Decimal::default();
    }
    let n = r.numer().abs();
    let d = r.denom().clone();
    // Estimate exp with 10^(exp-1) <= n/d < 10^exp, then correct it.
    let log2 = n.bits() as f64 - d.bits() as f64;
    let mut exp = (log2 * std::f64::consts::LOG10_2).floor() as i32;
    let at_least = |e: i32| -> bool {
        // n/d >= 10^e
        if e >= 0 {
            n >= &d * big10(e as u32)
        } else {
            &n * big10((-e) as u32) >= d
        }
    };
    while at_least(exp) {
        exp += 1;
    }
    while !at_least(exp - 1) {
        exp -= 1;
    }
    // q = floor(n/d * 10^(sig - exp)), which has exactly `sig` digits.
    let shift = sig as i64 - exp as i64;
    let (num, den) = if shift >= 0 {
        (n * big10(shift as u32), d)
    } else {
        (n, d * big10((-shift) as u32))
    };
    let (q, rem) = num.div_rem(&den);
    let mut digits: Vec<u8> = q.to_string().bytes().map(|b| b - b'0').collect();
    while digits.last() == Some(&0) {
        digits.pop();
    }
    Decimal {
        neg: r.is_negative(),
        digits,
        exp,
        inexact: !rem.is_zero(),
    }
}

fn approx_decimal(f: &BigFloat, sig: usize) -> Decimal {
    if f.is_zero() {
        return Decimal::default();
    }
    let Ok((sign, mut digits, exp)) = with_cc(|cc| f.convert_to_radix(Radix::Dec, RM, cc)) else {
        return Decimal::default();
    };
    let lead = digits.iter().take_while(|&&d| d == 0).count();
    digits.drain(..lead);
    let exp = exp - lead as i32;
    let keep = sig.min(APPROX_DIGITS);
    let inexact = digits.len() > keep && digits[keep..].iter().any(|&d| d != 0);
    digits.truncate(keep);
    while digits.last() == Some(&0) {
        digits.pop();
    }
    Decimal {
        neg: sign == Sign::Neg,
        digits,
        exp,
        inexact,
    }
}

/// Bernoulli numbers B2..B30 for the Stirling series.
const BERNOULLI: [(i64, i64); 15] = [
    (1, 6),
    (-1, 30),
    (1, 42),
    (-1, 30),
    (5, 66),
    (-691, 2730),
    (7, 6),
    (-3617, 510),
    (43867, 798),
    (-174611, 330),
    (854513, 138),
    (-236364091, 2730),
    (8553103, 6),
    (-23749461029, 870),
    (8615841276005, 14322),
];

/// Gamma function for non-integers, accurate well beyond the 32 displayed digits.
fn gamma(z: &Num) -> CalcResult {
    let one = Num::one();
    let half = Num::ratio_of(1, 2);
    if z.compare(&half) == Ordering::Less {
        // Reflection: Γ(z) = π / (sin(πz) · Γ(1 − z))
        let pi = Num::pi();
        let s = pi.mul(z)?.sin()?;
        return pi.div(&s.mul(&gamma(&one.sub(z)?)?)?);
    }
    // Shift z up so the asymptotic series converges fast, then divide the shift back out.
    let mut z = z.clone();
    let mut shift = one.clone();
    while z.compare(&Num::from_i64(50)) == Ordering::Less {
        shift = shift.mul(&z)?;
        z = z.add(&one)?;
    }
    let two_pi = Num::pi().mul(&Num::from_i64(2))?;
    let mut lg = z
        .sub(&half)?
        .mul(&z.ln()?)?
        .sub(&z)?
        .add(&half.mul(&two_pi.ln()?)?)?;
    let z2 = z.mul(&z)?;
    let mut zp = z.clone();
    for (k, (num, den)) in BERNOULLI.iter().enumerate() {
        let k2 = 2 * (k as i64 + 1);
        let b = Num::ratio_of(*num, *den);
        lg = lg.add(&b.div(&Num::from_i64(k2 * (k2 - 1)).mul(&zp)?)?)?;
        zp = zp.mul(&z2)?;
    }
    lg.exp()?.div(&shift)
}

#[cfg(test)]
mod tests {
    use super::*;
    use calc_format::{FormatSettings, Locale, Value, format};

    fn n(s: &str) -> Num {
        Num::parse(s).unwrap()
    }

    fn fmt(x: &Num, digits: usize) -> String {
        let s = FormatSettings {
            max_digits: digits,
            ..Default::default()
        };
        format(&Value::Dec(x.to_decimal(digits + 8)), &s, &Locale::EN_US)
    }

    fn f(x: &Num) -> String {
        fmt(x, 16)
    }

    #[test]
    fn exact_arithmetic() {
        let third = n("1").div(&n("3")).unwrap();
        assert!(third.is_exact());
        assert_eq!(f(&third.mul(&n("3")).unwrap()), "1");
        assert_eq!(f(&n("0.1").add(&n("0.2")).unwrap()), "0.3");
        assert_eq!(f(&third), "0.3333333333333333");
        assert_eq!(f(&n("2").div(&n("3")).unwrap()), "0.6666666666666667");
        assert_eq!(f(&n("1").div(&n("7000")).unwrap()), "1.428571428571429e-4");
        assert_eq!(f(&n("4").sqrt().unwrap()), "2");
        assert!(n("0.25").sqrt().unwrap().is_exact());
        assert!(!n("2").sqrt().unwrap().is_exact());
        assert_eq!(f(&n("1e3").log10().unwrap()), "3");
        assert_eq!(f(&n("0.001").log10().unwrap()), "-3");
    }

    #[test]
    fn conversions() {
        assert_eq!(n("-42.9").to_i128(), -42);
        assert_eq!(n("12345678901234567890").to_i128(), 12345678901234567890);
        assert_eq!(n("0.25").to_f64(), 0.25);
        assert_eq!(f(&Num::from_f64(0.1)), "0.1");
        assert_eq!(n("1.e+5").to_i128(), 100000);
        assert_eq!(Num::pi().int().to_i128(), 3);
        assert!(Num::parse("abc").is_none());
        assert!(Num::parse(".").is_none());
    }

    #[test]
    fn functions() {
        assert_eq!(f(&n("5").factorial().unwrap()), "120");
        assert_eq!(
            fmt(&n("0.5").factorial().unwrap(), 32),
            "0.88622692545275801364908374167057"
        );
        assert_eq!(
            fmt(&n("-0.5").factorial().unwrap(), 32),
            "1.7724538509055160272981674833411"
        );
        assert_eq!(f(&n("2").pow(&n("10")).unwrap()), "1024");
        assert_eq!(f(&n("2").pow(&n("-2")).unwrap()), "0.25");
        assert_eq!(f(&n("-2").pow(&n("3")).unwrap()), "-8");
        assert_eq!(f(&n("-8").root(&n("3")).unwrap()), "-2");
        assert_eq!(f(&Num::pi().sin().unwrap()), "0");
        assert_eq!(n("1").div(&n("0")).unwrap_err(), CalcError::DivideByZero);
        assert_eq!(n("0").div(&n("0")).unwrap_err(), CalcError::Undefined);
        assert_eq!(n("-1").sqrt().unwrap_err(), CalcError::InvalidInput);
    }

    #[test]
    fn range() {
        assert_eq!(n("10").pow(&n("10000")).unwrap_err(), CalcError::Overflow);
        assert_eq!(f(&n("10").pow(&n("9999")).unwrap()), "1.e+9999");
        assert_eq!(n("1e9999").mul(&n("10")).unwrap_err(), CalcError::Overflow);
        assert_eq!(f(&n("1e-9999").div(&n("10")).unwrap()), "0");
        assert_eq!(n("3249").factorial().unwrap_err(), CalcError::Overflow);
        assert!(n("3248").factorial().is_ok());
        assert_eq!(n("1e9999").exp().unwrap_err(), CalcError::Overflow);
    }
}
