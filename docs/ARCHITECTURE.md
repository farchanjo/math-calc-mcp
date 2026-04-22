# Architecture Overview

## System Design

```
MCP Client (Claude, Cursor, etc.)
         ↓ (JSON-RPC over stdio)
    arithma Binary
         ↓
    MCP Server (rmcp SDK)
         ↓
    Tool Router
         ↓
  15 Tool Modules
         ↓
   Math Engine
```

## High-Level Layers

### 1. **Transport Layer** (`main.rs`)
- Tokio multi-threaded async runtime
- Stdio I/O transport (JSON-RPC 2.0)
- Error handling via `Result<T>` → `"Error: ..."` strings

### 2. **MCP Server** (`server.rs`)
- Tool registration via `#[tool_router]` macro
- Parameter deserialization (JSON → Rust types)
- Schema generation for IDE type hints
- 87 tools organized in one block

### 3. **Tool Modules** (`tools/`)
- **15 categories**: basic, scientific, vector, financial, calculus, unit_converter, cooking, measure_reference, datetime, printing, graphing, network, analog_electronics, digital_electronics, programmable
- Each module exports a public function signature matching the MCP spec
- All return `String` (errors embedded as `"Error: ..."`)

### 4. **Engine** (`engine/`)

#### `expression.rs` — Expression Evaluator
- Recursive-descent parser for arbitrary math expressions
- Supports: `+`, `-`, `*`, `/`, `^`, `%`, parens
- 9 built-in functions: `sin`, `cos`, `tan`, `log`, `log10`, `sqrt`, `abs`, `ceil`, `floor`
- Operates on `f64` for speed; `f128` path exists for exact mode

#### `unit_registry.rs` — Unit System
- 21 categories, 118 unit definitions
- Conversion factors stored as `BigDecimal` (34-digit precision)
- Temperature uses formula routing (Celsius pivot)
- Gas mark uses fixed lookup table
- Errors match standard naming conventions

#### `bigdecimal_ext.rs` — Helpers
- DECIMAL128 context: 34 significant digits, `HALF_UP` rounding
- `BigDecimal` → plain string formatting (no scientific notation)
- Scale management for division results

#### `expression_exact.rs` — Exact Evaluator
- 128-bit precision via `astro-float`
- Same syntax as `expression.rs`, high-precision output
- Handles `evaluateExact` and `evaluateExactWithVariables`

## Module Dependencies

```
main.rs
  └─ server.rs
      ├─ rmcp (MCP SDK)
      ├─ tokio (async runtime)
      └─ tools/ (all 15 modules)
          └─ engine/
              ├─ expression.rs
              ├─ expression_exact.rs
              ├─ unit_registry.rs
              └─ bigdecimal_ext.rs

engine/
  ├─ bigdecimal (arbitrary precision)
  ├─ astro-float (128-bit transcendentals)
  ├─ jiff (IANA timezones)
  └─ wide (SIMD)
```

## Key Design Decisions

### 1. **String Returns for All Tools**
- MCP tools return `String`, not typed results
- Errors embedded as `"Error: message"` (no exceptions cross MCP boundary)
- Simplifies error handling and debugging

### 2. **BigDecimal Everywhere**
- All arithmetic uses arbitrary precision
- Avoids floating-point drift (e.g., `0.1 + 0.2 = 0.3` exactly)
- DECIMAL128 context ensures consistent rounding

### 3. **No State**
- Server is stateless — each tool call is independent
- No session management, caching, or side effects
- Safe for concurrent tool calls

### 4. **Portable SIMD**
- `wide` crate auto-dispatches based on CPU features
- No manual CPU detection needed
- Runs correctly on SSE2-only (fallback) through AVX-512

## Tool Invocation Flow

1. **Client** sends JSON-RPC `tools/call`
2. **Server** receives, deserializes params
3. **Router** dispatches to matching tool function
4. **Tool** calls engine helpers (expression, units, etc.)
5. **Engine** computes result using BigDecimal/astro-float
6. **Tool** formats as `String`
7. **Server** returns JSON-RPC response
8. **Client** receives and parses

## Performance Characteristics

| Operation | Latency | Notes |
|:---|:---|:---|
| Startup | ~50ms | Tokio init, LazyLock registration |
| Simple arithmetic | <1ms | BigDecimal overhead minimal |
| Unit conversion | <1ms | HashMap lookup + multiplication |
| Expression evaluation | 2–5ms | Parser overhead, depends on expression complexity |
| Financial (compound interest) | <1ms | BigDecimal context multiply/divide |
| Network (subnet calc) | <1ms | Bitwise ops, HashMap lookup |

## Error Handling

- All public functions return `Result<String, String>` (internally)
- Tool router catches panics and returns `"Error: ..."` strings
- No stack traces leak to MCP clients
- Errors are actionable for downstream LLMs

## Testing Strategy

- **Unit tests** (349): individual tool functions, edge cases
- **Integration tests** (85): full stdio flow, JSON-RPC round trips
- **Property tests**: numeric accuracy (BigDecimal vs. reference)

---

See [Tools Catalog](./TOOLS.md) for detailed tool specifications.
