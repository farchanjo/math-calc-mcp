# Development Guide

Complete guide for building, testing, and contributing to arithma.

## Building

### Prerequisites

- **Rust 1.94+** (pinned in `rust-toolchain.toml`)
- **Cargo** (comes with Rust)
- **Python 3.9+** (for integration tests)

### Quick Start

```bash
git clone https://github.com/farchanjo/arithma.git
cd arithma
cargo build --release
# Binary: ./target/release/arithma
```

### Build Profiles

```bash
# Native CPU (fastest on this machine)
cargo build --release

# Portable build (targets x86-64-v3: Haswell+, includes AVX2)
RUSTFLAGS="-C target-cpu=x86-64-v3" cargo build --profile release-portable

# Development (with debug symbols, slower)
cargo build

# Run directly
cargo run --release --bin arithma
```

### Minimum Binary Size

The release binary is ~3 MB statically-linked. Optimize further with:

```bash
cargo build --release -C lto=fat -C codegen-units=1 -C strip=symbols
# Result: ~2 MB
```

## Testing

### Unit Tests (349 tests)

```bash
cargo test --lib
```

Runs all internal tests: expression parser, unit registry, BigDecimal helpers, etc.

### Integration Tests (87 tools)

```bash
python3 scripts/test_stdio.py
```

Tests all 87 tools via JSON-RPC stdio protocol. Takes ~0.5 seconds.

### Full Test Suite

```bash
cargo test --all
```

Includes unit + doctests.

### Running Specific Tests

```bash
# Test a specific tool module
cargo test --lib unit_converter::tests

# Run a single test
cargo test --lib convert_length
```

## Linting & Formatting

### Format Check

```bash
cargo fmt --check
```

### Auto-Format

```bash
cargo fmt
```

### Lint (Clippy)

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

Treats all warnings as errors (deny mode).

### Pre-Commit Workflow

```bash
# 1. Format
cargo fmt

# 2. Lint
cargo clippy --all-targets -- -D warnings

# 3. Test
cargo test --lib

# 4. Integration test
python3 scripts/test_stdio.py
```

All must pass before committing.

## Project Structure

```
.
├── Cargo.toml                    # Package metadata, dependencies
├── rust-toolchain.toml           # Rust 1.94+ requirement
├── .cargo/config.toml            # Cargo aliases, target-cpu=native
├── clippy.toml                   # Linter configuration
├── src/
│   ├── main.rs                   # Binary entry, stdio transport
│   ├── lib.rs                    # Library exports
│   ├── server.rs                 # MCP tool registration (#[tool_router])
│   ├── engine/
│   │   ├── mod.rs                # Module documentation
│   │   ├── expression.rs         # Parser + evaluator (f64)
│   │   ├── expression_exact.rs   # High-precision evaluator (f128)
│   │   ├── unit_registry.rs      # 21 categories, 118 units
│   │   └── bigdecimal_ext.rs     # DECIMAL128 helpers
│   └── tools/                    # 15 tool modules
│       ├── mod.rs
│       ├── basic.rs              # add, subtract, multiply, etc.
│       ├── scientific.rs         # sin, cos, tan, log, sqrt, etc.
│       ├── vector.rs             # sum, dot product, scale, magnitude
│       ├── financial.rs          # compound interest, loans, etc.
│       ├── calculus.rs           # derivative, integral, tangent
│       ├── unit_converter.rs     # convert, convertAutoDetect
│       ├── cooking.rs            # Cooking-specific conversions
│       ├── measure_reference.rs  # listCategories, listUnits, etc.
│       ├── datetime.rs           # Timezone-aware date/time
│       ├── printing.rs           # Tape calculator
│       ├── graphing.rs           # Plot, solve, findRoots
│       ├── network.rs            # IPv4/IPv6, CIDR, VLSM
│       ├── analog_electronics.rs # Ohm's law, impedance, filters
│       ├── digital_electronics.rs # Bases, gates, timers, ADC/DAC
│       └── programmable.rs       # Expression evaluation with vars
├── scripts/
│   └── test_stdio.py             # Full integration test (87 tools)
├── docs/
│   ├── INDEX.md                  # Documentation index
│   ├── ARCHITECTURE.md           # System design
│   ├── TOOLS.md                  # 87-tool reference
│   ├── DEVELOPMENT.md            # This file
│   └── API.md                    # MCP integration guide
└── target/release/arithma        # Final binary
```

## Code Conventions

### Naming

- **Crate**: `arithma` (binary), `math_calc` (library)
- **Modules**: lowercase (e.g., `basic`, `unit_registry`)
- **Functions**: snake_case (e.g., `convert_units`)
- **Types**: PascalCase (e.g., `UnitCategory`, `UnitError`)
- **Constants**: SCREAMING_SNAKE_CASE (e.g., `FACTOR_SCALE`)

### Function Signatures

All tool functions follow the MCP pattern:

```rust
pub fn tool_name(param1: String, param2: i32) -> String {
    // Return result or error as String
    format!("{}", result)
    // or:
    format!("Error: {}", error_msg)
}
```

### Error Handling

- Use `Result<T, E>` internally
- Convert to `String` for MCP boundary
- Never panic; return `"Error: ..."` instead

### Module Documentation

Every module has a doc comment:

```rust
//! Brief description.
//!
//! Longer explanation of purpose, invariants, and design.
```

### Test Organization

Tests live inline with code:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name() {
        assert_eq!(function(), expected);
    }
}
```

## Dependency Management

### Current Versions

See `Cargo.toml` for exact versions. Key dependencies:

- **rmcp** — Rust MCP SDK
- **tokio** — Async runtime
- **bigdecimal** — Arbitrary precision
- **astro-float** — 128-bit transcendentals
- **jiff** — Timezone support
- **wide** — Portable SIMD

### Adding Dependencies

Before adding a dependency:

1. **Verify it's pure Rust** (no C FFI)
2. **Check licensing** (must be compatible with Apache-2.0)
3. **Test compilation** on macOS, Linux, Windows
4. **Update documentation** if it changes architecture

```bash
cargo add <crate> --vers ^X.Y.Z
```

## Debugging

### Logging

The server logs to stderr with `tracing`. Control with `RUST_LOG`:

```bash
RUST_LOG=debug ./target/release/arithma
RUST_LOG=math_calc=trace ./target/release/arithma
```

### Debugging in IDE

JetBrains Rust plugin supports native debugging. Set breakpoints and run:

```bash
cargo run --bin arithma
```

### Manual Testing

Test a tool manually via JSON-RPC:

```bash
(echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}';
 echo '{"jsonrpc":"2.0","method":"notifications/initialized"}';
 echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"add","arguments":{"first":"0.1","second":"0.2"}}}';
 sleep 0.1) | ./target/release/arithma
```

## Contributing

### Workflow

1. **Fork** the repository
2. **Create a branch** off `main`: `git checkout -b fix/description`
3. **Make changes** (format, lint, test locally)
4. **Commit** with Angular format: `fix(scope): description`
5. **Push** and open a PR

### Commit Messages

Use [Angular Commit Format](https://github.com/angular/angular/blob/main/CONTRIBUTING.md#commit):

```
<type>(<scope>): <subject>

<body>

<footer>
```

**Types**: `feat`, `fix`, `refactor`, `test`, `docs`, `chore`, `perf`, `style`

**Example**:
```
fix(unit_converter): handle temperature precision loss

Use DECIMAL128 context for Celsius pivot to prevent rounding errors
in Fahrenheit-to-Kelvin conversions.

Fixes #42
```

### What Gets Reviewed

- ✅ Code formatting (`cargo fmt`)
- ✅ Lint passes (`cargo clippy`)
- ✅ All tests pass
- ✅ No panics (use `Result` instead)
- ✅ Documentation updated
- ✅ English (en-US) only

### Performance Considerations

arithma prioritizes **correctness over speed**, but:

- Use BigDecimal only where precision matters
- Cache lookup tables in `LazyLock` (not recreated per call)
- Avoid allocations in tight loops
- Profile with `cargo bench` before optimizing

---

See [Architecture](./ARCHITECTURE.md) for system design details.
