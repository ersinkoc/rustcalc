use rust_decimal::Decimal;
use std::str::FromStr;

#[derive(Clone, Copy)]
pub struct Unit {
    pub name: &'static str,
    pub symbol: &'static str,
    factor: &'static str,
}
pub struct Category {
    pub name: &'static str,
    pub units: &'static [Unit],
}
macro_rules! unit {
    ($name:expr, $symbol:expr, $factor:expr) => {
        Unit {
            name: $name,
            symbol: $symbol,
            factor: $factor,
        }
    };
}
pub const CATEGORIES: &[Category] = &[
    Category {
        name: "Length",
        units: &[
            unit!("Metre", "m", "1"),
            unit!("Kilometre", "km", "1000"),
            unit!("Centimetre", "cm", "0.01"),
            unit!("Millimetre", "mm", "0.001"),
            unit!("Mile", "mi", "1609.344"),
            unit!("Yard", "yd", "0.9144"),
            unit!("Foot", "ft", "0.3048"),
            unit!("Inch", "in", "0.0254"),
        ],
    },
    Category {
        name: "Mass",
        units: &[
            unit!("Kilogram", "kg", "1"),
            unit!("Gram", "g", "0.001"),
            unit!("Milligram", "mg", "0.000001"),
            unit!("Tonne", "t", "1000"),
            unit!("Pound", "lb", "0.45359237"),
            unit!("Ounce", "oz", "0.028349523125"),
        ],
    },
    Category {
        name: "Temperature",
        units: &[
            unit!("Celsius", "°C", "1"),
            unit!("Fahrenheit", "°F", "1"),
            unit!("Kelvin", "K", "1"),
        ],
    },
    Category {
        name: "Area",
        units: &[
            unit!("Square metre", "m²", "1"),
            unit!("Square kilometre", "km²", "1000000"),
            unit!("Hectare", "ha", "10000"),
            unit!("Decare", "daa", "1000"),
            unit!("Square foot", "ft²", "0.09290304"),
        ],
    },
    Category {
        name: "Volume",
        units: &[
            unit!("Litre", "L", "1"),
            unit!("Millilitre", "mL", "0.001"),
            unit!("Cubic metre", "m³", "1000"),
            unit!("US gallon", "gal", "3.785411784"),
        ],
    },
    Category {
        name: "Speed",
        units: &[
            unit!("Metres / second", "m/s", "1"),
            unit!(
                "Kilometres / hour",
                "km/h",
                "0.2777777777777777777777777778"
            ),
            unit!("Miles / hour", "mph", "0.44704"),
            unit!("Knot", "kn", "0.5144444444444444444444444444"),
        ],
    },
    Category {
        name: "Time",
        units: &[
            unit!("Second", "s", "1"),
            unit!("Millisecond", "ms", "0.001"),
            unit!("Minute", "min", "60"),
            unit!("Hour", "h", "3600"),
            unit!("Day", "d", "86400"),
            unit!("Week", "wk", "604800"),
        ],
    },
    Category {
        name: "Data",
        units: &[
            unit!("Byte", "B", "1"),
            unit!("Kilobyte", "kB", "1000"),
            unit!("Megabyte", "MB", "1000000"),
            unit!("Gigabyte", "GB", "1000000000"),
            unit!("Kibibyte", "KiB", "1024"),
            unit!("Mebibyte", "MiB", "1048576"),
            unit!("Gibibyte", "GiB", "1073741824"),
            unit!("Bit", "bit", "0.125"),
        ],
    },
];

pub fn convert(value: Decimal, category: usize, from: usize, to: usize) -> Result<Decimal, String> {
    let category = CATEGORIES.get(category).ok_or("Invalid category")?;
    let source = category.units.get(from).ok_or("Invalid source unit")?;
    let target = category.units.get(to).ok_or("Invalid target unit")?;
    let checked = |v: Option<Decimal>| {
        v.ok_or_else(|| "Conversion outside the supported number range".to_string())
    };
    if category.name == "Temperature" {
        let d = |s: &str| Decimal::from_str(s).expect("static decimal");
        let kelvin = match from {
            0 => checked(value.checked_add(d("273.15")))?,
            1 => checked(
                checked(checked(value.checked_sub(d("32")))?.checked_mul(d("5")))?
                    .checked_div(d("9")),
            )?
            .checked_add(d("273.15"))
            .ok_or("Conversion outside the supported number range")?,
            _ => value,
        };
        if kelvin < Decimal::ZERO {
            return Err("Temperature cannot be below absolute zero".into());
        }
        return match to {
            0 => checked(kelvin.checked_sub(d("273.15"))),
            1 => checked(
                checked(
                    checked(checked(kelvin.checked_sub(d("273.15")))?.checked_mul(d("9")))?
                        .checked_div(d("5")),
                )?
                .checked_add(d("32")),
            ),
            _ => Ok(kelvin),
        };
    }
    if from == to {
        return Ok(value);
    }
    let a = Decimal::from_str(source.factor).expect("static factor");
    let b = Decimal::from_str(target.factor).expect("static factor");
    checked(checked(value.checked_mul(a))?.checked_div(b))
}
