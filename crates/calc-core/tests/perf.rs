//! ENG-009: every engine operation completes in ≤ 50 ms. Timing is only meaningful in
//! optimized builds: run with `cargo test --release -p calc-core --test perf`.

mod common;

use std::time::{Duration, Instant};

const LIMIT: Duration = Duration::from_millis(50);

/// Worst cases: huge exact values, huge exponents, transcendental functions at the edge
/// of the range and gamma of large non-integers.
const CASES: &[(&str, &str)] = &[
    ("sci", "3248!"),
    ("sci", "3249!"),
    ("sci", "3248.5!"),
    ("sci", "10y9999="),
    ("sci", "10y10000="),
    ("sci", "1.5y1234.5="),
    ("sci", "2y0.5="),
    ("sci", "1x9999s"),
    ("sci", "1x9999{RAD}s"),
    ("sci", "1x9999{RAD}t"),
    ("sci", "1x9999{RAD}o"),
    ("sci", "1x9999n"),
    ("sci", "1x9999@"),
    ("sci", "1x9999{NEG}n"),
    ("sci", "7r*3r*11r*13r*17r*19r*23r*29r*31r*37r*41r*43r="),
    ("sci", "1x9999/3="),
    ("sci", "1x9999/3=*7=*7=*7=*7=*7=*7=*7="),
    ("sci", "0.5{ATANH}"),
    ("sci", "1x9999{COSH}"),
    ("sci", "2x9998{EXPE}"),
    ("sta", "1.1{ADD}2.2{ADD}3.3{ADD}4.4{ADD}5.5{ADD}{SDN1}"),
    (
        "prg",
        "{BIN}1111111111111111111111111111111111111111111111111111111111111111*",
    ),
];

/// Splits a sequence into single keys: one character or one `{NAME}` token.
fn tokens(keys: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = keys.chars();
    while let Some(c) = chars.next() {
        if c == '{' {
            let name: String = chars.by_ref().take_while(|&c| c != '}').collect();
            out.push(format!("{{{name}}}"));
        } else if c != ' ' {
            out.push(c.to_string());
        }
    }
    out
}

#[test]
#[cfg_attr(debug_assertions, ignore = "timing needs --release")]
fn every_key_is_fast() {
    let mut slow = Vec::new();
    for (mode, keys) in CASES {
        let mut c = common::calc(common::mode(mode));
        for (i, key) in tokens(keys).iter().enumerate() {
            let start = Instant::now();
            common::feed(&mut c, key);
            let state = c.state();
            let _ = state.text(true, &calc_format::Locale::EN_US);
            let t = start.elapsed();
            if t > LIMIT {
                slow.push(format!("{mode} | {keys}: key #{i} {key:?} took {t:?}"));
            }
        }
    }
    assert!(
        slow.is_empty(),
        "slower than {LIMIT:?}:\n{}",
        slow.join("\n")
    );
}
