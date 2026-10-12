//! High-precision number used by the calculator engine, plus Win7-style formatting.

use astro_float::{BigFloat, Consts, Radix, RoundingMode, Sign};
use std::cell::RefCell;
use std::cmp::Ordering;
use std::fmt;

/// Working precision in bits (~77 decimal digits), well above the 32 digits Win7 shows.
const P: usize = 256;
const RM: RoundingMode = RoundingMode::ToEven;
/// Win7 reports overflow at 1e10000 (≈ 2^33219).
const MAX_BIN_EXP: i32 = 33219;
/// Results this close to zero are treated as zero (e.g. sin(π)).
const SNAP_BIN_EXP: i32 = -200;
/// Win7 switches to scientific notation when a fraction has more leading zeros than this.
const MAX_ZEROS_AFTER_DECIMAL: i32 = 2;

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
pub struct Num(BigFloat);

impl Num {
    pub fn zero() -> Self {
        Num(BigFloat::from_i32(0, P))
    }

    pub fn from_i64(v: i64) -> Self {
        Num(BigFloat::from_i64(v, P))
    }

    pub fn from_i128(v: i128) -> Self {
        Self::parse(&v.to_string()).expect("integer literal")
    }

    pub fn from_f64(v: f64) -> Self {
        // Go through the shortest decimal representation so 0.1 stays 0.1.
        Self::parse(&format!("{v:e}")).unwrap_or_else(Self::zero)
    }

    /// Parses a decimal number such as `-12.5`, `1.e+5` or `3e-2`.
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim().replace(',', "");
        if s.is_empty() || !s.chars().any(|c| c.is_ascii_digit()) {
            return None;
        }
        let ok = s
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | 'e' | 'E'));
        if !ok {
            return None;
        }
        let s = s.replace(".e", "e").replace(".E", "e");
        let s = s.strip_suffix('.').unwrap_or(&s);
        let n = with_cc(|cc| BigFloat::parse(s, Radix::Dec, P, RM, cc));
        (!n.is_nan()).then_some(Num(n))
    }

    pub fn pi() -> Self {
        Num(with_cc(|cc| cc.pi(P, RM)))
    }

    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    pub fn is_negative(&self) -> bool {
        self.0.is_negative() && !self.0.is_zero()
    }

    pub fn is_int(&self) -> bool {
        self.0.is_zero() || self.0.is_int()
    }

    pub fn neg(&self) -> Self {
        Num(self.0.neg())
    }

    pub fn abs(&self) -> Self {
        Num(self.0.abs())
    }

    /// Integer part (truncates toward zero).
    pub fn int(&self) -> Self {
        Num(self.0.int())
    }

    pub fn frac(&self) -> Self {
        Num(self.0.fract())
    }

    pub fn floor(&self) -> Self {
        Num(self.0.floor())
    }

    fn check(n: BigFloat) -> CalcResult {
        if n.is_nan() {
            return Err(CalcError::InvalidInput);
        }
        if n.is_inf() {
            return Err(CalcError::Overflow);
        }
        match n.exponent() {
            Some(e) if e > MAX_BIN_EXP => Err(CalcError::Overflow),
            Some(e) if e < SNAP_BIN_EXP - 33000 => Ok(Num::zero()),
            _ => Ok(Num(n)),
        }
    }

    /// Snaps tiny results (from transcendental functions) to zero.
    fn snapped(self) -> Self {
        match self.0.exponent() {
            Some(e) if !self.0.is_zero() && e < SNAP_BIN_EXP => Num::zero(),
            _ => self,
        }
    }

    pub fn add(&self, o: &Num) -> CalcResult {
        Self::check(self.0.add(&o.0, P, RM))
    }

    pub fn sub(&self, o: &Num) -> CalcResult {
        Self::check(self.0.sub(&o.0, P, RM))
    }

    pub fn mul(&self, o: &Num) -> CalcResult {
        Self::check(self.0.mul(&o.0, P, RM))
    }

    pub fn div(&self, o: &Num) -> CalcResult {
        if o.is_zero() {
            return Err(if self.is_zero() {
                CalcError::Undefined
            } else {
                CalcError::DivideByZero
            });
        }
        Self::check(self.0.div(&o.0, P, RM))
    }

    /// Truncating remainder (sign follows the dividend).
    pub fn rem(&self, o: &Num) -> CalcResult {
        if o.is_zero() {
            return Err(CalcError::Undefined);
        }
        let q = self.div(o)?.int();
        self.sub(&q.mul(o)?)
    }

    pub fn sqrt(&self) -> CalcResult {
        if self.is_negative() {
            return Err(CalcError::InvalidInput);
        }
        Self::check(self.0.sqrt(P, RM))
    }

    pub fn cbrt(&self) -> CalcResult {
        Self::check(self.0.cbrt(P, RM))
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
            let odd = e.rem(&Num::from_i64(2))?.abs().compare(&Num::from_i64(1)) == Ordering::Equal;
            return Ok(if odd { r.neg() } else { r });
        }
        if e.is_int() && e.abs().compare(&Num::from_i64(1 << 20)) == Ordering::Less {
            let n = e.to_i128().unsigned_abs() as usize;
            let r = Self::check(self.0.powi(n, P, RM))?;
            return if e.is_negative() {
                Num::from_i64(1).div(&r)
            } else {
                Ok(r)
            };
        }
        Self::check(with_cc(|cc| self.0.pow(&e.0, P, RM, cc)))
    }

    /// The `y`-th root of `self` (Win7 `y√x`).
    pub fn root(&self, y: &Num) -> CalcResult {
        if y.is_zero() {
            return Err(CalcError::InvalidInput);
        }
        if self.is_negative() {
            // Only odd integer roots of negative numbers are real.
            let odd = y.is_int()
                && y.rem(&Num::from_i64(2))?.abs().compare(&Num::from_i64(1)) == Ordering::Equal;
            if !odd {
                return Err(CalcError::InvalidInput);
            }
            return Ok(self.abs().root(y)?.neg());
        }
        self.pow(&Num::from_i64(1).div(y)?)
    }

    pub fn ln(&self) -> CalcResult {
        if self.is_negative() || self.is_zero() {
            return Err(CalcError::InvalidInput);
        }
        Self::check(with_cc(|cc| self.0.ln(P, RM, cc))).map(Self::snapped)
    }

    pub fn log10(&self) -> CalcResult {
        if self.is_negative() || self.is_zero() {
            return Err(CalcError::InvalidInput);
        }
        Self::check(with_cc(|cc| self.0.log10(P, RM, cc))).map(Self::snapped)
    }

    pub fn exp(&self) -> CalcResult {
        Self::check(with_cc(|cc| self.0.exp(P, RM, cc)))
    }

    pub fn sin(&self) -> CalcResult {
        Self::check(with_cc(|cc| self.0.sin(P, RM, cc))).map(Self::snapped)
    }

    pub fn cos(&self) -> CalcResult {
        Self::check(with_cc(|cc| self.0.cos(P, RM, cc))).map(Self::snapped)
    }

    pub fn tan(&self) -> CalcResult {
        Self::check(with_cc(|cc| self.0.tan(P, RM, cc))).map(Self::snapped)
    }

    pub fn asin(&self) -> CalcResult {
        if self.abs().compare(&Num::from_i64(1)) == Ordering::Greater {
            return Err(CalcError::InvalidInput);
        }
        Self::check(with_cc(|cc| self.0.asin(P, RM, cc))).map(Self::snapped)
    }

    pub fn acos(&self) -> CalcResult {
        if self.abs().compare(&Num::from_i64(1)) == Ordering::Greater {
            return Err(CalcError::InvalidInput);
        }
        Self::check(with_cc(|cc| self.0.acos(P, RM, cc))).map(Self::snapped)
    }

    pub fn atan(&self) -> CalcResult {
        Self::check(with_cc(|cc| self.0.atan(P, RM, cc))).map(Self::snapped)
    }

    pub fn sinh(&self) -> CalcResult {
        Self::check(with_cc(|cc| self.0.sinh(P, RM, cc))).map(Self::snapped)
    }

    pub fn cosh(&self) -> CalcResult {
        Self::check(with_cc(|cc| self.0.cosh(P, RM, cc)))
    }

    pub fn tanh(&self) -> CalcResult {
        Self::check(with_cc(|cc| self.0.tanh(P, RM, cc))).map(Self::snapped)
    }

    pub fn asinh(&self) -> CalcResult {
        Self::check(with_cc(|cc| self.0.asinh(P, RM, cc))).map(Self::snapped)
    }

    pub fn acosh(&self) -> CalcResult {
        if self.compare(&Num::from_i64(1)) == Ordering::Less {
            return Err(CalcError::InvalidInput);
        }
        Self::check(with_cc(|cc| self.0.acosh(P, RM, cc))).map(Self::snapped)
    }

    pub fn atanh(&self) -> CalcResult {
        if self.abs().compare(&Num::from_i64(1)) != Ordering::Less {
            return Err(CalcError::InvalidInput);
        }
        Self::check(with_cc(|cc| self.0.atanh(P, RM, cc))).map(Self::snapped)
    }

    pub fn factorial(&self) -> CalcResult {
        if self.is_int() {
            if self.is_negative() {
                return Err(CalcError::InvalidInput);
            }
            if self.compare(&Num::from_i64(3248)) == Ordering::Greater {
                return Err(CalcError::Overflow);
            }
            let n = self.to_i128();
            let mut r = Num::from_i64(1);
            for i in 2..=n {
                r = r.mul(&Num::from_i128(i))?;
            }
            return Ok(r);
        }
        // Non-integers use the gamma function, like Win7 (0.5! = 0.886…).
        gamma(&self.add(&Num::from_i64(1))?)
    }

    pub fn compare(&self, o: &Num) -> Ordering {
        match self.0.cmp(&o.0) {
            Some(c) if c < 0 => Ordering::Less,
            Some(c) if c > 0 => Ordering::Greater,
            _ => Ordering::Equal,
        }
    }

    /// Decimal digits and exponent: value = 0.d1d2d3… × 10^exp. Zero yields `([], 0)`.
    fn decimal(&self) -> (bool, Vec<u8>, i32) {
        if self.is_zero() {
            return (false, vec![], 0);
        }
        let (sign, mut digits, exp) = with_cc(|cc| self.0.convert_to_radix(Radix::Dec, RM, cc))
            .expect("finite number converts to decimal");
        while digits.last() == Some(&0) {
            digits.pop();
        }
        let lead = digits.iter().take_while(|&&d| d == 0).count();
        digits.drain(..lead);
        (sign == Sign::Neg, digits, exp - lead as i32)
    }

    /// Rounds decimal digits half-up to `sig` significant digits.
    fn round_digits(mut digits: Vec<u8>, mut exp: i32, sig: usize) -> (Vec<u8>, i32) {
        if digits.len() > sig {
            let up = digits[sig] >= 5;
            digits.truncate(sig);
            if up {
                let mut i = sig;
                loop {
                    if i == 0 {
                        digits.insert(0, 1);
                        exp += 1;
                        digits.truncate(sig.max(1));
                        break;
                    }
                    i -= 1;
                    if digits[i] == 9 {
                        digits[i] = 0;
                    } else {
                        digits[i] += 1;
                        break;
                    }
                }
            }
        }
        while digits.last() == Some(&0) {
            digits.pop();
        }
        (digits, exp)
    }

    pub fn to_f64(&self) -> f64 {
        self.to_plain_string().parse().unwrap_or(f64::NAN)
    }

    /// Integer value, truncated toward zero. Saturates outside the i128 range.
    pub fn to_i128(&self) -> i128 {
        let (neg, digits, exp) = self.int().decimal();
        if exp > 38 {
            return if neg { i128::MIN } else { i128::MAX };
        }
        let mut v: i128 = 0;
        for i in 0..exp.max(0) as usize {
            v = v * 10 + *digits.get(i).unwrap_or(&0) as i128;
        }
        if neg { -v } else { v }
    }

    /// Full-precision representation without grouping, used for parsing round-trips.
    pub fn to_plain_string(&self) -> String {
        let (neg, digits, exp) = self.decimal();
        if digits.is_empty() {
            return "0".into();
        }
        let m: String = digits.iter().map(|d| (b'0' + d) as char).collect();
        format!("{}0.{}e{}", if neg { "-" } else { "" }, m, exp)
    }

    /// Formats like Win7 calc: at most `max_digits` significant digits, switching to
    /// scientific notation (`1.e+16`) when the number doesn't fit.
    pub fn format(&self, max_digits: usize, force_sci: bool, grouping: bool) -> String {
        let (neg, digits, exp) = self.decimal();
        if digits.is_empty() {
            return if force_sci {
                "0.e+0".into()
            } else {
                "0".into()
            };
        }
        let sign = if neg { "-" } else { "" };
        let (r, e) = Self::round_digits(digits.clone(), exp, max_digits);
        let max = max_digits as i32;
        let body = if force_sci || e > max || e < -MAX_ZEROS_AFTER_DECIMAL {
            sci(&r, e)
        } else if e >= 1 {
            fixed(&r, e, grouping)
        } else {
            let sig = (max + e).max(1) as usize;
            let (r, e) = Self::round_digits(digits, exp, sig);
            fixed(&r, e, grouping)
        };
        format!("{sign}{body}")
    }
}

fn digit_str(d: &[u8]) -> String {
    d.iter().map(|d| (b'0' + d) as char).collect()
}

fn sci(d: &[u8], e: i32) -> String {
    let rest = digit_str(&d[1..]);
    let x = e - 1;
    format!(
        "{}.{}e{}{}",
        d[0],
        rest,
        if x < 0 { '-' } else { '+' },
        x.abs()
    )
}

fn fixed(d: &[u8], e: i32, grouping: bool) -> String {
    let (int, frac) = if e <= 0 {
        (
            "0".to_string(),
            format!("{}{}", "0".repeat((-e) as usize), digit_str(d)),
        )
    } else {
        let e = e as usize;
        let mut int = digit_str(&d[..e.min(d.len())]);
        int.push_str(&"0".repeat(e.saturating_sub(d.len())));
        (int, digit_str(d.get(e..).unwrap_or(&[])))
    };
    let int = if grouping { group(&int, 3, ',') } else { int };
    if frac.is_empty() {
        int
    } else {
        format!("{int}.{frac}")
    }
}

/// Inserts `sep` every `n` characters counting from the right.
pub fn group(s: &str, n: usize, sep: char) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::new();
    for (i, c) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i).is_multiple_of(n) {
            out.push(sep);
        }
        out.push(*c);
    }
    out
}

/// Bernoulli numbers B2..B30 for the Stirling series.
const BERNOULLI: [(&str, &str); 15] = [
    ("1", "6"),
    ("-1", "30"),
    ("1", "42"),
    ("-1", "30"),
    ("5", "66"),
    ("-691", "2730"),
    ("7", "6"),
    ("-3617", "510"),
    ("43867", "798"),
    ("-174611", "330"),
    ("854513", "138"),
    ("-236364091", "2730"),
    ("8553103", "6"),
    ("-23749461029", "870"),
    ("8615841276005", "14322"),
];

/// Gamma function for non-integers, accurate well beyond the 32 displayed digits.
fn gamma(z: &Num) -> CalcResult {
    let one = Num::from_i64(1);
    if z.compare(&Num::from_f64(0.5)) == Ordering::Less {
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
    let half = Num::from_f64(0.5);
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
        let b = Num::parse(num).unwrap().div(&Num::parse(den).unwrap())?;
        lg = lg.add(&b.div(&Num::from_i64(k2 * (k2 - 1)).mul(&zp)?)?)?;
        zp = zp.mul(&z2)?;
    }
    lg.exp()?.div(&shift)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(s: &str) -> Num {
        Num::parse(s).unwrap()
    }

    fn f(x: &Num) -> String {
        x.format(16, false, false)
    }

    #[test]
    fn formatting() {
        assert_eq!(f(&n("0")), "0");
        assert_eq!(f(&n("123")), "123");
        assert_eq!(f(&n("-12.5")), "-12.5");
        assert_eq!(f(&n("1").div(&n("3")).unwrap()), "0.3333333333333333");
        assert_eq!(f(&n("2").div(&n("3")).unwrap()), "0.6666666666666667");
        assert_eq!(f(&n("1").div(&n("700")).unwrap()), "0.0014285714285714");
        assert_eq!(f(&n("1").div(&n("7000")).unwrap()), "1.428571428571429e-4");
        assert_eq!(f(&n("1e16")), "1.e+16");
        assert_eq!(f(&n("9999999999999999")), "9999999999999999");
        assert_eq!(f(&n("0.1").add(&n("0.2")).unwrap()), "0.3");
        assert_eq!(n("1234567").format(16, false, true), "1,234,567");
        assert_eq!(n("1234.5").format(16, true, false), "1.2345e+3");
        assert_eq!(n("99999999999999999").format(16, false, false), "1.e+17");
    }

    #[test]
    fn conversions() {
        assert_eq!(n("-42.9").to_i128(), -42);
        assert_eq!(n("12345678901234567890").to_i128(), 12345678901234567890);
        assert_eq!(n("0.25").to_f64(), 0.25);
        assert_eq!(f(&Num::from_f64(0.1)), "0.1");
    }

    #[test]
    fn functions() {
        assert_eq!(f(&n("5").factorial().unwrap()), "120");
        assert_eq!(
            n("0.5").factorial().unwrap().format(32, false, false),
            "0.88622692545275801364908374167057"
        );
        assert_eq!(
            n("-0.5").factorial().unwrap().format(32, false, false),
            "1.7724538509055160272981674833411"
        );
        assert_eq!(f(&n("2").pow(&n("10")).unwrap()), "1024");
        assert_eq!(f(&n("-2").pow(&n("3")).unwrap()), "-8");
        assert_eq!(f(&n("-8").root(&n("3")).unwrap()), "-2");
        assert_eq!(f(&Num::pi().sin().unwrap()), "0");
        assert_eq!(n("1").div(&n("0")).unwrap_err(), CalcError::DivideByZero);
        assert_eq!(n("0").div(&n("0")).unwrap_err(), CalcError::Undefined);
        assert_eq!(n("-1").sqrt().unwrap_err(), CalcError::InvalidInput);
        assert_eq!(n("10").pow(&n("10000")).unwrap_err(), CalcError::Overflow);
    }
}
