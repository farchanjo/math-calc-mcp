# API Usage & Integration

How to integrate arithma into your MCP client and call tools from code or LLMs.

## MCP Integration

### Claude Code

```bash
claude mcp add arithma -- /absolute/path/to/target/release/arithma
```

### Claude Desktop (mcp.json)

```json
{
  "mcpServers": {
    "arithma": {
      "command": "/absolute/path/to/target/release/arithma"
    }
  }
}
```

### Cursor, Windsurf, OpenCode

All support the same stdio interface. Create a `.mcp-config.json` or equivalent and point to the binary path.

### Verify Connection

```bash
(printf '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}\n';
 printf '{"jsonrpc":"2.0","method":"notifications/initialized"}\n';
 printf '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}\n';
 sleep 0.3) | /absolute/path/to/arithma 2>/dev/null | jq .
```

Should return `initialize`, then `tools/list` with all 87 tools.

## Tool Calling Convention

All tools follow the MCP `tools/call` JSON-RPC method:

```json
{
  "jsonrpc": "2.0",
  "id": 123,
  "method": "tools/call",
  "params": {
    "name": "tool_name",
    "arguments": {
      "param1": "value1",
      "param2": 42,
      "param3": {"nested": "object"}
    }
  }
}
```

### Response Format

**Success**:
```json
{
  "jsonrpc": "2.0",
  "id": 123,
  "result": "computed_value"
}
```

**Error** (embedded as string):
```json
{
  "jsonrpc": "2.0",
  "id": 123,
  "result": "Error: invalid parameter"
}
```

## Common Use Cases

### 1. Basic Arithmetic

**Request**:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "add",
    "arguments": {"first": "0.1", "second": "0.2"}
  }
}
```

**Response**:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": "0.3"
}
```

### 2. Expression Evaluation

**Request**:
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/call",
  "params": {
    "name": "evaluate",
    "arguments": {"expression": "2 * sin(45) + 3^2"}
  }
}
```

**Response**:
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": "10.41421356237"
}
```

### 3. Variable Substitution

**Request**:
```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "tools/call",
  "params": {
    "name": "evaluateWithVariables",
    "arguments": {
      "expression": "principal * (1 + rate/100)^years",
      "variables": "{\"principal\": 1000, \"rate\": 5, \"years\": 10}"
    }
  }
}
```

**Response**:
```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "result": "1628.89463..."
}
```

### 4. Unit Conversion

**Request**:
```json
{
  "jsonrpc": "2.0",
  "id": 4,
  "method": "tools/call",
  "params": {
    "name": "convert",
    "arguments": {
      "value": "5",
      "fromUnit": "km",
      "toUnit": "mi",
      "category": "LENGTH"
    }
  }
}
```

**Response**:
```json
{
  "jsonrpc": "2.0",
  "id": 4,
  "result": "3.10685596120..."
}
```

### 5. Financial Calculation

**Request**:
```json
{
  "jsonrpc": "2.0",
  "id": 5,
  "method": "tools/call",
  "params": {
    "name": "compoundInterest",
    "arguments": {
      "principal": "10000",
      "annualRate": "5",
      "years": "20",
      "compoundsPerYear": 12
    }
  }
}
```

**Response**:
```json
{
  "jsonrpc": "2.0",
  "id": 5,
  "result": "27099.234..."
}
```

### 6. Networking

**Request**:
```json
{
  "jsonrpc": "2.0",
  "id": 6,
  "method": "tools/call",
  "params": {
    "name": "subnetCalculator",
    "arguments": {
      "address": "192.168.1.0",
      "cidr": 24
    }
  }
}
```

**Response**:
```json
{
  "jsonrpc": "2.0",
  "id": 6,
  "result": "{\"network\": \"192.168.1.0\", \"broadcast\": \"192.168.1.255\", \"mask\": \"255.255.255.0\", \"wildcard\": \"0.0.0.255\", \"firstHost\": \"192.168.1.1\", \"lastHost\": \"192.168.1.254\", \"usableHosts\": 254, \"ipClass\": \"C\"}"
}
```

### 7. Electronics

**Request**:
```json
{
  "jsonrpc": "2.0",
  "id": 7,
  "method": "tools/call",
  "params": {
    "name": "ohmsLaw",
    "arguments": {
      "voltage": "12",
      "current": "",
      "resistance": "4",
      "power": ""
    }
  }
}
```

**Response**:
```json
{
  "jsonrpc": "2.0",
  "id": 7,
  "result": "{\"voltage\": \"12\", \"current\": \"3\", \"resistance\": \"4\", \"power\": \"36\"}"
}
```

## Parameter Types

### Strings (Decimal Numbers)

For all numeric parameters, use **strings** to preserve arbitrary precision:

```json
{"value": "0.123456789012345678901234567890"}
```

**Not** a JSON number (which loses precision).

### JSON Objects (Nested Parameters)

Some tools require JSON objects as string arguments:

```json
{
  "name": "evaluateWithVariables",
  "arguments": {
    "expression": "2*x + y",
    "variables": "{\"x\": 3.5, \"y\": 2}"
  }
}
```

The `variables` parameter is a **string containing JSON**, not a direct object.

### CSV Arrays

Array parameters use comma-separated strings:

```json
{
  "name": "sumArray",
  "arguments": {"numbers": "1,2,3,4,5"}
}
```

### Boolean Flags

Pass empty string for "not provided":

```json
{
  "name": "ohmsLaw",
  "arguments": {
    "voltage": "12",
    "current": "",
    "resistance": "4",
    "power": ""
  }
}
```

Provide any 2 of the 4 (voltage, current, resistance, power).

## Error Responses

All errors are embedded as strings:

```json
{
  "jsonrpc": "2.0",
  "id": 123,
  "result": "Error: Unknown unit: foobar"
}
```

**Common errors**:
- `"Error: Unknown unit: ..."` — Unit not recognized
- `"Error: Cannot convert between ... and ..."`  — Cross-category conversion
- `"Error: Division by zero"`
- `"Error: Unknown category: ..."`
- `"Error: Gas mark must be 1-10..."`

The MCP protocol guarantees a `result` field (never a protocol error).

## Performance Tips

1. **Batch requests** — Send multiple tool calls in sequence rather than waiting for each response
2. **Cache results** — If the same calculation runs twice, cache locally
3. **Use expressionExact sparingly** — High precision (~128 bits) is slower than standard evaluation
4. **Pre-compile expressions** — If evaluating the same expression many times, consider evaluating once and using the result

## Precision Guarantees

| Tool | Precision | Notes |
|:---|:---|:---|
| Arithmetic (add, sub, mul, div) | Exact (BigDecimal) | Division scaled to 20 decimal places |
| Scientific (sin, cos, tan, log) | Exact at notable angles | 9 built-in functions, lookup tables |
| Programmable (evaluate) | f64 (53-bit) | ~15–17 significant digits |
| Programmable (evaluateExact) | 128-bit float | ~34 significant digits |
| Financial | DECIMAL128 | 34 significant digits, HALF_UP rounding |
| Unit conversion | 34 significant digits | DECIMAL128 factors |

---

See [Tools Catalog](./TOOLS.md) for complete tool reference.
