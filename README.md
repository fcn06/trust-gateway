# 🛡️ Trust Gateway

![Trust Gateway Overall](docs/illustrations/Trust_Gateway_Overall_1.png)

[![Rust](https://img.shields.io/badge/Rust-1.88-orange?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue)](LICENSE)
[![NATS](https://img.shields.io/badge/NATS-JetStream-green?logo=nats.io)](https://nats.io)
[![MCP](https://img.shields.io/badge/MCP-SSE%20%2B%20Streamable-purple)](https://modelcontextprotocol.io)


> **Stop giving AI agents raw API keys. Trust Gateway is an Execution Firewall and Human-in-the-Loop gateway for AI tools.**

AI agents should be able to propose actions without automatically possessing the authority to execute them.

**Trust Gateway** sits between AI agents and side-effecting tools and systems (SaaS APIs, databases, native scripts). It evaluates proposed actions against deterministic policy rules and issues a short-lived, Ed25519-signed **ExecutionGrant** cryptographically bound to exactly one tool and one set of canonical parameters. 

Executors independently verify that grant before performing any mutation.

> **"Agents propose. Gateway decides. Executors verify."**

---
## **🚀 Quickstart (Python SDK in \< 2 minutes)**

Protect your AI agent's tool calls in 3 simple steps.

### **1. Clone & Start the Gateway**

```bash
git clone https://github.com/fcn06/trust_gateway.git  
cd trust_gateway

# Spin up Trust Gateway & dependencies in local development mode  
docker compose -f deploy/docker-compose.yml up -d
```

### **2. Install the Python SDK**
```bash
pip install -e sdks/python
```

### **3. Guard Your Tools (`quickstart.py`)**

Create a script named `quickstart.py` (or run `python examples/python-agent/quickstart.py`):

```python
import os  
from trust_gateway.client import TrustGatewayClient, guard_tool

# Initialize client (automatically uses local development credentials)  
client = TrustGatewayClient.dev_mode(gateway_url="http://localhost:3060")

# 1. Guard a safe/read-only action (Auto-Allowed by Gateway)  
@guard_tool(client, "claw_hello_world")  
def say_hello(message: str):  
    return {"status": "ok", "message": f"Hello, {message} ! "}

# 2. Guard a high-risk financial mutation (Requires Approval / Blocked by Gateway)  
@guard_tool(client, "stripe_refund")  
def process_refund(amount: int, order_id: str):  
    return {"status": "refunded", "amount": amount}

if __name__ == "__main__":  
    print("--- 1. Testing Allowed Action ---")  
    result = say_hello(message="World")  
    print(f"✅ Executed: {result}\n")

    print("--- 2. Testing High-Risk Mutation ---")  
    try:  
        # Gateway intercepts and blocks this because policy requires human confirmation  
        process_refund(amount=500, order_id="ord_123")  
    except Exception as e:  
        print(f"🛑 FIREWALL BLOCKED ACTION: {e}")
```

Run it:

```bash
python quickstart.py
```

```
--- 1. Testing Allowed Action ---  
✅ Executed: {'status': 'ok', 'message': 'Hello, World!'}

--- 2. Testing High-Risk Mutation ---  
⚠️  Decision: require_approval (Financial mutation requires human confirmation)  
🛑 FIREWALL BLOCKED ACTION: Action 'stripe_refund' requires human approval before an ExecutionGrant is issued.
```

---

## Other Ways to Run Trust Gateway

### Docker Demo (No Rust toolchain required)

Build and run the standalone demonstration locally in Docker:

```bash
# 1. Build local container image (or run: make demo-docker)
docker build -t trust-gateway-demo -f deploy/Dockerfile.demo .

# 2. Run standard execution demo
docker run --rm trust-gateway-demo
```

### Source Build (Rust 1.88+)

Run the standalone quickstart demo directly from source:

```bash
# Prerequisites: Rust 1.88+, build-essential / xcode-select, libssl-dev
cargo run -p quickstart-standalone
```
---

## 📜 Protocol vs. Reference Implementation

The **Execution Authorization Protocol** defines normative authorization contracts independent of specific runtime components:
* **Normative Schemas**: `ProposedAction`, `PolicyDecision`, `ExecutionGrant`, `GrantedAction`, `ExecutionResult`.
* **Canonicalization & Hashing**: Deterministic canonical JSON serialization with lexicographically sorted object keys followed by SHA-256 hashing (`input_hash`).
* **Verification Rules**: Ed25519 public key signature verification, single-use nonce checking (`jti`), and strict TTL expiration.

This repository provides the official **Rust reference implementation** and a **Python SDK**.

---
## 📖 Choose Your Path

| Goal | Resource / Guide |
| :--- | :--- |
| **Integrate via Python** | [`examples/python-agent/quickstart.py`](examples/python-agent/quickstart.py) |
| **Protocol Specification** | [`docs/reference/PROTOCOL_SPEC.md`](docs/reference/PROTOCOL_SPEC.md) |
| **Architecture Deep-Dive** | [`docs/concepts/ARCHITECTURE.md`](docs/concepts/ARCHITECTURE.md) |
| **Integrate via REST** | [`docs/tutorials/rest-curl-agent.md`](docs/tutorials/rest-curl-agent.md) |
| **Write a Custom Policy** | [`docs/how-to/write-policy.md`](docs/how-to/write-policy.md) |

---
## 🧰 Development

```bash
make doctor       # Verify prerequisites
make check        # Check workspace compilation
make test         # Run unit tests
make quickstart   # Run standalone demo
make demo-docker  # Build demo Docker image
make conformance  # Run protocol conformance vectors
make lint         # Run clippy & cargo fmt checks
```

---

Maintained by [lianxi.io](https://lianxi.io). Security disclosures: [`SECURITY.md`](SECURITY.md).
