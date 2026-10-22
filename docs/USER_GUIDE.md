# RustCalc user guide

This guide explains every part of RustCalc with examples. The same guide is
available in the app: press **F1** or select **? Help** in the sidebar.
[Türkçe kullanım kılavuzu](KULLANIM_KILAVUZU.md)

- [Calculator](#calculator)
- [Percent](#percent)
- [Exact and approximate results](#exact-and-approximate-results)
- [Functions](#functions)
- [Dates](#dates)
- [Memory and history](#memory-and-history)
- [Formulas](#formulas)
- [Unit converter](#unit-converter)
- [Programmer](#programmer)
- [Plots](#plots)
- [Settings, language and updates](#settings-language-and-updates)

## Calculator

Type an expression; the result appears as you type in **LIVE PREVIEW**. Press
**Enter** (or `=`) to save it, and it moves to the history. You can type at any
time without clicking the input first, and use the on-screen keypad with the
mouse. **Standard** shows the basic keypad; **Scientific** adds functions.

After a result:

- typing or pasting a number starts a new calculation;
- typing an operator (`+ - * / ^ %`) continues from the result (`ans`);
- clicking inside the previous expression and typing edits it.

The input is a normal text field: move the cursor, select text, paste, undo
(Ctrl+Z). Keypad buttons insert at the cursor or replace the selection; **Del**
removes the character before the cursor and **AC** (or Esc) clears everything.
In the history search box, Esc clears only the search. The calculator keeps
working while the Help window (F1) is open, so you can try its examples. When
you type `<=`, `>=` or `!=`, the `=` is part of the comparison and does not
calculate.

Missing closing brackets are added at the end, so `sqrt(16` already previews 4
and Enter saves it as `sqrt(16)`.

### Order of operations

| You type | Result | Why |
| --- | --- | --- |
| `2 + 3 * 4` | `14` | Multiplication and division before addition and subtraction |
| `(2 + 3) * 4` | `20` | Parentheses first |
| `2^3^2` | `512` | Powers group from the right: 2^(3^2) |
| `-2^2` | `-4` | The power applies before the minus |
| `(-2)^2` | `4` | Put the minus inside parentheses to square it |
| `3!^2` | `36` | Factorial and percent bind tightest |
| `2(3+4)` | `14` | A number next to a bracket multiplies |
| `24 / 4 / 2` | `3` | Same-level operations run left to right |

### Decimal point

Use a comma or a period: `1,5` and `1.5` are both one and a half. Do not type
thousands separators: `1,000` is one, not one thousand. `1e3` means 1000 and
`2,5e-3` means 0.0025.

## Percent

RustCalc follows the convention of shop and phone calculators. Adding or
subtracting a percentage changes the number by that share of itself:

| You type | Result | Meaning |
| --- | --- | --- |
| `100 + 10%` | `110` | 100 plus 10% of 100 |
| `100 - 10%` | `90` | 100 minus 10% of 100 |
| `250 - 15%` | `212.5` | A 15% discount on 250 |
| `100 * 10%` | `10` | 10% of 100 |
| `100 / 10%` | `1000` | 100 divided by 0.1 |
| `10%` | `0.1` | On its own, a percentage is a fraction |
| `100 + 10% + 10%` | `121` | Each step uses the running total |
| `200 * (1 + 10%)` | `220` | Inside the parentheses, 1 + 10% of 1 = 1.1 |
| `100 + (5 + 5)%` | `110` | The percentage can be a bracketed expression |

The rule applies when `+` or `-` is directly followed by a percentage. In
`100 + 10% * 2` the percentage is part of a product (0.1 × 2), so the result is
100.2. When unsure, write the share explicitly: `100 + 100 * 10%`.

## Exact and approximate results

Ordinary arithmetic uses exact decimals:

| You type | Result |
| --- | --- |
| `0.1 + 0.2` | `0.3` |
| `19.99 * 3` | `59.97` |
| `1 / 8` | `0.125` |

Results can have up to 28 decimal places and a size of about ±7.9 × 10²⁸.
A division that never ends, such as `1/3`, is marked ≈ and shown with 15
significant digits (`≈ 0.333333333333333`); RustCalc keeps 28 digits inside, so
`1/3*3` still gives `1`.

Scientific functions (roots, logarithms, trigonometry, non-integer powers) are
computed to about 15 significant digits and marked **≈**. When the result is
provably exact, the ≈ disappears:

| You type | Result |
| --- | --- |
| `sqrt(144)` | `12` |
| `log(1000)` | `3` |
| `sin(30)` | `0.5` |
| `asin(0.5)` | `30` |

`sqrt(2)` shows ≈ 1.4142135623731. The marker follows the value into `ans`, the
memory and the history.

Errors such as division by zero, `sqrt(-1)`, `tan(90)` or results beyond the
supported range appear under the result; nothing is saved.

When a result is a simple fraction (denominator up to 1000), it is shown under
the result too: `0.125` shows `= 1/8`, `1/3` shows `= 1/3` and `5/3` shows
`= 5/3 = 1 2/3`.

## Functions

Separate arguments with `;` because the comma is a decimal point.

| You type | Result | Function |
| --- | --- | --- |
| `sqrt(16)` | `4` | Square root (`cbrt` for cube root) |
| `log(100)` | `2` | Base-10 logarithm; `ln` is natural |
| `log(8; 2)` | `3` | Logarithm in any base |
| `abs(-7)` | `7` | Absolute value |
| `floor(2.7)` | `2` | Round down (`ceil` rounds up) |
| `round(2.675; 2)` | `2.68` | Round to digits; halves round up |
| `round(2.5)` | `3` | Halves round away from zero, as at school |
| `round(1234; -2)` | `1200` | Negative digits round to tens, hundreds … |
| `max(4; 9; 2)` | `9` | Largest (`min`, `sum`, `avg` likewise) |
| `avg(2; 4; 9)` | `5` | Average |
| `mod(17; 5)` | `2` | Remainder, with the divisor's sign |
| `gcd(48; 18)` | `6` | Greatest common divisor (`lcm` likewise) |
| `ncr(5; 2)` | `10` | Combinations (`npr` for permutations) |
| `5!` | `120` | Factorial, 0 to 27 |
| `if(7 > 5; 1; 0)` | `1` | Choose a value by a condition |

Comparisons `<`, `>`, `<=`, `>=`, `==` and `!=` give 1 (true) or 0 (false).
`and`, `or` and `not` combine them. `if`, `and` and `or` evaluate only what they
need, so `if(x == 0; 0; 1/x)` never divides by zero.

Trigonometry (`sin`, `cos`, `tan`, `asin`, `acos`, `atan`) follows **DEG/RAD**;
hyperbolic functions (`sinh`, `cosh`, `tanh`, `asinh`, `acosh`, `atanh`) take
plain numbers. Constants: `pi`, `e`, `ans` (last result), `mem` (memory).

## Dates

Dates are day numbers, so ordinary arithmetic works on them:

| You type | Meaning |
| --- | --- |
| `date(2026; 12; 31) - today()` | Days until 31 December 2026 |
| `today() + 90` | The date 90 days from now (shown as a day number in the calculator) |
| `weekday(today())` | 1 = Monday … 7 = Sunday |
| `years(date(1990; 5; 15); today())` | Complete years since a date (an age) |
| `workdays(today(); date(2026; 12; 31))` | Monday–Friday days until then |
| `addmonths(date(2024; 1; 31); 1)` | 29 February 2024: day 31 becomes the month's last day |

The calculator shows dates as day numbers; the **Formulas** workspace shows them
as dates and has ready-made date calculations. Public holidays are not known.
All date functions are listed in the [formula guide](FORMULAS.md#dates).

## Variables

Give a value a name with `name = expression` and press Enter:

| You type | Result | Meaning |
| --- | --- | --- |
| `vat = 20` | `20` | Stores 20 as `vat` |
| `1500 + vat%` | `1800` | Uses it like a number |
| `vat = vat / 2` | `10` | Replaces it, using the old value |

Variables are saved with your history and appear under the result: click one to
insert its name, **×** to delete it. They also work in plots (`a*x^2`) and in
the unit converter. Names start with a letter (accented and Turkish letters
work too) and may contain letters, digits and `_`. `pi`, `e`, `ans`, `mem`, `x`,
function names and names like `e1`, which read as scientific notation, are
taken.
Typing `=` right after a name starts an assignment; anywhere else `=`
calculates, as always.

## Memory and history

| Button | Action |
| --- | --- |
| MS | Store the current result in memory |
| M+ / M− | Add the current result to / subtract it from memory |
| MR | Insert `mem` into the expression |
| MC | Clear memory |

The history keeps up to 500 calculations, newest first, under **Today**,
**Yesterday** or date headings; each entry shows the time it was calculated. Click a result to start
a new calculation with it, **↗** to edit its expression again, **☆** to pin it
(pinned entries are kept when old ones are removed), **Copy** to copy and **×**
to delete.
Search filters by expression or result; **Pinned** shows only pinned entries.

**Files…** saves a JSON backup, exports a readable text file, or imports a
backup (it merges, skips duplicates and asks for confirmation). **Undo deletion**
restores deleted entries or a cleared history during the current session.
Details: [history backups](HISTORY.md).

## Formulas

**05 Formulas** runs ready-made and custom calculations. Choose a formula on the
left, type values, and read the results; they update as you type.

- Number fields accept expressions: `1500*12`, `ans`, or an earlier field's name.
- Date fields accept `2026-10-21`, `21.10.2026`, `21/10/2026` or expressions such
  as `today() + 30`; **Today** fills in the current date.
- Click a result to copy it. **Reset values** restores the defaults.

Templates: loan payment, compound interest, savings plan, add VAT, remove VAT,
profit margin, hourly to yearly pay, days between dates, add or subtract days,
workday deadline, add months or years, age, discount, percentage change,
percentage of a total, split the bill, trip fuel cost, travel time, compare unit
prices, electricity cost and body mass index.

**+ New formula** creates your own; **Customize a copy** starts from a template.
The [formula guide](FORMULAS.md) walks through an example and lists everything
an expression can use.

## Unit converter

Choose a category (length, mass, temperature, area, volume, speed, time, data,
pressure, energy, power),
type the value (or an expression such as `1500*12`) at the top and pick the
units. **Swap units** reverses the direction; **Copy result** copies the converted value. Conversion factors are
exact definitions, for example 1 mile = 1609.344 m and 1 lb = 0.45359237 kg.
Temperatures below absolute zero are rejected. Data units come in SI (1 kB =
1000 B) and binary (1 KiB = 1024 B) forms.

**In every unit**, below, lists the value in each unit of the category at once;
choose a row to make that unit the target. Values show 15 significant digits,
like a calculator result; **Copy result** copies every digit.

## Programmer

1. Choose the base you type in: **HEX**, **DEC**, **OCT** or **BIN**.
2. Choose the word size (**8, 16, 32 or 64-bit**) and **Signed** for two's
   complement.
3. Read the value in every base; click a value to copy it. Click bits in the bit
   map to flip them.
4. Enter a second operand (decimal) for `+ − × ÷ MOD AND OR XOR << n >> n`;
   `NOT`, `NEG`, `<< 1` and `>> 1` work on the value alone.

Results wrap to the word size like hardware integers: in signed 8-bit, 127 + 1 is
−128. Division truncates toward zero, and `>>` keeps the sign for signed words.
DEC input must fit the word (−128 to 127 for signed 8-bit); HEX, OCT and BIN input
is a bit pattern of at most the word's width. Switching to a smaller word keeps
the low bits; the **Signed** switch keeps the bits and changes only the decimal
meaning.

## Plots

Enter up to three functions of `x` as **f**, **g** and **h** (leave a row empty to
hide it). The presets fill in **f**.

- Drag to move; scroll or pinch to zoom around the pointer; double-click to reset.
- **+ / −** zoom, **Reset view** restores ±10, and **View ±** sets the visible
  range. Over the plot, arrow keys move, `+`/`-` zoom and Home resets.
- Hover to read the value of every curve at the pointer.
- **Roots and intersections** marks where curves cross zero or each other and
  lists the x values under the plot. Touching zero without crossing (like `x^2`
  at 0) is not listed.

Both axes show the same numeric range; plots always use radians. Undefined
points and poles leave gaps instead of false vertical lines. A function that
cannot be drawn shows the reason under the plot.

## Settings, language and updates

- **⚙ Settings** (Ctrl+,) holds every preference:
  - **Language** — English or Türkçe (also **EN / TR** in the sidebar).
  - **Appearance** — follow the system, light or dark (the sidebar button
    switches between light and dark).
  - **Text size** — 80 % to 175 %; Ctrl++ and Ctrl+− step through the sizes and
    Ctrl+0 resets to 100 %.
  - **Numbers** — the decimal mark (by language: a comma in Turkish, a point in
    English; or always a point or a comma), digit grouping (`1 234 567`) and the
    number of decimals shown (all digits, or a fixed 0–10, rounded half up).
    Only the display changes: RustCalc calculates with every digit, you can
    type either mark, and copied results are not grouped so they paste back.
- Ctrl+1 to Ctrl+5 switch between the five workspaces. RustCalc reopens the
  workspace, keypad, converter units and formula you used last.
- **About & updates** shows the version, checks for updates and sets automatic
  updates: **Off** (default), **Notify me** or **Install automatically**. On first
  launch the status bar asks once. Automatic installs are verified like manual
  ones and take effect when you restart; RustCalc never restarts by itself.
- `rustcalc update --rollback` returns to the version the last update replaced.
- Screen readers (Narrator, NVDA, Orca, VoiceOver) read every control by name:
  keys such as **×** and **Del** are announced as "Multiply" and "Delete last
  character", the history icons as "Pin", "Edit this expression" and "Delete
  this calculation", and text fields by their purpose.

Everything you do stays on your computer: history, memory, settings and your
formulas are saved in `%APPDATA%\RustCalc\data\app.ron` on Windows,
`~/.local/share/rustcalc/app.ron` on Linux and
`~/Library/Application Support/RustCalc/app.ron` on macOS.
