# Formulas

The **Formulas** workspace runs reusable calculations: you enter values, results
update as you type. RustCalc ships 21 templates and lets you create your own.

## Templates

| Category | Templates |
| --- | --- |
| Finance | Loan payment, compound interest, savings plan, add/remove VAT, profit margin, hourly to yearly pay |
| Dates | Days between dates, add or subtract days, workday deadline, add months or years, age |
| Everyday | Discount, percentage change, percentage of a total, split the bill, trip fuel cost, travel time, unit prices, electricity cost |
| Health | Body mass index |

Templates are read-only. **Customize a copy** opens the editor with a copy in the
current language, saved under your own formulas.

Inputs accept expressions as well as numbers: `1500*12`, `ans`, or an earlier
input's name. Date inputs accept `2026-10-21`, `21.10.2026`, `21/10/2026` or an
expression such as `today() + 30`; **Today** fills in the current date. Click a
result to copy it. Money, percent and number results copy as plain numbers.

## Creating a formula

Select **+ New formula**. A formula has:

- **Inputs**: a variable name, a label, a kind (number or date) and a default.
- **Results**: a variable name, a label, an expression, a display format and an
  optional unit. Each result can use the inputs and the results above it.

Variable names start with a letter and contain letters, digits or `_`; case is
ignored. Function names, `pi`, `e`, `ans` and `mem` are reserved. The editor
validates while you type and previews every result with the default values.

### Example: room paint

| Inputs | Name | Default |
| --- | --- | --- |
| Width (m) | `width` | 4 |
| Length (m) | `length` | 5 |
| Height (m) | `height` | 2.6 |
| Coverage (m² per litre) | `coverage` | 10 |

| Result | Expression | Format |
| --- | --- | --- |
| Wall area | `2 * (width + length) * height` | Number, unit `m²` |
| Paint for two coats | `ceil(area * 2 / coverage)` | Number, unit `L` |

Name the first result `area` so the second can use it.

### Display formats

| Format | Shows |
| --- | --- |
| Number | Up to ten decimals; `≈` for approximate values |
| Money | Two decimals with digit grouping (`1,234.50`, Turkish `1.234,50`) |
| Percent | The value with a percent sign; compute it as `part / whole * 100` |
| Date | A day number as a date with its weekday |
| Day of the week | Only the weekday name |

## Expression reference

Everything from the calculator works, plus:

| Function | Meaning |
| --- | --- |
| `min(a; b; …)`, `max`, `sum`, `avg` | Over up to 64 values |
| `round(x; digits)` | Ties to even; digits 0–28 |
| `mod(a; b)` | Remainder with the divisor's sign |
| `gcd`, `lcm`, `ncr(n; k)`, `npr(n; k)` | Whole numbers |
| `log(x; base)`, `sign(x)` | Logarithm in any base; −1, 0 or 1 |
| `if(condition; then; else)` | Evaluates only the chosen branch |
| `and(…)`, `or(…)`, `not(x)` | Short-circuit logic; nonzero is true |
| `<  >  <=  >=  ==  !=` | Comparisons give 1 or 0 |

Arguments are separated by `;` because the comma is a decimal separator.

Percent follows the calculator rule: `price + rate%` adds `rate` percent of
`price` and `price - rate%` subtracts it, while `price * rate%` is that share on
its own. Writing `price * rate / 100` is equivalent and often clearer in a
formula.

### Dates

Dates are whole day numbers, so `end - start` is a number of days and
`start + 30` is a date thirty days later.

| Function | Meaning |
| --- | --- |
| `today()`, `date(y; m; d)` | Today, or a calendar date (years 1–9999) |
| `year(d)`, `month(d)`, `day(d)`, `weekday(d)` | Parts; weekday 1 = Monday … 7 = Sunday |
| `addmonths(d; n)`, `addyears(d; n)` | Calendar arithmetic; 31 January + 1 month = end of February |
| `addworkdays(d; n)` | Skip Saturdays and Sundays; public holidays are not known |
| `months(a; b)`, `years(a; b)` | Complete months or years, for ages and terms |
| `workdays(a; b)` | Monday–Friday days from `a` up to, not including, `b` |
| `endofmonth(d)` | Last day of the month |

## Sharing

**Export…** saves all your formulas to a JSON file; **Import…** adds formulas
from one. Imports are validated completely before anything changes, skip
formulas identical to ones you have, and keep at most 200 formulas. Files are
limited to 1 MiB. A formula file never runs code: expressions only reach the
bounded calculator engine.

```json
{
  "format": "rustcalc-formulas",
  "version": 1,
  "formulas": [
    {
      "name": "Room paint",
      "category": "Home",
      "description": "",
      "inputs": [{ "name": "width", "label": "Width (m)", "kind": "number", "default": "4" }],
      "outputs": [{ "name": "area", "label": "Wall area", "expression": "width * 2.6", "format": "number", "unit": "m²" }]
    }
  ]
}
```

Your formulas are stored with history and preferences in the RustCalc state file
and survive updates.
