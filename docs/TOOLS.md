# Tools Catalog — 173 tools, 23 categories

Every tool is a stateless function whose response is a compact, line-oriented envelope built by `src/mcp/message/builder.rs`. Numeric parameters are passed as **strings** to preserve arbitrary precision.

> **Total**: 173 tools — original 87 plus 86 across 8 new domains (statistics, combinatorics, geometry, complex numbers, crypto/encoding, matrices, physics, chemistry).

## Response format

| Shape | Layout | Example |
|:---|:---|:---|
| Scalar success | `TOOL: OK \| RESULT: value` | `ADD: OK \| RESULT: 0.3` |
| Multi-field success | `TOOL: OK \| KEY: v \| KEY: v \| …` | `OHMS_LAW: OK \| VOLTAGE: 12 \| CURRENT: 3 \| RESISTANCE: 4 \| POWER: 36` |
| Tabular success (block) | `TOOL: OK\n<fields>\nROW_N: k=v \| k=v` | Used by `amortizationSchedule`, `plotFunction`, `vlsmSubnets`. |
| Error | `TOOL: ERROR\nREASON: [CODE] text\n[DETAIL: k=v]` | `DIVIDE: ERROR\nREASON: [DIVISION_BY_ZERO] cannot divide by zero` |

Tool names in responses use `SCREAMING_SNAKE_CASE` (e.g. `SUBNET_CALCULATOR`, `EVALUATE_WITH_VARIABLES`). Values containing newlines are escaped as `\n`.

**Error codes**: `DOMAIN_ERROR`, `OUT_OF_RANGE`, `DIVISION_BY_ZERO`, `PARSE_ERROR`, `INVALID_INPUT`, `UNKNOWN_VARIABLE`, `UNKNOWN_FUNCTION`, `OVERFLOW`, `NOT_IMPLEMENTED`.

## Category index

| # | Category | Count | Jump |
|:-:|:---|:-:|:---|
| 1 | Basic math | 7 | [↓](#basic-math-7) |
| 2 | Scientific | 7 | [↓](#scientific-7) |
| 3 | Expression engine | 4 | [↓](#expression-engine-4) |
| 4 | Vectors & arrays | 4 | [↓](#vectors--arrays-4) |
| 5 | Finance | 6 | [↓](#finance-6) |
| 6 | Calculus | 4 | [↓](#calculus-4) |
| 7 | Unit conversion | 2 | [↓](#unit-conversion-2) |
| 8 | Cooking | 3 | [↓](#cooking-3) |
| 9 | Measure reference | 4 | [↓](#measure-reference-4) |
| 10 | Date & time | 5 | [↓](#date--time-5) |
| 11 | Tape calculator | 1 | [↓](#tape-calculator-1) |
| 12 | Graphing & roots | 3 | [↓](#graphing--roots-3) |
| 13 | Networking | 13 | [↓](#networking-13) |
| 14 | Analog electronics | 14 | [↓](#analog-electronics-14) |
| 15 | Digital electronics | 10 | [↓](#digital-electronics-10) |
| 16 | Statistics | 16 | [↓](#statistics-16) |
| 17 | Combinatorics & number theory | 7 | [↓](#combinatorics--number-theory-7) |
| 18 | Geometry | 12 | [↓](#geometry-12) |
| 19 | Complex numbers | 10 | [↓](#complex-numbers-10) |
| 20 | Crypto & encoding | 10 | [↓](#crypto--encoding-10) |
| 21 | Matrices | 10 | [↓](#matrices-10) |
| 22 | Physics | 12 | [↓](#physics-12) |
| 23 | Chemistry | 9 | [↓](#chemistry-9) |

---

## Basic math (7)

Arbitrary-precision arithmetic via `BigDecimal`. All return `TOOL: OK | RESULT: <value>`.

| Tool | Inputs | Example response |
|:---|:---|:---|
| `add` | `first`, `second` | `ADD: OK \| RESULT: 0.3` |
| `subtract` | `first`, `second` | `SUBTRACT: OK \| RESULT: 0.1` |
| `multiply` | `first`, `second` | `MULTIPLY: OK \| RESULT: 6` |
| `divide` | `first`, `second` | `DIVIDE: OK \| RESULT: 3.33333333333333333333` |
| `power` | `base`, `exponent` | `POWER: OK \| RESULT: 1024` |
| `modulo` | `first`, `second` | `MODULO: OK \| RESULT: 1` |
| `abs` | `value` | `ABS: OK \| RESULT: 42` |

## Scientific (7)

Trigonometry and transcendentals — exact at notable angles, 128-bit elsewhere.

| Tool | Inputs | Example response |
|:---|:---|:---|
| `sin` | `degrees` | `SIN: OK \| RESULT: 0.5` |
| `cos` | `degrees` | `COS: OK \| RESULT: 0.5` |
| `tan` | `degrees` | `TAN: OK \| RESULT: 1` |
| `sqrt` | `number` | `SQRT: OK \| RESULT: 1.4142135623730951` |
| `log` | `number` | `LOG: OK \| RESULT: 1.0` |
| `log10` | `number` | `LOG10: OK \| RESULT: 3.0` |
| `factorial` | `num` (0–20) | `FACTORIAL: OK \| RESULT: 120` |

## Expression engine (4)

Parse-and-evaluate any expression with full operator support. `variables` is a **string containing JSON**.

| Tool | Inputs | Response shape |
|:---|:---|:---|
| `evaluate` | `expression` | `EVALUATE: OK \| RESULT: <f64>` |
| `evaluateWithVariables` | `expression`, `variables` | `EVALUATE_WITH_VARIABLES: OK \| RESULT: <f64>` |
| `evaluateExact` | `expression` | `EVALUATE_EXACT: OK \| RESULT: <128-bit>` |
| `evaluateExactWithVariables` | `expression`, `variables` | `EVALUATE_EXACT_WITH_VARIABLES: OK \| RESULT: <128-bit>` |

**Operators**: `+ - * / ^ %` and parentheses.

**Constants**: `pi`, `e`, `tau`, `phi` (recognised when used as a bare identifier; variable bindings shadow the built-in).

**Functions** (both `evaluate` and `evaluateExact` share the same surface):

- Trigonometric (degrees in, degrees out for inverses): `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2(y, x)`.
- Hyperbolic (radians): `sinh`, `cosh`, `tanh`, `asinh`, `acosh`, `atanh`.
- Exponential / log: `exp`, `log` (alias `ln`), `log10`, `log2`.
- Roots and rounding: `sqrt`, `cbrt`, `abs`, `ceil`, `floor`, `round`, `trunc`, `sign`.
- Multi-arg: `min(a, b, …)`, `max(a, b, …)`, `mod(a, b)`, `hypot(x, y)`, `pow(base, exp)`, `gcd(a, b)`, `lcm(a, b)`.
- Combinatorial: `factorial(n)`.

Common errors: `UNKNOWN_VARIABLE` (e.g. `DETAIL: name=foo`), `UNKNOWN_FUNCTION`, `DIVISION_BY_ZERO`, `PARSE_ERROR`, `DOMAIN_ERROR` (e.g. `asin(2)`, `atanh(±1)`, arity mismatch).

## Vectors & arrays (4)

SIMD-accelerated via `wide`. Arrays are comma-separated strings.

| Tool | Inputs | Example response |
|:---|:---|:---|
| `sumArray` | CSV numbers | `SUM_ARRAY: OK \| RESULT: 15` |
| `dotProduct` | two CSV arrays | `DOT_PRODUCT: OK \| RESULT: 32` |
| `scaleArray` | CSV array, scalar | `SCALE_ARRAY: OK \| RESULT: 2,4,6` |
| `magnitudeArray` | CSV array | `MAGNITUDE_ARRAY: OK \| RESULT: 5` |

## Finance (6)

Time-value-of-money calculations with DECIMAL128 precision.

| Tool | Inputs | Response |
|:---|:---|:---|
| `compoundInterest` | principal, annualRate (%), years, compoundsPerYear | `COMPOUND_INTEREST: OK \| RESULT: <fv>` |
| `loanPayment` | principal, annualRate (%), years | `LOAN_PAYMENT: OK \| RESULT: <monthly>` |
| `presentValue` | futureValue, rate (%), years | `PRESENT_VALUE: OK \| RESULT: <pv>` |
| `futureValueAnnuity` | payment, rate (%), years | `FUTURE_VALUE_ANNUITY: OK \| RESULT: <fv>` |
| `returnOnInvestment` | gain, cost | `RETURN_ON_INVESTMENT: OK \| RESULT: <roi%>` |
| `amortizationSchedule` | principal, annualRate (%), years | Block layout with `MONTHLY_PAYMENT`, `TOTAL_INTEREST`, `TOTAL_PAID`, `MONTHS`, then `ROW_N: month=… \| payment=… \| principal=… \| interest=… \| balance=…` |

## Calculus (4)

Numerical calculus on parsed expressions.

| Tool | Inputs | Output | Method |
|:---|:---|:---|:---|
| `derivative` | expression, variable, point | `DERIVATIVE: OK \| RESULT: <f'(x)>` | Five-point central difference |
| `nthDerivative` | expression, variable, point, order (1–10) | `NTH_DERIVATIVE: OK \| RESULT: …` | Repeated finite differences |
| `definiteIntegral` | expression, variable, lower, upper | `DEFINITE_INTEGRAL: OK \| RESULT: …` | Composite Simpson's (10 000 intervals) |
| `tangentLine` | expression, variable, point | `TANGENT_LINE: OK \| SLOPE: … \| Y_INTERCEPT: … \| EQUATION: …` | Derivative + point-slope |

## Unit conversion (2)

21 categories, 118 units.

| Tool | Inputs | Response |
|:---|:---|:---|
| `convert` | value, fromUnit, toUnit, category | `CONVERT: OK \| RESULT: <34-digit value>` |
| `convertAutoDetect` | value, fromUnit, toUnit | `CONVERT_AUTO_DETECT: OK \| RESULT: …` |

**Categories**: `DATA_STORAGE`, `LENGTH`, `MASS`, `VOLUME`, `TEMPERATURE`, `TIME`, `SPEED`, `AREA`, `ENERGY`, `FORCE`, `PRESSURE`, `POWER`, `DENSITY`, `FREQUENCY`, `ANGLE`, `DATA_RATE`, `RESISTANCE`, `CAPACITANCE`, `INDUCTANCE`, `VOLTAGE`, `CURRENT`.

Unknown units raise `INVALID_INPUT` with `DETAIL: unit=<name>`.

## Cooking (3)

Kitchen-scale conversions.

| Tool | Units | Example response |
|:---|:---|:---|
| `convertCookingVolume` | `cup`, `tbsp`, `tsp`, `ml`, `l` | `CONVERT_COOKING_VOLUME: OK \| RESULT: 236.588…` |
| `convertCookingWeight` | `g`, `kg`, `oz`, `lb` | `CONVERT_COOKING_WEIGHT: OK \| RESULT: 453.592…` |
| `convertOvenTemperature` | `c`, `f`, `gasmark` | `CONVERT_OVEN_TEMPERATURE: OK \| RESULT: 176.67` |

## Measure reference (4)

Introspection helpers for the unit system.

| Tool | Inputs | Response |
|:---|:---|:---|
| `listCategories` | — | `LIST_CATEGORIES: OK \| COUNT: 21 \| VALUES: DATA_STORAGE,LENGTH,…,CURRENT` |
| `listUnits` | category | `LIST_UNITS: OK \| CATEGORY: LENGTH \| COUNT: N \| VALUES: m,km,mi,…` |
| `getConversionFactor` | fromUnit, toUnit | `GET_CONVERSION_FACTOR: OK \| RESULT: <factor>` |
| `explainConversion` | fromUnit, toUnit | `EXPLAIN_CONVERSION: OK \| RESULT: "<human readable>"` |

## Date & time (5)

IANA-aware; no `libicu`.

| Tool | Inputs | Response |
|:---|:---|:---|
| `convertTimezone` | datetime, fromTimezone, toTimezone | `CONVERT_TIMEZONE: OK \| RESULT: <ISO-8601 zoned>` |
| `formatDateTime` | datetime, inputFormat, outputFormat, timezone | `FORMAT_DATE_TIME: OK \| RESULT: <formatted>` |
| `currentDateTime` | timezone, format | `CURRENT_DATE_TIME: OK \| RESULT: <now>` |
| `listTimezones` | region prefix (`"America"`, `"Europe"`, `"all"`) | `LIST_TIMEZONES: OK \| COUNT: N \| VALUES: <IANA,ids,…>` |
| `dateTimeDifference` | datetime1, datetime2, timezone | `DATE_TIME_DIFFERENCE: OK \| YEARS: … \| MONTHS: … \| DAYS: … \| HOURS: … \| MINUTES: … \| SECONDS: … \| TOTAL_SECONDS: …` |

**Format keywords**: `iso`, `iso-offset`, `iso-local`, `epoch`, `epochmillis`, `rfc1123` — or any strftime pattern.

## Tape calculator (1)

| Tool | Inputs | Response |
|:---|:---|:---|
| `calculateWithTape` | JSON array of `{op, value}` | Block layout. Each tape line is one `ROW_N: op=… \| value=… \| running=…` row, ending with a `TOTAL` field. |

**Ops**: `+`, `-`, `*`, `/`, `=` (total), `C` (clear), `T` (subtotal).

## Graphing & roots (3)

| Tool | Inputs | Response |
|:---|:---|:---|
| `plotFunction` | expression, variable, min, max, steps | Block layout: `COUNT: N` + `ROW_N: x=… \| y=…`. |
| `solveEquation` | expression, variable, initialGuess | `SOLVE_EQUATION: OK \| RESULT: <root>` (or `NO_ROOT` status on failure). |
| `findRoots` | expression, variable, min, max | Block layout: `COUNT: N` + `ROW_N: x=…`. |

## Networking (13)

IPv4/IPv6, CIDR, VLSM, throughput.

| Tool | Response |
|:---|:---|
| `subnetCalculator` | `SUBNET_CALCULATOR: OK \| NETWORK: … \| BROADCAST: … \| MASK: … \| WILDCARD: … \| FIRST_HOST: … \| LAST_HOST: … \| USABLE_HOSTS: … \| IP_CLASS: …` |
| `ipToBinary` | `IP_TO_BINARY: OK \| RESULT: <dotted/colon binary>` |
| `binaryToIp` | `BINARY_TO_IP: OK \| RESULT: <ip>` |
| `ipToDecimal` | `IP_TO_DECIMAL: OK \| RESULT: <unsigned>` |
| `decimalToIp` | `DECIMAL_TO_IP: OK \| RESULT: <ip>` |
| `ipInSubnet` | `IP_IN_SUBNET: OK \| RESULT: true \| false` |
| `vlsmSubnets` | Block layout — one `ROW_N: network=… \| cidr=… \| first=… \| last=… \| hosts=…` per allocation. |
| `summarizeSubnets` | `SUMMARIZE_SUBNETS: OK \| RESULT: <supernet/cidr>` |
| `expandIpv6` | `EXPAND_IPV6: OK \| RESULT: <8-group>` |
| `compressIpv6` | `COMPRESS_IPV6: OK \| RESULT: <shortest>` |
| `transferTime` | `TRANSFER_TIME: OK \| SECONDS: … \| MINUTES: … \| HOURS: …` |
| `throughput` | `THROUGHPUT: OK \| RESULT: <rate> \| UNIT: <Mbps \| Gbps \| …>` |
| `tcpThroughput` | `TCP_THROUGHPUT: OK \| RESULT: <Mbps>` |

## Analog electronics (14)

Circuit analysis.

| Tool | Response / formula |
|:---|:---|
| `ohmsLaw` | `OHMS_LAW: OK \| VOLTAGE: … \| CURRENT: … \| RESISTANCE: … \| POWER: …` — supply any 2, get all 4. |
| `resistorCombination` | `RESISTOR_COMBINATION: OK \| RESULT: <Ω>` — series sum or `1/Σ(1/Rᵢ)`. |
| `capacitorCombination` | Dual of resistors. |
| `inductorCombination` | Dual of resistors. |
| `voltageDivider` | `VOLTAGE_DIVIDER: OK \| RESULT: <Vout>` — `Vin · R2 / (R1+R2)`. |
| `currentDivider` | `CURRENT_DIVIDER: OK \| I1: … \| I2: …`. |
| `rcTimeConstant` | `RC_TIME_CONSTANT: OK \| TAU: … \| CUTOFF: …`. |
| `rlTimeConstant` | `RL_TIME_CONSTANT: OK \| TAU: … \| CUTOFF: …`. |
| `rlcResonance` | `RLC_RESONANCE: OK \| FREQUENCY: … \| Q_FACTOR: … \| BANDWIDTH: …`. |
| `impedance` | `IMPEDANCE: OK \| MAGNITUDE: … \| PHASE_DEG: …`. |
| `decibelConvert` | `DECIBEL_CONVERT: OK \| RESULT: …` — modes `powerToDb`, `voltageToDb`, `dbToPower`, `dbToVoltage`. |
| `filterCutoff` | `FILTER_CUTOFF: OK \| RESULT: <Hz>`. |
| `ledResistor` | `LED_RESISTOR: OK \| RESULT: <Ω>` — `R = (Vs − Vf) / If`. |
| `wheatstoneBridge` | `WHEATSTONE_BRIDGE: OK \| RESULT: <R4>` — `R4 = R3·R2 / R1`. |

## Digital electronics (10)

Bit-level operations, ADC/DAC, timers.

| Tool | Response |
|:---|:---|
| `convertBase` | `CONVERT_BASE: OK \| RESULT: <uppercase digits>` — bases 2–36. |
| `twosComplement` | `TWOS_COMPLEMENT: OK \| RESULT: …` — `bits ∈ [1, 64]`. |
| `grayCode` | `GRAY_CODE: OK \| RESULT: …` — `toGray` / `fromGray`. |
| `bitwiseOp` | `BITWISE_OP: OK \| DECIMAL: … \| BINARY: …` — `AND`, `OR`, `XOR`, `NOT`, `SHL`, `SHR`. |
| `adcResolution` | `ADC_RESOLUTION: OK \| LSB: … \| STEPS: …`. |
| `dacOutput` | `DAC_OUTPUT: OK \| RESULT: <V>`. |
| `timer555Astable` | `TIMER_555_ASTABLE: OK \| FREQUENCY: … \| DUTY_CYCLE: … \| PERIOD: …`. |
| `timer555Monostable` | `TIMER_555_MONOSTABLE: OK \| RESULT: <pulse width>` — `PW = 1.1·R·C`. |
| `frequencyPeriod` | `FREQUENCY_PERIOD: OK \| RESULT: …` — `freqToPeriod` / `periodToFreq`. |
| `nyquistRate` | `NYQUIST_RATE: OK \| RESULT: <min sample rate>`. |

## Statistics (16)

Descriptive stats, distributions, and regression. Values are comma-separated decimals.

| Tool | Inputs | Response |
|:---|:---|:---|
| `mean` | values | `MEAN: OK \| RESULT: …` |
| `median` | values | `MEDIAN: OK \| RESULT: …` |
| `mode` | values | `MODE: OK \| MODES: v1,v2 \| COUNT: n` — multi-modal output when ties exist |
| `variance` | values, population (bool) | `VARIANCE: OK \| RESULT: …` — sample (`n-1`) or population (`n`) |
| `stdDev` | values, population | `STDDEV: OK \| RESULT: …` |
| `percentile` | values, p (0–100) | Linear-interpolated (R-7 / Excel definition) |
| `quartile` | values, q (1–3) | `QUARTILE: OK \| Q: n \| VALUE: …` |
| `iqr` | values | `IQR: OK \| Q1: … \| Q3: … \| IQR: …` |
| `correlation` | xValues, yValues | Pearson coefficient |
| `covariance` | xValues, yValues, population | |
| `linearRegression` | xValues, yValues | `LINEAR_REGRESSION: OK \| SLOPE \| INTERCEPT \| R \| R_SQUARED` |
| `normalPdf` | x, mean, stdDev | `NORMAL_PDF: OK \| RESULT: …` |
| `normalCdf` | x, mean, stdDev | erf-approximated (max error ~1.5e-7) |
| `tTestOneSample` | values, hypothesizedMean | `T_TEST: OK \| T \| DF \| MEAN \| SE` |
| `binomialPmf` | n, k, p | Capped at n=1000 for numerical safety |
| `confidenceInterval` | values, confidenceLevel (0–1) | `CONFIDENCE_INTERVAL: OK \| MEAN \| LOWER \| UPPER \| MARGIN` — normal (z-score) |

## Combinatorics & number theory (7)

Exact arbitrary-precision integer arithmetic via `num-bigint`.

| Tool | Inputs | Example |
|:---|:---|:---|
| `combination` | n, k | `COMBINATION: OK \| RESULT: 126410606437752` (C(50,25)) |
| `permutation` | n, k | `PERMUTATION: OK \| RESULT: 720` (P(10,3)) |
| `fibonacci` | n | `FIBONACCI: OK \| RESULT: 354224848179261915075` (F(100)); capped at n=50000 |
| `isPrime` | n | `IS_PRIME: OK \| N: … \| IS_PRIME: true \| false` |
| `nextPrime` | n | Smallest prime strictly greater than n |
| `primeFactors` | n | `PRIME_FACTORS: OK \| N: … \| FACTORS: 2,2,3 \| COUNT: 3` — n ≤ 10^12 |
| `eulerTotient` | n | Count of integers in [1, n] coprime to n |

## Geometry (12)

Areas, volumes, distances for common shapes. Radii/lengths must be positive.

| Tool | Inputs | Response |
|:---|:---|:---|
| `circleArea` | radius | `CIRCLE_AREA: OK \| RESULT: π·r²` |
| `circlePerimeter` | radius | `CIRCLE_PERIMETER: OK \| RESULT: 2π·r` |
| `sphereVolume` | radius | `4π·r³/3` |
| `sphereArea` | radius | `4π·r²` |
| `triangleArea` | sides=`"a,b,c"` | Heron's formula; inequality violation errors |
| `polygonArea` | coordinates=`"x1,y1,x2,y2,…"` | Shoelace formula; `POLYGON_AREA: OK \| AREA: … \| VERTICES: n` |
| `coneVolume` | radius, height | `π·r²·h/3` |
| `cylinderVolume` | radius, height | `π·r²·h` |
| `distance2D` | p1=`"x,y"`, p2 | `DISTANCE_2D: OK \| RESULT: …` |
| `distance3D` | p1=`"x,y,z"`, p2 | `DISTANCE_3D: OK \| RESULT: …` |
| `regularPolygon` | sides (≥3), sideLength | `REGULAR_POLYGON: OK \| AREA \| PERIMETER \| APOTHEM \| CIRCUMRADIUS` |
| `pointToLineDistance` | point, lineP1, lineP2 (2D) | Perpendicular distance |

## Complex numbers (10)

Rectangular form `real,imag` CSV. Polar conversions use **degrees** to match trig conventions.

| Tool | Inputs | Response |
|:---|:---|:---|
| `complexAdd` | a, b | `COMPLEX_ADD: OK \| REAL \| IMAG` |
| `complexMult` | a, b | `COMPLEX_MULT: OK \| REAL \| IMAG` |
| `complexDiv` | a, b | `COMPLEX_DIV: OK \| REAL \| IMAG` — errors with `DIVISION_BY_ZERO` on complex zero |
| `complexConjugate` | z | Flips sign of imaginary part |
| `complexPower` | z, exponent | De Moivre: `r^n·(cos nθ + i sin nθ)` |
| `complexMagnitude` | z | Scalar — `\|z\| = √(re²+im²)` |
| `complexPhase` | z | Degrees, range (-180, 180] |
| `polarToRect` | magnitude, angleDegrees | `POLAR_TO_RECT: OK \| REAL \| IMAG` |
| `rectToPolar` | z | `RECT_TO_POLAR: OK \| MAGNITUDE \| ANGLE_DEG` |
| `complexSqrt` | z | Principal square root |

## Crypto & encoding (10)

Pure-Rust `RustCrypto` + helpers. UTF-8 in, UTF-8 / hex out.

| Tool | Inputs | Response |
|:---|:---|:---|
| `hashMd5` | input | Hex digest |
| `hashSha1` | input | Hex digest |
| `hashSha256` | input | Hex digest |
| `hashSha512` | input | Hex digest |
| `base64Encode` | input | Standard alphabet |
| `base64Decode` | input | Errors on invalid b64 or non-UTF-8 bytes |
| `urlEncode` | input | Percent-encoded |
| `urlDecode` | input | |
| `hexEncode` | input | Lowercase hex |
| `crc32` | input | `CRC32: OK \| DECIMAL \| HEX` (IEEE CRC-32) |

## Matrices (10)

Matrices are passed as row-major strings: rows separated by `;`, cells by `,` (e.g. `"1,2;3,4"`).

| Tool | Inputs | Response |
|:---|:---|:---|
| `matrixAdd` | a, b | `MATRIX_ADD: OK \| DIM \| MATRIX: <row-major>` |
| `matrixMultiply` | a, b | Requires `a.cols == b.rows` |
| `matrixTranspose` | a | Swaps rows and columns |
| `matrixDeterminant` | a (square) | Partial-pivoted Gaussian elimination |
| `matrixInverse` | a (invertible) | Gauss-Jordan; singular → `DOMAIN_ERROR` |
| `matrixTrace` | a (square) | Sum of diagonal entries |
| `matrixRank` | a | Gauss-Jordan with ε=1e-9 pivot threshold |
| `matrixEigenvalues2x2` | 2x2 matrix | `KIND: real \| LAMBDA1 \| LAMBDA2` — or `KIND: complex` with `re,im` pairs |
| `crossProduct` | a=`"x,y,z"`, b | 3D cross product |
| `gaussianElimination` | augmented `[A\|b]` (N rows × N+1 cols) | `GAUSSIAN_ELIMINATION: OK \| N \| SOLUTION` |

## Physics (12)

Classical physics formulas. SI units except where noted.

| Tool | Inputs | Response |
|:---|:---|:---|
| `kinematics` | initialVelocity, acceleration, time | `FINAL_VELOCITY \| DISPLACEMENT` |
| `projectileMotion` | speed, angleDegrees, gravity | `RANGE \| PEAK_HEIGHT \| TIME_OF_FLIGHT` (no air resistance) |
| `newtonsForce` | mass, acceleration | `F = m·a` |
| `gravitationalForce` | m1, m2, distance | `F = G·m1·m2/r²` (`G = 6.674e-11`) |
| `dopplerEffect` | sourceFreq, soundSpeed, sourceVelocity, observerVelocity | Classical (non-relativistic) |
| `waveLength` | frequency, waveSpeed | `λ = waveSpeed/f` |
| `planckEnergy` | frequency | `E = h·f` (`h = 6.626e-34`) |
| `idealGasLaw` | pressure, volume, moles, temperature, solveFor (P\|V\|n\|T) | Solves for the unknown |
| `heatTransfer` | thermalConductivity, area, deltaTemp, thickness | Fourier's law `Q = kAΔT/L` |
| `stefanBoltzmann` | emissivity (0–1), area, temperatureK | `P = σεAT⁴` |
| `escapeVelocity` | mass, radius | `v = √(2GM/r)` |
| `orbitalVelocity` | mass, radius | `v = √(GM/r)` |

## Chemistry (9)

Stoichiometry, pH, concentration, decay.

| Tool | Inputs | Response |
|:---|:---|:---|
| `molarMass` | formula (e.g. `H2O`, `Ca(OH)2`, `Fe2(SO4)3`) | `MOLAR_MASS: OK \| FORMULA \| MOLAR_MASS_G_MOL \| BREAKDOWN` — nested parens supported |
| `ph` | hConcentration (mol/L) | `pH = -log10([H+])` |
| `poh` | ohConcentration | `pOH = -log10([OH-])` |
| `molarity` | moles, volumeLitres | mol/L |
| `molality` | moles, kilogramsSolvent | mol/kg |
| `hendersonHasselbalch` | pka, conjugateBase, weakAcid | `pH = pKa + log10([A⁻]/[HA])` |
| `halfLife` | decayConstant | `t½ = ln(2)/λ` |
| `decayConstant` | halfLife | `λ = ln(2)/t½` |
| `idealGasMoles` | pressurePa, volumeM3, temperatureK | `n = PV/(RT)` |

---

## Summary

- **173 tools** across **23 categories** (original 87 plus 86 across 8 new domains).
- Every response is a single string in arithma's line-oriented envelope: `TOOL: OK | …` on success, `TOOL: ERROR\nREASON: [CODE] …` on failure.
- Arbitrary precision where it matters (arithmetic, finance, unit conversion, combinatorics).
- Stateless — safe to fan out concurrent calls.
- Pure Rust — zero C dependencies, single static binary.

See [API.md](./API.md) for wire-level JSON-RPC examples.
