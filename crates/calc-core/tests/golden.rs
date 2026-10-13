//! Golden tests (§10.1): key sequences replayed against the engine, compared with the
//! display strings recorded on the Windows 7 reference.
//!
//! Each `tests/golden/*.txt` line is `mode | keys | display` with an optional
//! `| expression` column; `#` starts a comment. See `tests/common/mod.rs` for the key
//! notation.

mod common;

use std::path::Path;

#[test]
fn golden_sequences() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .expect("tests/golden exists")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "txt"))
        .collect();
    files.sort();
    let mut failures = Vec::new();
    let mut count = 0;
    for file in &files {
        let text = std::fs::read_to_string(file).expect("readable golden file");
        let name = file.file_name().unwrap_or_default().to_string_lossy();
        for (i, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let cols: Vec<&str> = line.split(" | ").map(str::trim).collect();
            assert!(
                cols.len() >= 3,
                "{name}:{}: expected `mode | keys | display`",
                i + 1
            );
            let mut c = common::calc(common::mode(cols[0]));
            common::feed(&mut c, cols[1]);
            count += 1;
            let got = c.display();
            if got != cols[2] {
                failures.push(format!(
                    "{name}:{}: {} | {} → display {got:?}, expected {:?}",
                    i + 1,
                    cols[0],
                    cols[1],
                    cols[2]
                ));
            }
            if let Some(expr) = cols.get(3)
                && c.expression() != *expr
            {
                failures.push(format!(
                    "{name}:{}: {} | {} → expression {:?}, expected {expr:?}",
                    i + 1,
                    cols[0],
                    cols[1],
                    c.expression()
                ));
            }
        }
    }
    assert!(count > 0, "no golden sequences found in {}", dir.display());
    assert!(
        failures.is_empty(),
        "{} of {count} golden sequences failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
