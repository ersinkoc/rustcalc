use rust_decimal::Decimal;
use rustcalc::{
    engine::{self, Angle, Context},
    model::Notebook,
    units,
};
use std::str::FromStr;

fn result(s: &str) -> String {
    engine::evaluate(s, Context::default())
        .unwrap_or_else(|e| panic!("{s}: {e}"))
        .text()
}
fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

#[test]
fn decimal_arithmetic_avoids_binary_rounding_errors() {
    for (expression, expected) in [
        ("0.1 + 0.2", "0.3"),
        ("1.01 - 0.99", "0.02"),
        ("19.99*3", "59.97"),
        ("1/8", "0.125"),
        ("0,1 + 0,2", "0.3"),
        (".5 * 2", "1"),
        ("1e-3 * 1000", "1"),
    ] {
        assert_eq!(result(expression), expected, "{expression}");
    }
}
#[test]
fn precedence_and_associativity_follow_mathematical_conventions() {
    for (expression, expected) in [
        ("2+3*4", "14"),
        ("(2+3)*4", "20"),
        ("2^3^2", "512"),
        ("-2^2", "-4"),
        ("(-2)^2", "4"),
        ("2^-3", "0.125"),
        ("10-3-2", "5"),
        ("24/4/2", "3"),
        ("3!^2", "36"),
        ("2(3+4)", "14"),
        ("200*(1+10%)", "220"),
        ("200+10%", "200.1"),
    ] {
        assert_eq!(result(expression), expected, "{expression}");
    }
}
#[test]
fn degree_and_radian_functions_are_explicit() {
    assert_eq!(result("sin(30)"), "0.5");
    assert_eq!(result("asin(0.5)"), "30");
    let value = engine::evaluate(
        "sin(pi/2)",
        Context {
            angle: Angle::Radians,
            ..Default::default()
        },
    )
    .unwrap();
    assert!((value.number - Decimal::ONE).abs() < dec("0.000000000001"));
    assert!(value.approximate);
    assert!(
        !engine::evaluate("0.1+0.2", Context::default())
            .unwrap()
            .approximate
    );
}
#[test]
fn scientific_functions_and_constants_work() {
    for (expression, expected) in [
        ("sqrt(144)", "12"),
        ("cbrt(-8)", "-2"),
        ("log(1000)", "3"),
        ("ln(1)", "0"),
        ("exp(0)", "1"),
        ("abs(-12)", "12"),
        ("floor(-1.2)", "-2"),
        ("ceil(-1.2)", "-1"),
        ("0!", "1"),
        ("27!", "10888869450418352160768000000"),
    ] {
        assert_eq!(result(expression), expected);
    }
    assert!(result("2pi").starts_with("6.28318"));
    assert!(result("2e").starts_with("5.43656"));
}
#[test]
fn invalid_domains_and_overflows_are_errors() {
    for expression in [
        "1/0",
        "0^-1",
        "0^0",
        "sqrt(-1)",
        "ln(0)",
        "acos(2)",
        "tan(90)",
        "(-1)!",
        "1.5!",
        "28!",
        "10^29",
        "2^10001",
        "1e-100",
        "0.1^100",
        "exp(1000)",
    ] {
        assert!(
            engine::evaluate(expression, Context::default()).is_err(),
            "{expression}"
        );
    }
}
#[test]
fn malformed_input_never_panics() {
    for expression in [
        "", "(", ")", "1+", "sin", "sin(", "sin()", "foo(2)", "1..2", "2 3", "2**3", "1e+", "1;2",
        "∞", "sqrt(4))", "((((1)",
    ] {
        assert!(
            engine::evaluate(expression, Context::default()).is_err(),
            "{expression}"
        );
    }
}
#[test]
fn resource_limits_bound_untrusted_input() {
    assert!(engine::evaluate(&"1".repeat(4097), Context::default()).is_err());
    assert!(
        engine::evaluate(
            &format!("{}1{}", "(".repeat(100), ")".repeat(100)),
            Context::default()
        )
        .is_err()
    );
    let long_sum = std::iter::repeat_n("1", 1000).collect::<Vec<_>>().join("+");
    assert_eq!(result(&long_sum), "1000");
}
#[test]
fn unicode_input_matches_keyboard_input() {
    assert_eq!(result("(12 − 2) × 3 ÷ 5"), "6");
}
#[test]
fn answer_and_memory_are_resolved_from_the_notebook() {
    let mut book = Notebook::default();
    book.calculate("12*3").unwrap();
    assert_eq!(book.calculate("ans+4").unwrap().text(), "40");
    book.memory_add(engine::evaluate("0.1", book.context()).unwrap(), false)
        .unwrap();
    book.memory_add(engine::evaluate("0.2", book.context()).unwrap(), false)
        .unwrap();
    assert_eq!(book.calculate("mem*10").unwrap().text(), "3");
    assert_eq!(book.entries.len(), 3);
}
#[test]
fn failed_calculation_preserves_answer_and_history() {
    let mut book = Notebook::default();
    book.calculate("7").unwrap();
    assert!(book.calculate("1/0").is_err());
    assert_eq!(book.answer, "7");
    assert_eq!(book.entries.len(), 1);
    book.calculate("7").unwrap();
    assert_eq!(book.entries.len(), 1);
}
#[test]
fn history_is_bounded_and_preserves_pins() {
    let mut book = Notebook::default();
    book.calculate("1").unwrap();
    book.entries[0].pinned = true;
    for i in 2..=510 {
        book.calculate(&i.to_string()).unwrap();
    }
    assert_eq!(book.entries.len(), 500);
    assert!(book.entries[0].pinned);
    assert_eq!(book.entries[0].result, "1");
    assert_eq!(book.entries.last().unwrap().result, "510");
}
#[test]
fn export_is_readable_and_marks_angle_and_approximation() {
    let mut book = Notebook::default();
    book.calculate("sin(30)").unwrap();
    assert_eq!(book.export(), "sin(30) = ≈ 0.5 [DEG]\n");
}
#[test]
fn unit_conversions_use_defined_exact_factors() {
    assert_eq!(units::convert(dec("1"), 0, 4, 0).unwrap(), dec("1609.344"));
    assert_eq!(
        units::convert(dec("1"), 1, 4, 0).unwrap(),
        dec("0.45359237")
    );
    assert_eq!(units::convert(dec("2"), 3, 3, 0).unwrap(), dec("2000"));
    assert_eq!(
        units::convert(dec("1"), 4, 3, 0).unwrap(),
        dec("3.785411784")
    );
    assert_eq!(units::convert(dec("2"), 6, 3, 0).unwrap(), dec("7200"));
    assert_eq!(
        units::convert(dec("1"), 7, 6, 0).unwrap(),
        dec("1073741824")
    );
    assert_eq!(units::convert(dec("8"), 7, 7, 0).unwrap(), dec("1"));
}
#[test]
fn temperature_offsets_and_absolute_zero_are_validated() {
    for (input, from, to, expected) in [
        ("0", 0, 1, "32"),
        ("100", 0, 1, "212"),
        ("32", 1, 0, "0"),
        ("-40", 0, 1, "-40"),
        ("0", 2, 0, "-273.15"),
        ("-459.67", 1, 2, "0"),
    ] {
        assert_eq!(
            units::convert(dec(input), 2, from, to).unwrap(),
            dec(expected)
        );
    }
    assert!(units::convert(dec("-273.16"), 2, 0, 1).is_err());
    assert!(units::convert(dec("-1"), 2, 2, 2).is_err());
}
#[test]
fn converter_rejects_invalid_selections_and_overflow() {
    assert!(units::convert(Decimal::ONE, 999, 0, 0).is_err());
    assert!(units::convert(Decimal::ONE, 0, 999, 0).is_err());
    assert!(units::convert(Decimal::MAX, 0, 1, 0).is_err());
}
#[test]
fn programmer_parses_full_u64_without_float_precision_loss() {
    for (s, base, n) in [
        ("FFFFFFFFFFFFFFFF", 16, u64::MAX),
        ("18446744073709551615", 10, u64::MAX),
        ("0xFF", 16, 255),
        ("0b1111_0000", 2, 240),
        ("377", 8, 255),
    ] {
        assert_eq!(engine::parse_integer(s, base).unwrap(), n);
    }
    for (s, base) in [
        ("18446744073709551616", 10),
        ("2", 2),
        ("-1", 10),
        ("FF", 10),
        ("1", 3),
    ] {
        assert!(engine::parse_integer(s, base).is_err());
    }
}

#[test]
fn arithmetic_rejects_underflow_instead_of_returning_a_false_zero() {
    for input in ["1e-20*1e-20", "1e-28/10", "1e-28%"] {
        assert!(
            engine::evaluate(input, Context::default()).is_err(),
            "{input}"
        );
    }
}

#[test]
fn ans_and_memory_retain_the_approximation_marker() {
    let mut book = Notebook::default();
    let approximate = book.calculate("sqrt(2)").unwrap();
    assert!(book.calculate("ans*2").unwrap().approximate);
    book.memory_add(approximate, false).unwrap();
    assert!(book.calculate("mem+1").unwrap().approximate);
}
