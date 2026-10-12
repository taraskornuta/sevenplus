//! Win7 "Worksheets": mortgage, vehicle lease and fuel economy.
//!
//! Each worksheet has a natural output (its last field) computed from the others; solving
//! for any other field is done numerically, since every output is monotonic in each input.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Worksheet {
    Mortgage,
    VehicleLease,
    FuelMpg,
    FuelL100km,
}

pub const ALL: [Worksheet; 4] = [
    Worksheet::Mortgage,
    Worksheet::VehicleLease,
    Worksheet::FuelMpg,
    Worksheet::FuelL100km,
];

impl Worksheet {
    pub fn name(self) -> &'static str {
        match self {
            Worksheet::Mortgage => "Mortgage",
            Worksheet::VehicleLease => "Vehicle lease",
            Worksheet::FuelMpg => "Fuel economy (mpg)",
            Worksheet::FuelL100km => "Fuel economy (L/100 km)",
        }
    }

    pub fn fields(self) -> &'static [&'static str] {
        match self {
            Worksheet::Mortgage => &[
                "Purchase price",
                "Down payment",
                "Term (years)",
                "Interest rate (%)",
                "Monthly payment",
            ],
            Worksheet::VehicleLease => &[
                "Lease value",
                "Lease period (years)",
                "Interest rate (%)",
                "Residual value",
                "Monthly payment",
            ],
            Worksheet::FuelMpg => &[
                "Distance (miles)",
                "Fuel used (gallons)",
                "Fuel economy (mpg)",
            ],
            Worksheet::FuelL100km => &[
                "Distance (kilometers)",
                "Fuel used (liters)",
                "Fuel economy (L/100 km)",
            ],
        }
    }

    /// Computes the last field from the others.
    fn output(self, v: &[f64]) -> f64 {
        match self {
            Worksheet::Mortgage => annuity(v[0] - v[1], 0.0, v[2], v[3]),
            Worksheet::VehicleLease => annuity(v[0], v[3], v[1], v[2]),
            Worksheet::FuelMpg => v[0] / v[1],
            Worksheet::FuelL100km => v[1] / v[0] * 100.0,
        }
    }

    /// Solves for field `target` given all other fields in `values`.
    pub fn solve(self, target: usize, values: &[f64]) -> Option<f64> {
        let last = self.fields().len() - 1;
        let others_ok = values
            .iter()
            .enumerate()
            .all(|(i, v)| i == target || (v.is_finite() && *v >= 0.0));
        if !others_ok {
            return None;
        }
        if target == last {
            let r = self.output(values);
            return r.is_finite().then_some(r);
        }
        let goal = values[last];
        let mut v = values.to_vec();
        let mut f = |x: f64| {
            v[target] = x;
            self.output(&v) - goal
        };
        // Bracket the root on [0, hi], then bisect.
        let (mut lo, mut hi) = (0.0, 1.0);
        let f_lo = f(lo);
        while hi < 1e15 && (f(hi).signum() == f_lo.signum() || !f(hi).is_finite()) {
            lo = hi;
            hi *= 2.0;
        }
        if hi >= 1e15 {
            return None;
        }
        let s_lo = f(lo).signum();
        for _ in 0..200 {
            let mid = (lo + hi) / 2.0;
            let fm = f(mid);
            if !fm.is_finite() || fm.signum() == s_lo {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        Some((lo + hi) / 2.0)
    }
}

/// Monthly payment for `principal` over `years` at `rate_pct` annual interest,
/// leaving `residual` at the end.
fn annuity(principal: f64, residual: f64, years: f64, rate_pct: f64) -> f64 {
    let n = years * 12.0;
    let r = rate_pct / 100.0 / 12.0;
    if r == 0.0 {
        return (principal - residual) / n;
    }
    let k = (1.0 + r).powf(-n);
    (principal - residual * k) * r / (1.0 - k)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 0.01
    }

    #[test]
    fn mortgage() {
        let w = Worksheet::Mortgage;
        let pay = w.solve(4, &[200000.0, 40000.0, 30.0, 6.0, 0.0]).unwrap();
        assert!(close(pay, 959.28), "{pay}");
        let rate = w.solve(3, &[200000.0, 40000.0, 30.0, 0.0, pay]).unwrap();
        assert!(close(rate, 6.0), "{rate}");
        let price = w.solve(0, &[0.0, 40000.0, 30.0, 6.0, pay]).unwrap();
        assert!(close(price, 200000.0), "{price}");
        let term = w.solve(2, &[200000.0, 40000.0, 0.0, 6.0, pay]).unwrap();
        assert!(close(term, 30.0), "{term}");
    }

    #[test]
    fn fuel() {
        assert!(close(
            Worksheet::FuelMpg.solve(2, &[300.0, 10.0, 0.0]).unwrap(),
            30.0
        ));
        assert!(close(
            Worksheet::FuelMpg.solve(1, &[300.0, 0.0, 30.0]).unwrap(),
            10.0
        ));
        assert!(close(
            Worksheet::FuelL100km.solve(2, &[500.0, 40.0, 0.0]).unwrap(),
            8.0
        ));
    }

    #[test]
    fn lease() {
        let w = Worksheet::VehicleLease;
        let pay = w.solve(4, &[30000.0, 3.0, 5.0, 15000.0, 0.0]).unwrap();
        let residual = w.solve(3, &[30000.0, 3.0, 5.0, 0.0, pay]).unwrap();
        assert!(close(residual, 15000.0), "{residual}");
    }
}
