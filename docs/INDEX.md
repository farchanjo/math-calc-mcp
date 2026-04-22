# arithma Documentation

**The Ultimate LLM Calculator Engine** — 87 precision math tools via Model Context Protocol (MCP).

## Quick Links

| Document | Purpose |
|:---|:---|
| [Architecture](./ARCHITECTURE.md) | System design, module layout, data flow |
| [Tools Catalog](./TOOLS.md) | Complete 87-tool reference by category |
| [Development Guide](./DEVELOPMENT.md) | Building, testing, contributing |
| [API Usage](./API.md) | MCP integration, tool calling conventions |

## What is arithma?

A pure-Rust MCP server exposing expert-grade calculator tools for language models. Designed for precision, performance, and seamless LLM integration.

**Key properties:**
- **87 tools** across 15 categories (math, finance, electronics, networking, units, etc.)
- **Arbitrary precision** via BigDecimal (DECIMAL128 semantics)
- **Zero C dependencies** — single static binary, ~3 MB
- **Fast** — sub-second startup, millisecond tool latency
- **Portable SIMD** — auto-dispatches SSE2/AVX2/AVX-512/NEON
- **Tested** — 434 unit + integration tests

## Common Tasks

### I want to...

**Build and run locally**
→ See [Development Guide](./DEVELOPMENT.md#building)

**Integrate with Claude Code**
→ See [API Usage](./API.md#integration)

**Understand a specific tool**
→ See [Tools Catalog](./TOOLS.md)

**Contribute a fix or feature**
→ See [Development Guide](./DEVELOPMENT.md#contributing)

**Understand how it works internally**
→ See [Architecture](./ARCHITECTURE.md)

---

**Repository**: [github.com/farchanjo/arithma](https://github.com/farchanjo/arithma)  
**License**: Apache-2.0
