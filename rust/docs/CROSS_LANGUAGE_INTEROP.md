# Cross-Language Interoperability Guide

The Life Savor agent supports multiple implementation languages for skills and providers. This guide explains how Rust-built components coexist with Node.js and Python implementations on the same agent.

## Language Support Overview

The agent runs providers and skills as child processes. It does not distinguish by implementation language — it only cares about the `command` and `args` fields in the Provider Manifest. This means:

| Language | Skill Providers | System Components |
|----------|----------------|-------------------|
| Rust     | Yes (compiled binary) | Yes (in-process) |
| Node.js  | Yes (via `node` command) | No |
| Python   | Yes (via `python` command) | No |

All skill providers communicate over the same JSON stdin/stdout protocol or MCP, regardless of language.

## System Components vs. Providers

This is the key architectural distinction:

**System Components** (System SDK) compile directly into the agent binary. They run in-process with privileged access to agent internals. They cannot be implemented in Node.js or Python — only Rust.

**Providers** (Model SDK, Assistant SDK, Skill SDK) are spawned as child processes. The agent communicates with them over stdin/stdout using JSON messages or MCP. Any language that can read JSON from stdin and write JSON to stdout works.

```
┌─────────────────────────────────────────────┐
│                  Agent Process               │
│                                              │
│  ┌──────────────┐  ┌──────────────────────┐ │
│  │ System       │  │ Provider Registry    │ │
│  │ Components   │  │                      │ │
│  │ (Rust only,  │  │  spawn + manage      │ │
│  │  in-process) │  │  child processes     │ │
│  └──────────────┘  └──────┬───────────────┘ │
│                           │                  │
└───────────────────────────┼──────────────────┘
                            │ stdin/stdout (JSON) or MCP
              ┌─────────────┼─────────────┐
              │             │             │
        ┌─────┴─────┐ ┌────┴────┐ ┌──────┴──────┐
        │ Rust Skill │ │ Node.js │ │ Python Skill│
        │ (binary)   │ │ Skill   │ │             │
        └────────────┘ └─────────┘ └─────────────┘
```

## JSON stdin/stdout Protocol

All skill providers use the same wire format. The agent writes a JSON request to the skill's stdin and reads a JSON response from stdout.

### Request Format

```json
{
  "tool": "my-tool",
  "input": {
    "query": "example input"
  },
  "context": {
    "correlation_id": "abc-123",
    "instance_id": "agent-01"
  }
}
```

### Response Format

```json
{
  "output": {
    "result": "example output"
  },
  "status": "success"
}
```

### Rust Implementation

```rust
use serde::{Deserialize, Serialize};
use std::io::{self, BufRead, Write};

#[derive(Deserialize)]
struct Request {
    tool: String,
    input: serde_json::Value,
}

#[derive(Serialize)]
struct Response {
    output: serde_json::Value,
    status: String,
}

fn main() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line.expect("failed to read stdin");
        let req: Request = serde_json::from_str(&line).expect("invalid JSON");

        let resp = Response {
            output: serde_json::json!({ "result": format!("handled {}", req.tool) }),
            status: "success".to_string(),
        };

        serde_json::to_writer(&mut stdout, &resp).expect("failed to write");
        stdout.write_all(b"\n").expect("failed to write newline");
        stdout.flush().expect("failed to flush");
    }
}
```

### Node.js Implementation

```javascript
const readline = require('readline');

const rl = readline.createInterface({ input: process.stdin });

rl.on('line', (line) => {
  const req = JSON.parse(line);

  const resp = {
    output: { result: `handled ${req.tool}` },
    status: 'success',
  };

  process.stdout.write(JSON.stringify(resp) + '\n');
});
```

### Python Implementation

```python
import json
import sys

for line in sys.stdin:
    req = json.loads(line.strip())

    resp = {
        "output": {"result": f"handled {req['tool']}"},
        "status": "success",
    }

    print(json.dumps(resp), flush=True)
```

## Provider Manifest — Language Is Just a Command

The Provider Manifest's `command` field determines how the agent spawns the provider. The agent applies the same sandbox restrictions regardless of language.

**Rust skill:**
```toml
[connection]
command = "./target/release/my-rust-skill"
args = []
transport = "stdio"
```

**Node.js skill:**
```toml
[connection]
command = "node"
args = ["dist/index.js"]
transport = "stdio"
```

**Python skill:**
```toml
[connection]
command = "python"
args = ["skill.py"]
transport = "stdio"
```

## MCP Transport

Skills can also use the Model Context Protocol (MCP) instead of raw JSON stdin/stdout. The agent performs a capability handshake and then invokes tools via the MCP protocol. This works identically across languages — the MCP wire format is JSON-RPC over stdio.

In the Skill SDK, MCP types are gated behind the `mcp` feature flag:

```toml
[dependencies]
lifesavor-skill-sdk = { version = "0.1", features = ["mcp"] }
```

## Sandbox Behavior Across Languages

All child-process providers (regardless of language) are subject to the same `ProcessSandbox` restrictions:

- **Environment variables**: Only variables declared in `sandbox.allowed_env_vars` are passed to the child process
- **Filesystem access**: Restricted to paths declared in `sandbox.allowed_paths`
- **Output size**: Stdout response must not exceed `sandbox.max_output_bytes`
- **Resource limits**: CPU and memory limits apply equally

The Skill SDK's `SandboxRunner` binary can test Rust skills locally with these restrictions. For Node.js and Python skills, the same restrictions are applied by the agent at runtime.

## Related Resources

- `SDK/system/` — System SDK (Rust-only, in-process components)
- `SDK/model/` — Model SDK (LLM provider integrations)
- `SDK/assistant/` — Assistant SDK (assistant provider integrations)
- `SDK/skill/` — Skill SDK (skill provider integrations, any language)
- `sdks/schemas/` — Language-agnostic JSON schemas for manifests and build configs
- `sdks/skills/` — Node.js and Rust skill SDK implementations
