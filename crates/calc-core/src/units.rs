//! Unit conversion tables matching the Win7 "Unit conversion" panel (EXT-001).
//!
//! Factors are compile-time constants, exact by definition where the unit is defined in
//! SI terms, otherwise given to at least 15 significant digits and checked against
//! NIST SP 811 Appendix B (EXT-002).

pub struct Category {
    pub name: &'static str,
    /// (unit name, factor to the category's base unit)
    pub units: &'static [(&'static str, f64)],
}

const IN: f64 = 0.0254;
const FT: f64 = 0.3048;
const YD: f64 = 0.9144;
const MI: f64 = 1609.344;
const GAL_US: f64 = 3.785411784e-3;
const GAL_UK: f64 = 4.54609e-3;
const LB: f64 = 0.45359237;
/// Pound-force in newtons (exact: lb × standard gravity 9.80665 m/s²).
const LBF: f64 = 4.4482216152605;

pub const CATEGORIES: &[Category] = &[
    Category {
        name: "Angle",
        units: &[
            ("Degree", 1.0),
            ("Gradian", 0.9),
            ("Radian", 180.0 / std::f64::consts::PI),
        ],
    },
    Category {
        name: "Area",
        units: &[
            ("Acres", 4046.8564224),
            ("Hectares", 10000.0),
            ("Square centimeter", 1e-4),
            ("Square feet", FT * FT),
            ("Square inch", IN * IN),
            ("Square kilometer", 1e6),
            ("Square meter", 1.0),
            ("Square mile", MI * MI),
            ("Square millimeter", 1e-6),
            ("Square yard", YD * YD),
        ],
    },
    Category {
        name: "Energy",
        units: &[
            ("British thermal unit", 1055.05585262),
            ("Calorie", 4.1868),
            ("Electron-volts", 1.602176634e-19),
            ("Foot-pound", FT * LBF),
            ("Joule", 1.0),
            ("Kilocalorie", 4186.8),
            ("Kilojoule", 1000.0),
        ],
    },
    Category {
        name: "Length",
        units: &[
            ("Angstroms", 1e-10),
            ("Centimeters", 0.01),
            ("Chains", 20.1168),
            ("Feet", FT),
            ("Hands", 0.1016),
            ("Inches", IN),
            ("Kilometers", 1000.0),
            ("Links", 0.201168),
            ("Meters", 1.0),
            ("Microns", 1e-6),
            ("Miles", MI),
            ("Millimeters", 0.001),
            ("Nanometers", 1e-9),
            ("Nautical miles", 1852.0),
            ("PICAs", IN / 6.0),
            ("Rods", 5.0292),
            ("Spans", 0.2286),
            ("Yards", YD),
        ],
    },
    Category {
        name: "Power",
        units: &[
            ("BTU/minute", 1055.05585262 / 60.0),
            ("Foot-pound/minute", FT * LBF / 60.0),
            ("Horsepower", 550.0 * FT * LBF),
            ("Kilowatt", 1000.0),
            ("Watt", 1.0),
        ],
    },
    Category {
        name: "Pressure",
        units: &[
            ("Atmosphere", 101325.0),
            ("Bar", 100000.0),
            ("Kilopascal", 1000.0),
            ("Millimeter of mercury", 133.322387415),
            ("Pascal", 1.0),
            ("Pound per square inch (PSI)", LBF / (IN * IN)),
        ],
    },
    Category {
        // Factors unused: temperature is affine, see `convert`.
        name: "Temperature",
        units: &[
            ("Degrees Celsius", 0.0),
            ("Degrees Fahrenheit", 0.0),
            ("Kelvin", 0.0),
        ],
    },
    Category {
        name: "Time",
        units: &[
            ("Day", 86400.0),
            ("Hour", 3600.0),
            ("Microsecond", 1e-6),
            ("Millisecond", 1e-3),
            ("Minute", 60.0),
            ("Second", 1.0),
            ("Week", 604800.0),
        ],
    },
    Category {
        name: "Velocity",
        units: &[
            ("Centimeters per second", 0.01),
            ("Feet per second", FT),
            ("Kilometers per hour", 1000.0 / 3600.0),
            ("Knots", 1852.0 / 3600.0),
            ("Mach", 340.3),
            ("Meters per second", 1.0),
            ("Miles per hour", MI / 3600.0),
        ],
    },
    Category {
        name: "Volume",
        units: &[
            ("Cubic centimeter", 1e-6),
            ("Cubic feet", FT * FT * FT),
            ("Cubic inch", IN * IN * IN),
            ("Cubic meter", 1.0),
            ("Cubic yard", YD * YD * YD),
            ("Fluid ounce (UK)", GAL_UK / 160.0),
            ("Fluid ounce (US)", GAL_US / 128.0),
            ("Gallon (UK)", GAL_UK),
            ("Gallon (US)", GAL_US),
            ("Liter", 1e-3),
            ("Pint (UK)", GAL_UK / 8.0),
            ("Pint (US)", GAL_US / 8.0),
            ("Quart (UK)", GAL_UK / 4.0),
            ("Quart (US)", GAL_US / 4.0),
        ],
    },
    Category {
        name: "Weight/Mass",
        units: &[
            ("Carat", 2e-4),
            ("Centigram", 1e-5),
            ("Decigram", 1e-4),
            ("Dekagram", 1e-2),
            ("Gram", 1e-3),
            ("Hectogram", 0.1),
            ("Kilogram", 1.0),
            ("Long ton", 2240.0 * LB),
            ("Milligram", 1e-6),
            ("Ounce", LB / 16.0),
            ("Pound", LB),
            ("Short ton", 2000.0 * LB),
            ("Stone", 14.0 * LB),
            ("Tonne", 1000.0),
        ],
    },
];

/// Converts `v` from unit index `from` to `to` within category `cat`.
pub fn convert(cat: usize, from: usize, to: usize, v: f64) -> f64 {
    let c = &CATEGORIES[cat];
    if c.name == "Temperature" {
        let kelvin = match from {
            0 => v + 273.15,
            1 => (v - 32.0) * 5.0 / 9.0 + 273.15,
            _ => v,
        };
        return match to {
            0 => kelvin - 273.15,
            1 => (kelvin - 273.15) * 9.0 / 5.0 + 32.0,
            _ => kelvin,
        };
    }
    v * c.units[from].1 / c.units[to].1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn idx(cat: &str, unit: &str) -> (usize, usize) {
        let c = CATEGORIES.iter().position(|c| c.name == cat).unwrap();
        (
            c,
            CATEGORIES[c]
                .units
                .iter()
                .position(|u| u.0 == unit)
                .unwrap(),
        )
    }

    #[test]
    fn conversions() {
        let (c, mi) = idx("Length", "Miles");
        let (_, km) = idx("Length", "Kilometers");
        assert!((convert(c, mi, km, 1.0) - 1.609344).abs() < 1e-12);
        let (t, cel) = idx("Temperature", "Degrees Celsius");
        let (_, fah) = idx("Temperature", "Degrees Fahrenheit");
        assert!((convert(t, cel, fah, 100.0) - 212.0).abs() < 1e-9);
        assert!((convert(t, fah, cel, -40.0) + 40.0).abs() < 1e-9);
    }

    /// NIST SP 811 (2008) Appendix B.8/B.9 factors to SI, with the exact definitions where
    /// the guide marks them exact.
    #[test]
    fn factors_match_nist_sp_811() {
        const NIST: &[(&str, &str, f64)] = &[
            ("Area", "Acres", 4.046_856_422_4e3),
            ("Area", "Square feet", 9.290_304e-2),
            ("Area", "Square inch", 6.4516e-4),
            ("Area", "Square mile", 2.589_988_110_336e6),
            ("Area", "Square yard", 8.361_273_6e-1),
            ("Energy", "British thermal unit", 1.055_055_852_62e3),
            ("Energy", "Calorie", 4.1868),
            ("Energy", "Electron-volts", 1.602_176_634e-19),
            ("Energy", "Foot-pound", 1.355_817_948_331_400_4),
            ("Length", "Chains", 2.011_68e1),
            ("Length", "Feet", 3.048e-1),
            ("Length", "Inches", 2.54e-2),
            ("Length", "Miles", 1.609_344e3),
            ("Length", "Nautical miles", 1.852e3),
            ("Length", "PICAs", 4.233_333_333_333_333e-3),
            ("Length", "Rods", 5.0292),
            ("Power", "Horsepower", 7.456_998_715_822_702e2),
            ("Pressure", "Atmosphere", 1.013_25e5),
            ("Pressure", "Millimeter of mercury", 1.333_223_874_15e2),
            (
                "Pressure",
                "Pound per square inch (PSI)",
                6.894_757_293_168_361e3,
            ),
            ("Volume", "Cubic feet", 2.831_684_659_2e-2),
            ("Volume", "Cubic inch", 1.638_706_4e-5),
            ("Volume", "Gallon (UK)", 4.546_09e-3),
            ("Volume", "Gallon (US)", 3.785_411_784e-3),
            ("Volume", "Fluid ounce (US)", 2.957_352_956_25e-5),
            ("Volume", "Fluid ounce (UK)", 2.841_306_25e-5),
            ("Weight/Mass", "Pound", 4.535_923_7e-1),
            ("Weight/Mass", "Ounce", 2.834_952_312_5e-2),
            ("Weight/Mass", "Long ton", 1.016_046_908_8e3),
            ("Weight/Mass", "Short ton", 9.071_847_4e2),
            ("Weight/Mass", "Stone", 6.350_293_18),
            ("Weight/Mass", "Carat", 2e-4),
        ];
        for (cat, unit, si) in NIST {
            let (c, u) = idx(cat, unit);
            let f = CATEGORIES[c].units[u].1;
            let rel = ((f - si) / si).abs();
            assert!(
                rel < 1e-15,
                "{cat}/{unit}: {f:e} vs NIST {si:e} (rel {rel:e})"
            );
        }
    }
}
