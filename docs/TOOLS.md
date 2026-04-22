# Tool Catalog — 87 Expert Calculators

Complete reference for all 87 tools available in arithma, organized by category.

## Basic Math (7 tools)

Arbitrary-precision arithmetic using BigDecimal.

| Tool | Input | Output | Example |
|:---|:---|:---|:---|
| `add` | `first`, `second` | Sum (exact) | `add("0.1", "0.2")` → `"0.3"` |
| `subtract` | `first`, `second` | Difference | `subtract("1.0", "0.9")` → `"0.1"` |
| `multiply` | `first`, `second` | Product | `multiply("2", "3")` → `"6"` |
| `divide` | `first`, `second` | Quotient (scale 20) | `divide("10", "3")` → `"3.33333..."` |
| `power` | `base`, `exponent` | Base^exponent | `power("2", "10")` → `"1024"` |
| `modulo` | `first`, `second` | Remainder | `modulo("10", "3")` → `"1"` |
| `abs` | `value` | Absolute value | `abs("-42")` → `"42"` |

## Scientific (7 tools)

Trigonometry and transcendental functions with exact values at notable angles.

| Tool | Input | Output | Notes |
|:---|:---|:---|:---|
| `sin` | `degrees` | Sine (exact at 0/30/45/60/90°) | `sin(45)` → `"0.7071..."` |
| `cos` | `degrees` | Cosine | `cos(60)` → `"0.5"` (exact) |
| `tan` | `degrees` | Tangent | `tan(45)` → `"1.0"` (exact) |
| `sqrt` | `number` | Square root | `sqrt(2)` → `"1.414..."` |
| `log` | `number` | Natural log | `log(2.71828)` → `"1.0"` (approx) |
| `log10` | `number` | Base-10 log | `log10(1000)` → `"3.0"` |
| `factorial` | `num` (0–20) | n! | `factorial(5)` → `"120"` |

## Programmable (4 tools)

Expression evaluation with variables.

| Tool | Input | Output | Use Case |
|:---|:---|:---|:---|
| `evaluate` | `expression` | Result (f64 precision) | Parse and evaluate math expressions |
| `evaluateWithVariables` | `expression`, `variables` (JSON) | Result | Evaluate with variable substitution |
| `evaluateExact` | `expression` | Result (128-bit precision) | High-precision expression evaluation |
| `evaluateExactWithVariables` | `expression`, `variables` (JSON) | Result (128-bit) | High-precision with variables |

**Supported operators**: `+`, `-`, `*`, `/`, `^` (power), `%` (modulo)  
**Built-in functions**: `sin`, `cos`, `tan`, `log`, `log10`, `sqrt`, `abs`, `ceil`, `floor`

## Vectors & Arrays (4 tools)

SIMD-accelerated array operations.

| Tool | Input | Output | Example |
|:---|:---|:---|:---|
| `sumArray` | CSV numeric array | Sum | `sumArray("1,2,3,4,5")` → `"15"` |
| `dotProduct` | Two CSV arrays | Dot product | `dotProduct("1,2,3", "4,5,6")` → `"32"` |
| `scaleArray` | CSV array, scalar | Scaled array (CSV) | `scaleArray("1,2,3", "2")` → `"2,4,6"` |
| `magnitudeArray` | CSV array | L2 norm | `magnitudeArray("3,4")` → `"5"` |

## Financial (6 tools)

Time-value-of-money calculations with DECIMAL128 precision.

| Tool | Input | Output | Use Case |
|:---|:---|:---|:---|
| `compoundInterest` | Principal, rate (%), years, compounds/year | Future value | Calculate compound interest |
| `loanPayment` | Principal, annual rate (%), years | Monthly payment | Fixed-rate loan payment |
| `presentValue` | Future value, rate (%), years | Present value | Discount cash flows |
| `futureValueAnnuity` | Payment, rate (%), years | Future value | Series of equal payments |
| `returnOnInvestment` | Gain, cost | ROI (%) | Investment performance |
| `amortizationSchedule` | Principal, rate (%), years | Month-by-month table | Full amortization breakdown |

## Calculus (4 tools)

Numerical calculus operations.

| Tool | Input | Output | Notes |
|:---|:---|:---|:---|
| `derivative` | Expression, variable, point | Numeric derivative | Five-point central difference |
| `nthDerivative` | Expression, variable, point, order (1–10) | nth derivative | Higher-order derivatives |
| `definiteIntegral` | Expression, variable, lower, upper | Integral value | Composite Simpson's rule (10k intervals) |
| `tangentLine` | Expression, variable, point | `{slope, yIntercept, equation}` | Tangent at a point |

## Unit Conversion (2 tools)

21 categories, 118 units with DECIMAL128 precision factors.

| Tool | Input | Output | Example |
|:---|:---|:---|:---|
| `convert` | Value, fromUnit, toUnit, category (e.g., "LENGTH") | Converted value | `convert("1", "km", "mi", "LENGTH")` → `"0.621..."` |
| `convertAutoDetect` | Value, fromUnit, toUnit | Converted value | Auto-detect category (less precise) |

**Categories**: DATA_STORAGE, LENGTH, MASS, VOLUME, TEMPERATURE, TIME, SPEED, AREA, ENERGY, FORCE, PRESSURE, POWER, DENSITY, FREQUENCY, ANGLE, DATA_RATE, RESISTANCE, CAPACITANCE, INDUCTANCE, VOLTAGE, CURRENT

## Cooking (3 tools)

Culinary unit conversions and temperature.

| Tool | Input | Output | Example |
|:---|:---|:---|:---|
| `convertCookingVolume` | Value, fromUnit (cup/tbsp/tsp/ml/l), toUnit | Converted | `convertCookingVolume("1", "cup", "ml")` → `"236.588"` |
| `convertCookingWeight` | Value, fromUnit (g/kg/oz/lb), toUnit | Converted | `convertCookingWeight("1", "lb", "g")` → `"453.592"` |
| `convertOvenTemperature` | Value, fromUnit (c/f/gasmark), toUnit | Temp in target unit | `convertOvenTemperature("350", "f", "c")` → `"176.67"` |

## Measure Reference (4 tools)

Introspection helpers for the unit system.

| Tool | Input | Output |
|:---|:---|:---|
| `listCategories` | — | JSON array of 21 categories |
| `listUnits` | Category name (e.g., "LENGTH") | JSON array of units in category |
| `getConversionFactor` | fromUnit, toUnit | Numeric factor (e.g., 1 km → X miles) |
| `explainConversion` | fromUnit, toUnit | Human-readable explanation |

## Date & Time (5 tools)

IANA timezone support, no libicu dependency.

| Tool | Input | Output | Notes |
|:---|:---|:---|:---|
| `convertTimezone` | Datetime, fromTimezone, toTimezone | ISO-8601 zoned result | `fromTimezone`, `toTimezone` as IANA IDs (e.g., "UTC", "America/New_York") |
| `formatDateTime` | Datetime, inputFormat, outputFormat, timezone | Formatted string | Keywords: iso, iso-offset, iso-local, epoch, epochmillis, rfc1123; or strftime pattern |
| `currentDateTime` | Timezone, format | Current time in zone/format | Format as above |
| `listTimezones` | Region prefix (e.g., "America", or "all") | JSON array of IANA IDs | For IDE/client autocomplete |
| `dateTimeDifference` | datetime1, datetime2, timezone | `{years, months, days, hours, minutes, seconds, totalSeconds}` | Positive difference |

## Tape Calculator (1 tool)

Retro tape calculator with running totals.

| Tool | Input | Output |
|:---|:---|:---|
| `calculateWithTape` | JSON array of `{op, value}` | Printed tape (multiline) |

**Operations**: `+`, `-`, `*`, `/`, `=` (total), `C` (clear), `T` (subtotal)

## Graphing (3 tools)

Function analysis and root finding.

| Tool | Input | Output | Use Case |
|:---|:---|:---|:---|
| `plotFunction` | Expression, variable, min, max, steps | JSON array of `{x, y}` | Sample function for graphing |
| `solveEquation` | Expression, variable, initialGuess | Root value | Newton-Raphson solver |
| `findRoots` | Expression, variable, min, max | JSON array of roots | Bracketed root finding |

## Networking (13 tools)

IPv4/IPv6, CIDR, VLSM, throughput.

| Tool | Input | Output | Category |
|:---|:---|:---|:---|
| `subnetCalculator` | Address, CIDR prefix | Network, broadcast, mask, hosts, etc. | Subnet info |
| `ipToBinary` | IP address | Binary (dot/colon separated) | Conversion |
| `binaryToIp` | Binary string | IPv4/IPv6 address | Conversion |
| `ipToDecimal` | IP address | Unsigned integer | Conversion |
| `decimalToIp` | Decimal integer, version (4 or 6) | IP address | Conversion |
| `ipInSubnet` | Address, network, CIDR | true/false | Membership test |
| `vlsmSubnets` | Network CIDR, hostCounts (JSON array) | Allocated subnets (JSON) | VLSM allocation |
| `summarizeSubnets` | CIDR blocks (JSON array) | Supernet CIDR | Summarization |
| `expandIpv6` | Compressed IPv6 | Full 8-group form | Expansion |
| `compressIpv6` | IPv6 address | Shortest form with :: | Compression |
| `transferTime` | File size, size unit, bandwidth, bandwidth unit | `{seconds, minutes, hours}` | Data transfer estimate |
| `throughput` | Data size, size unit, time, time unit, output unit | Throughput (Mbps, Gbps, etc.) | Data rate |
| `tcpThroughput` | Bandwidth (Mbps), RTT (ms), window size (KB) | Effective throughput (Mbps) | TCP performance |

## Analog Electronics (14 tools)

Circuit analysis with impedance, resonance, filters.

| Tool | Input | Output | Formula |
|:---|:---|:---|:---|
| `ohmsLaw` | Voltage, Current, Resistance, Power (provide 2, compute 2) | Computed values | V=IR, P=VI |
| `resistorCombination` | CSV resistances, mode (series/parallel) | Total resistance | Series: sum; Parallel: 1/(1/R1+1/R2) |
| `capacitorCombination` | CSV capacitances, mode (series/parallel) | Total capacitance | Dual of resistors |
| `inductorCombination` | CSV inductances, mode (series/parallel) | Total inductance | Dual of resistors |
| `voltageDivider` | Vin, R1, R2 | Vout across R2 | Vout = Vin × R2/(R1+R2) |
| `currentDivider` | Total current, R1, R2 (parallel) | I1, I2 | Current splits inversely to R |
| `rcTimeConstant` | Resistance, capacitance | τ, cutoff frequency | τ=RC, fc=1/(2πRC) |
| `rlTimeConstant` | Resistance, inductance | τ, cutoff frequency | τ=L/R, fc=R/(2πL) |
| `rlcResonance` | R, L, C | Resonant frequency, Q factor, bandwidth | Series RLC |
| `impedance` | R, L, C, frequency | Magnitude, phase (degrees) | Z = √(R² + (XL-XC)²) |
| `decibelConvert` | Value, mode | Conversion result | Modes: powerToDb, voltageToDb, dbToPower, dbToVoltage |
| `filterCutoff` | Resistance, reactive, filterType (lowpass/highpass) | Cutoff frequency | fc = 1/(2πRC) |
| `ledResistor` | Supply voltage, LED forward voltage, forward current | Series resistance | R = (Vs - Vf) / If |
| `wheatstoneBridge` | R1, R2, R3 | R4 (balance resistor) | R4 = R3 × R2 / R1 |

## Digital Electronics (10 tools)

Bit-level operations, ADC/DAC, timers.

| Tool | Input | Output | Category |
|:---|:---|:---|:---|
| `convertBase` | Value, fromBase (2–36), toBase (2–36) | Converted (uppercase) | Base conversion |
| `twosComplement` | Value, bits (1–64), direction (toTwos/fromTwos) | Converted | 2's complement encode/decode |
| `grayCode` | Binary value, direction (toGray/fromGray) | Gray code / binary | Gray code conversion |
| `bitwiseOp` | A, B, operation (AND/OR/XOR/NOT/SHL/SHR) | Result (decimal, binary) | Bitwise operations |
| `adcResolution` | Bit width, Vref | LSB, step count | ADC specs |
| `dacOutput` | Bit width, Vref, digital code | Analog output voltage | DAC conversion |
| `timer555Astable` | R1, R2, C | Frequency, duty cycle (%), period | 555 astable oscillator |
| `timer555Monostable` | R, C | Pulse width | 555 monostable: PW = 1.1 × R × C |
| `frequencyPeriod` | Value, mode (freqToPeriod/periodToFreq) | Converted | f ↔ T reciprocal |
| `nyquistRate` | Bandwidth (Hz) | Minimum sample rate | Fmin = 2 × bandwidth |

---

## Summary

- **87 total tools** across **15 categories**
- **All return `String`** for error embedding
- **Arbitrary precision** for math, finance, conversion
- **Portable** — runs identically across Linux, macOS, Windows
- **Fast** — sub-millisecond latency for most tools

See [API Usage](./API.md) for integration examples.
