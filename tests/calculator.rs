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
        ("200+10%", "220"),
    ] {
        assert_eq!(result(expression), expected, "{expression}");
    }
}
#[test]
fn trigonometric_zeros_and_tiny_float_results_are_representable() {
    let rad = |s: &str| {
        engine::evaluate(
            s,
            Context {
                angle: Angle::Radians,
                ..Default::default()
            },
        )
    };
    for (expression, expected) in [
        ("cos(90)", "0"),
        ("sin(180)", "0"),
        ("tan(180)", "0"),
        ("cos(270)", "0"),
        ("sin(3600)", "0"),
    ] {
        let value = engine::evaluate(expression, Context::default()).unwrap();
        assert_eq!(value.text(), expected, "{expression}");
        assert!(!value.approximate, "{expression} is exact in degrees");
    }
    assert_eq!(rad("sin(pi)").unwrap().text(), "0");
    assert_eq!(rad("cos(pi/2)").unwrap().text(), "0");
    // Small angles are genuine values, not rounding residue.
    assert_eq!(rad("sin(1e-15)").unwrap().text(), "0.000000000000001");
    assert_eq!(
        rad("exp(0-40)").unwrap().text(),
        "0.0000000000000000042483542553"
    );
    assert_eq!(
        rad("exp(0-63)").unwrap().text(),
        "0.0000000000000000000000000004"
    );
    assert_eq!(
        rad("exp(0-70)").unwrap_err().message,
        "The result is smaller than the supported precision"
    );
    assert_eq!(
        rad("sqrt(2e-20)").unwrap().text(),
        "0.00000000014142135623731"
    );
}
#[test]
fn hyperbolic_functions_ignore_the_angle_mode() {
    for angle in [Angle::Degrees, Angle::Radians] {
        let eval = |s: &str| {
            engine::evaluate(
                s,
                Context {
                    angle,
                    ..Default::default()
                },
            )
        };
        assert_eq!(eval("sinh(0)").unwrap().text(), "0");
        assert_eq!(eval("cosh(0)").unwrap().text(), "1");
        assert_eq!(eval("tanh(1)").unwrap().text(), "0.761594155955765");
        assert_eq!(eval("asinh(sinh(2))").unwrap().text(), "2");
        assert_eq!(eval("acosh(1)").unwrap().text(), "0");
        assert!(eval("atanh(0.5)").unwrap().approximate);
        assert!(eval("acosh(0.5)").is_err());
        assert!(eval("atanh(1)").is_err());
        assert!(eval("cosh(1000)").is_err());
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
    book.calculate("sqrt(2)").unwrap();
    assert_eq!(
        book.export(),
        "sin(30) = 0.5 [DEG]\nsqrt(2) = ≈ 1.4142135623731 [DEG]\n"
    );
}
#[test]
fn provably_exact_scientific_results_drop_the_approximation_marker() {
    let deg = |s: &str| engine::evaluate(s, Context::default()).unwrap();
    let rad = |s: &str| {
        engine::evaluate(
            s,
            Context {
                angle: Angle::Radians,
                ..Default::default()
            },
        )
        .unwrap()
    };
    for (expression, expected) in [
        ("sqrt(144)", "12"),
        ("sqrt(2,25)", "1.5"),
        ("cbrt(-27)", "-3"),
        ("log(1000)", "3"),
        ("log(0,01)", "-2"),
        ("ln(1)", "0"),
        ("exp(0)", "1"),
        ("sin(30)", "0.5"),
        ("cos(180)", "-1"),
        ("tan(45)", "1"),
        ("sin(0)", "0"),
        ("asin(0,5)", "30"),
        ("atan(1)", "45"),
        ("sqrt(144) + 8^2", "76"),
    ] {
        let value = deg(expression);
        assert_eq!(value.text(), expected, "{expression}");
        assert!(!value.approximate, "{expression} should be exact");
    }
    for expression in ["sqrt(2)", "sin(1)", "log(2)", "cos(10)", "sqrt(ans)"] {
        assert!(
            deg(expression).approximate || expression == "sqrt(ans)",
            "{expression}"
        );
    }
    // Radians involve π, so the result stays approximate.
    assert!(rad("sin(pi/6)").approximate);
    // An approximate input never becomes exact.
    assert!(deg("sqrt(pi^2)").approximate);
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

fn with(vars: &[(&str, &str)], expression: &str) -> Result<String, String> {
    let variables = vars
        .iter()
        .map(|(k, v)| (k.to_string(), engine::Value::exact(dec(v))))
        .collect();
    engine::evaluate_with(
        expression,
        Context {
            today: Some(rustcalc::dates::from_ymd(2026, 10, 21).unwrap()),
            ..Default::default()
        },
        &variables,
    )
    .map(|v| v.text())
    .map_err(|e| e.message)
}

#[test]
fn variables_and_implicit_multiplication_read_naturally() {
    let vars = [("principal", "1000"), ("rate", "5"), ("n_2", "2")];
    assert_eq!(with(&vars, "principal * (1 + rate%)").unwrap(), "1050");
    assert_eq!(with(&vars, "principal(1 + rate%)^n_2").unwrap(), "1102.5");
    assert_eq!(with(&vars, "2rate").unwrap(), "10");
    assert_eq!(with(&vars, "RATE + 1").unwrap(), "6", "names ignore case");
    assert_eq!(
        with(&vars, "missing + 1").unwrap_err(),
        "Unknown name: missing"
    );
    let parsed = engine::Expression::parse("principal * rate + ans + x + sin(n_2)").unwrap();
    assert_eq!(
        parsed.names().into_iter().collect::<Vec<_>>(),
        ["n_2", "principal", "rate"]
    );
    assert!(engine::is_identifier("annual_rate2"));
    assert!(!engine::is_identifier("2rate") && !engine::is_identifier("a-b"));
    assert!(engine::is_reserved("sin") && engine::is_reserved("PI") && engine::is_reserved("ans"));
    assert!(!engine::is_reserved("rate") && !engine::is_reserved("x"));
}

#[test]
fn multi_argument_functions_use_semicolons() {
    for (expression, expected) in [
        ("min(3; 1,5; 2)", "1.5"),
        ("max(3; -1; 2)", "3"),
        ("sum(1; 2; 3,5)", "6.5"),
        ("avg(1; 2; 3; 4)", "2.5"),
        ("gcd(48; 18; 30)", "6"),
        ("lcm(4; 6; 10)", "60"),
        ("ncr(5; 2)", "10"),
        ("ncr(60; 30)", "118264581564861424"),
        ("npr(5; 2)", "20"),
        ("mod(7; 3)", "1"),
        ("mod(-7; 3)", "2"),
        ("mod(7; -3)", "-2"),
        ("round(2,675; 2)", "2.68"),
        ("round(2.5)", "2"),
        ("round(3.5)", "4"),
        ("sign(-4)", "-1"),
        ("log(8; 2)", "3"),
    ] {
        assert_eq!(with(&[], expression).unwrap(), expected, "{expression}");
    }
    for (expression, error) in [
        ("min()", "Wrong number of arguments for min"),
        ("sqrt(4; 2)", "Wrong number of arguments for sqrt"),
        ("ncr(2; 5)", "Whole numbers required for ncr"),
        ("gcd(1,5; 3)", "Whole numbers required for gcd"),
        ("mod(1; 0)", "Division by zero is undefined"),
        ("round(1; 29)", "Rounding digits must be 0 to 28"),
        ("log(8; 1)", "The logarithm base must be positive and not 1"),
        ("max(1; 2", "Missing closing parenthesis"),
        ("1; 2", "Unexpected token or unmatched parenthesis"),
    ] {
        assert_eq!(with(&[], expression).unwrap_err(), error, "{expression}");
    }
}

#[test]
fn comparisons_and_conditionals_short_circuit() {
    for (expression, expected) in [
        ("3 > 2", "1"),
        ("3 < 2", "0"),
        ("2 + 2 == 4", "1"),
        ("1 != 1", "0"),
        ("1 <> 2", "1"),
        ("2 >= 2", "1"),
        ("2 ≤ 1", "0"),
        ("if(5 > 3; 10; 1/0)", "10"),
        ("if(0; 1/0; 7)", "7"),
        ("and(1; 0; 1/0)", "0"),
        ("or(0; 2; 1/0)", "1"),
        ("not(0)", "1"),
        ("if(1000 > 500; 1000 * 10%; 0) + 1", "101"),
        ("3! == 6", "1"),
    ] {
        assert_eq!(with(&[], expression).unwrap(), expected, "{expression}");
    }
    assert!(with(&[], "if(1; 1/0; 2)").is_err());
    assert!(with(&[], "1 = 1").is_err());
}

#[test]
fn dates_are_day_numbers_with_calendar_functions() {
    let vars = [];
    assert_eq!(with(&vars, "today()").unwrap(), "20747");
    assert_eq!(with(&vars, "date(2026; 10; 21)").unwrap(), "20747");
    assert_eq!(with(&vars, "date(2026; 12; 25) - today()").unwrap(), "65");
    assert_eq!(with(&vars, "weekday(today())").unwrap(), "3");
    assert_eq!(
        with(&vars, "day(addmonths(date(2024; 1; 31); 1))").unwrap(),
        "29"
    );
    assert_eq!(with(&vars, "year(addyears(today(); 10))").unwrap(), "2036");
    assert_eq!(
        with(&vars, "years(date(1990; 10; 22); today())").unwrap(),
        "35"
    );
    assert_eq!(
        with(&vars, "years(date(1990; 10; 21); today())").unwrap(),
        "36"
    );
    assert_eq!(
        with(&vars, "months(date(2026; 1; 15); today())").unwrap(),
        "9"
    );
    assert_eq!(
        with(&vars, "workdays(today(); today() + 14)").unwrap(),
        "10"
    );
    assert_eq!(
        with(&vars, "day(endofmonth(date(2028; 2; 3)))").unwrap(),
        "29"
    );
    assert_eq!(with(&vars, "month(today() + 30)").unwrap(), "11");
    for expression in [
        "date(2026; 2; 30)",
        "date(2026; 13; 1)",
        "year(10000000)",
        "addyears(today(); 9000)",
        "weekday(1,5)",
    ] {
        assert!(with(&vars, expression).is_err(), "{expression}");
    }
}

#[test]
fn a_parsed_expression_is_reusable_across_contexts() {
    let parsed = engine::Expression::parse("x^2 + 1").unwrap();
    let values: Vec<_> = (0..4)
        .map(|x| {
            parsed
                .eval(
                    &Context {
                        x: Decimal::from(x),
                        ..Default::default()
                    },
                    &engine::Variables::new(),
                )
                .unwrap()
                .text()
        })
        .collect();
    assert_eq!(values, ["1", "2", "5", "10"]);
    // Deep nesting is still bounded and long flat chains still evaluate.
    assert!(
        engine::Expression::parse(&format!("{}1{}", "max(".repeat(70), ")".repeat(70))).is_err()
    );
}

#[test]
fn percent_follows_everyday_calculator_rules() {
    for (expression, expected) in [
        ("10%", "0.1"),
        ("100 + 10%", "110"),
        ("100 - 10%", "90"),
        ("100 * 10%", "10"),
        ("100 / 10%", "1000"),
        ("250 - 15%", "212.5"),
        ("80 + 2,5%", "82"),
        ("100 + 10% + 10%", "121"),
        ("200 * (1 + 10%)", "220"),
        ("100 + (5 + 5)%", "110"),
        ("100 - -10%", "110"),
        ("100 + 10% * 2", "100.2"),
        ("50% + 50%", "0.75"),
        ("1200 + 20% - 50", "1390"),
    ] {
        assert_eq!(result(expression), expected, "{expression}");
    }
    assert_eq!(
        with(&[("price", "250"), ("vat", "20")], "price + vat%").unwrap(),
        "300"
    );
    assert!(engine::evaluate("1e-28 + 1e-10%", Context::default()).is_err());
}

/// Every `| `expression` | `result` |` row in the user-facing documents must
/// match what the engine computes, so examples cannot drift from behavior.
#[test]
fn documented_examples_match_the_engine() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut checked = 0;
    for document in [
        "README.md",
        "docs/USER_GUIDE.md",
        "docs/KULLANIM_KILAVUZU.md",
    ] {
        let text = std::fs::read_to_string(root.join(document)).unwrap();
        for line in text.lines().filter(|l| l.starts_with("| `")) {
            let cells: Vec<&str> = line.split('|').map(str::trim).collect();
            let code = |cell: &str| {
                cell.strip_prefix('`')
                    .and_then(|c| c.strip_suffix('`'))
                    .filter(|c| !c.contains('`'))
                    .map(str::to_owned)
            };
            let (Some(expression), Some(expected)) = (code(cells[1]), code(cells[2])) else {
                continue;
            };
            if expected.parse::<f64>().is_err() {
                continue;
            }
            let value = engine::evaluate(
                &expression,
                Context {
                    today: rustcalc::dates::from_ymd(2026, 10, 21),
                    ..Default::default()
                },
            )
            .unwrap_or_else(|e| panic!("{document}: {expression}: {e}"));
            assert_eq!(value.text(), expected, "{document}: {expression}");
            checked += 1;
        }
    }
    assert!(checked >= 30, "only {checked} documented examples found");
}
