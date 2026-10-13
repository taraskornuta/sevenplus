//! Display formatting for the calculator (ENG-010): pure functions of
//! `(value, settings, locale)` with no engine or UI dependency.
//!
//! The engine hands over numbers as [`Decimal`] digit strings or programmer-mode
//! [`Value::Int`] bits; this crate decides rounding (half-to-even, ENG-001), notation
//! (fixed or `d.ddd…e+n`), digit grouping and the locale's separators.

#![forbid(unsafe_code)]

/// Decimal digits of a number: `value = 0.d1 d2 d3 … × 10^exp`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Decimal {
    pub neg: bool,
    /// Significant digits (0–9) without leading or trailing zeros; empty for zero.
    pub digits: Vec<u8>,
    pub exp: i32,
    /// True when non-zero digits follow the ones in `digits` (the value was truncated).
    pub inexact: bool,
}

impl Decimal {
    pub fn is_zero(&self) -> bool {
        self.digits.is_empty()
    }
}

/// A number ready for display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Dec(Decimal),
    /// Programmer mode: two's-complement bits of a `word`-bit integer.
    Int {
        bits: u64,
        word: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Notation {
    /// Fixed point, switching to exponential when the number does not fit.
    #[default]
    Auto,
    /// Always exponential (the `F-E` key).
    Exponential,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormatSettings {
    /// Significant digits shown: 16 in Standard/Statistics, 32 in Scientific.
    pub max_digits: usize,
    pub notation: Notation,
    pub grouping: bool,
    /// Programmer-mode radix: 16, 10, 8 or 2.
    pub radix: u32,
}

impl Default for FormatSettings {
    fn default() -> Self {
        FormatSettings {
            max_digits: 16,
            notation: Notation::Auto,
            grouping: false,
            radix: 10,
        }
    }
}

/// Decimal and digit-group separators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Locale {
    pub decimal: char,
    pub group: char,
}

impl Default for Locale {
    fn default() -> Self {
        Locale::EN_US
    }
}

impl Locale {
    pub const EN_US: Locale = Locale {
        decimal: '.',
        group: ',',
    };

    /// Separators for a BCP 47 tag such as `de-DE` or `fr_CA.UTF-8` (CLDR defaults).
    pub fn from_tag(tag: &str) -> Locale {
        let tag = tag.split(['.', '@']).next().unwrap_or("").replace('_', "-");
        let mut parts = tag.split('-');
        let lang = parts.next().unwrap_or("").to_ascii_lowercase();
        let region = parts
            .find(|p| p.len() == 2)
            .map(|p| p.to_ascii_uppercase())
            .unwrap_or_default();
        const NBSP: char = '\u{a0}';
        let (decimal, group) = match (lang.as_str(), region.as_str()) {
            ("de" | "it" | "rm", "CH" | "LI") => ('.', '\''),
            ("es", "MX" | "US" | "PR") | ("pt", "") => ('.', ','),
            ("pt", "BR") => (',', '.'),
            ("pt", _) => (',', NBSP),
            ("fr", "CH") => (',', '\u{202f}'),
            (
                "de" | "es" | "it" | "nl" | "id" | "tr" | "da" | "el" | "ro" | "hr" | "sl" | "sr"
                | "vi" | "ca" | "is",
                _,
            ) => (',', '.'),
            (
                "fr" | "ru" | "uk" | "be" | "pl" | "cs" | "sk" | "sv" | "fi" | "nb" | "nn" | "no"
                | "hu" | "bg" | "lt" | "lv" | "et" | "kk" | "ka" | "hy" | "az",
                _,
            ) => (',', NBSP),
            _ => ('.', ','),
        };
        Locale { decimal, group }
    }
}

/// The number being typed, kept as text so trailing zeros and the decimal point survive.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EntryText {
    pub neg: bool,
    /// Digits in the current radix, with an optional `.`.
    pub mant: String,
    /// Exponent typed after `Exp`: (negative, digits).
    pub exp: Option<(bool, String)>,
}

/// Formats a value for the main display.
pub fn format(v: &Value, s: &FormatSettings, l: &Locale) -> String {
    match v {
        Value::Dec(d) => format_decimal(d, s, l),
        Value::Int { bits, word } => format_int(*bits, *word, s, l),
    }
}

/// Formats a number being typed.
pub fn format_entry(e: &EntryText, s: &FormatSettings, l: &Locale) -> String {
    let mut out = String::new();
    if e.neg {
        out.push('-');
    }
    let (int, frac) = match e.mant.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (e.mant.as_str(), None),
    };
    if s.grouping {
        let (n, sep) = group_style(s.radix, l);
        out.push_str(&group(int, n, sep));
    } else {
        out.push_str(int);
    }
    if let Some(f) = frac {
        out.push(l.decimal);
        out.push_str(f);
    }
    if let Some((neg, digits)) = &e.exp {
        if frac.is_none() {
            out.push(l.decimal);
        }
        out.push_str(if *neg { "e-" } else { "e+" });
        out.push_str(if digits.is_empty() { "0" } else { digits });
    }
    out
}

/// Rewrites canonical text (`.` decimal point, as in the expression line) for a locale.
pub fn localize(text: &str, l: &Locale) -> String {
    if l.decimal == '.' {
        return text.to_string();
    }
    text.replace('.', &l.decimal.to_string())
}

/// Turns locale-formatted text (e.g. pasted `1.234,5`) into canonical `1234.5`.
pub fn delocalize(text: &str, l: &Locale) -> String {
    text.chars()
        .filter(|&c| c != l.group && !(c.is_whitespace() && l.group.is_whitespace()))
        .map(|c| if c == l.decimal { '.' } else { c })
        .collect()
}

/// Digit-group size and separator for a radix.
fn group_style(radix: u32, l: &Locale) -> (usize, char) {
    match radix {
        16 | 2 => (4, ' '),
        8 => (3, ' '),
        _ => (3, l.group),
    }
}

/// Rounds to `sig` significant digits, half to even (ENG-001). Returns digits without
/// trailing zeros and the possibly incremented exponent.
pub fn round_half_even(d: &Decimal, sig: usize) -> (Vec<u8>, i32) {
    let mut digits = d.digits.clone();
    let mut exp = d.exp;
    if digits.len() > sig {
        let next = digits.get(sig).copied().unwrap_or(0);
        let rest_nonzero = d.inexact || digits[sig + 1..].iter().any(|&x| x != 0);
        let up = match next {
            6..=9 => true,
            5 if rest_nonzero => true,
            5 => sig > 0 && digits[sig - 1] % 2 == 1,
            _ => false,
        };
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

/// Win7 switches to scientific notation when a fraction has more leading zeros than this.
const MAX_ZEROS_AFTER_DECIMAL: i32 = 2;

fn format_decimal(d: &Decimal, s: &FormatSettings, l: &Locale) -> String {
    let force_sci = s.notation == Notation::Exponential;
    let (digits, exp) = round_half_even(d, s.max_digits);
    if digits.is_empty() {
        let zero = if force_sci { "0.e+0" } else { "0" };
        return localize(zero, l);
    }
    let sign = if d.neg { "-" } else { "" };
    let max = s.max_digits as i32;
    let body = if force_sci || exp > max || exp < -MAX_ZEROS_AFTER_DECIMAL {
        sci(&digits, exp, l)
    } else if exp >= 1 {
        fixed(&digits, exp, s.grouping, l)
    } else {
        // Leading zeros of a fraction count towards the digit limit, like Win7.
        let (r, e) = round_half_even(d, (max + exp).max(1) as usize);
        if r.is_empty() {
            return localize("0", l);
        }
        fixed(&r, e, s.grouping, l)
    };
    format!("{sign}{body}")
}

fn digit_str(d: &[u8]) -> String {
    d.iter().map(|d| (b'0' + d) as char).collect()
}

fn sci(d: &[u8], e: i32, l: &Locale) -> String {
    let x = e - 1;
    format!(
        "{}{}{}e{}{}",
        d[0],
        l.decimal,
        digit_str(&d[1..]),
        if x < 0 { '-' } else { '+' },
        x.abs()
    )
}

fn fixed(d: &[u8], e: i32, grouping: bool, l: &Locale) -> String {
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
    let int = if grouping {
        group(&int, 3, l.group)
    } else {
        int
    };
    if frac.is_empty() {
        int
    } else {
        format!("{int}{}{frac}", l.decimal)
    }
}

fn format_int(bits: u64, word: u32, s: &FormatSettings, l: &Locale) -> String {
    let word = word.clamp(1, 64);
    let mask = if word == 64 {
        u64::MAX
    } else {
        (1u64 << word) - 1
    };
    let u = bits & mask;
    let text = match s.radix {
        16 => format!("{u:X}"),
        8 => format!("{u:o}"),
        2 => format!("{u:b}"),
        _ => {
            // Sign-extend from the word size.
            let shift = 64 - word;
            (((u << shift) as i64) >> shift).to_string()
        }
    };
    if !s.grouping {
        return text;
    }
    let (n, sep) = group_style(s.radix, l);
    match text.strip_prefix('-') {
        Some(d) => format!("-{}", group(d, n, sep)),
        None => group(&text, n, sep),
    }
}

/// Inserts `sep` every `n` characters counting from the right.
pub fn group(s: &str, n: usize, sep: char) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len() + s.len() / n.max(1));
    for (i, c) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i).is_multiple_of(n.max(1)) {
            out.push(sep);
        }
        out.push(*c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a [`Decimal`] from text like `-12.5` or `1e16` (test helper only).
    fn dec(s: &str) -> Decimal {
        let (neg, s) = match s.strip_prefix('-') {
            Some(r) => (true, r),
            None => (false, s),
        };
        let (mant, e10) = match s.split_once('e') {
            Some((m, e)) => (m, e.parse::<i32>().unwrap()),
            None => (s, 0),
        };
        let (int, frac) = mant.split_once('.').unwrap_or((mant, ""));
        let all: Vec<u8> = int.bytes().chain(frac.bytes()).map(|b| b - b'0').collect();
        let lead = all.iter().take_while(|&&d| d == 0).count();
        let mut digits = all[lead..].to_vec();
        while digits.last() == Some(&0) {
            digits.pop();
        }
        let exp = if digits.is_empty() {
            0
        } else {
            int.len() as i32 - lead as i32 + e10
        };
        Decimal {
            neg,
            digits,
            exp,
            inexact: false,
        }
    }

    fn f(s: &str) -> String {
        format(
            &Value::Dec(dec(s)),
            &FormatSettings::default(),
            &Locale::EN_US,
        )
    }

    fn fs(s: &str, set: FormatSettings) -> String {
        format(&Value::Dec(dec(s)), &set, &Locale::EN_US)
    }

    #[test]
    fn fixed_and_scientific() {
        assert_eq!(f("0"), "0");
        assert_eq!(f("123"), "123");
        assert_eq!(f("-12.5"), "-12.5");
        assert_eq!(f("0.33333333333333333333"), "0.3333333333333333");
        assert_eq!(f("0.66666666666666666666"), "0.6666666666666667");
        assert_eq!(f("0.00142857142857142857"), "0.0014285714285714");
        assert_eq!(f("0.000142857142857142857"), "1.428571428571429e-4");
        assert_eq!(f("1e16"), "1.e+16");
        assert_eq!(f("9999999999999999"), "9999999999999999");
        assert_eq!(f("99999999999999999"), "1.e+17");
        let sci = FormatSettings {
            notation: Notation::Exponential,
            ..Default::default()
        };
        assert_eq!(fs("1234.5", sci), "1.2345e+3");
        assert_eq!(fs("0", sci), "0.e+0");
    }

    #[test]
    fn half_to_even() {
        // Exact ties round to the even neighbour (ENG-001)…
        assert_eq!(f("0.12345678901234565"), "0.1234567890123456");
        assert_eq!(f("0.12345678901234575"), "0.1234567890123458");
        assert_eq!(f("12345678901234565"), "1.234567890123456e+16");
        // …but anything past the tie rounds up.
        let mut d = dec("0.12345678901234565");
        d.inexact = true;
        assert_eq!(
            format(&Value::Dec(d), &FormatSettings::default(), &Locale::EN_US),
            "0.1234567890123457"
        );
        assert_eq!(f("0.12345678901234565000001"), "0.1234567890123457");
    }

    #[test]
    fn grouping_and_locale() {
        let g = FormatSettings {
            grouping: true,
            ..Default::default()
        };
        assert_eq!(fs("1234567", g), "1,234,567");
        let de = Locale::from_tag("de_DE.UTF-8");
        assert_eq!(
            de,
            Locale {
                decimal: ',',
                group: '.'
            }
        );
        assert_eq!(
            format(&Value::Dec(dec("1234567.25")), &g, &de),
            "1.234.567,25"
        );
        assert_eq!(Locale::from_tag("fr-FR").group, '\u{a0}');
        assert_eq!(Locale::from_tag("en-US"), Locale::EN_US);
        assert_eq!(Locale::from_tag("C"), Locale::EN_US);
        assert_eq!(delocalize("1.234,5", &de), "1234.5");
        assert_eq!(localize("5 + sqrt(2.5)", &de), "5 + sqrt(2,5)");
    }

    #[test]
    fn programmer_ints() {
        let s = |radix, grouping| FormatSettings {
            radix,
            grouping,
            ..Default::default()
        };
        let l = Locale::EN_US;
        let v = Value::Int {
            bits: 0x80,
            word: 8,
        };
        assert_eq!(format(&v, &s(10, false), &l), "-128");
        assert_eq!(format(&v, &s(16, false), &l), "80");
        let v = Value::Int {
            bits: u64::MAX,
            word: 64,
        };
        assert_eq!(format(&v, &s(10, false), &l), "-1");
        assert_eq!(format(&v, &s(16, true), &l), "FFFF FFFF FFFF FFFF");
        let v = Value::Int {
            bits: 1234567,
            word: 32,
        };
        assert_eq!(format(&v, &s(10, true), &l), "1,234,567");
    }

    #[test]
    fn entries() {
        let e = EntryText {
            neg: true,
            mant: "1234567.50".into(),
            exp: None,
        };
        let g = FormatSettings {
            grouping: true,
            ..Default::default()
        };
        assert_eq!(format_entry(&e, &g, &Locale::EN_US), "-1,234,567.50");
        let e = EntryText {
            neg: false,
            mant: "12".into(),
            exp: Some((true, String::new())),
        };
        assert_eq!(
            format_entry(&e, &FormatSettings::default(), &Locale::EN_US),
            "12.e-0"
        );
    }
}
