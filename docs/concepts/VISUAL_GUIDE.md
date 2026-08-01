# 🎨 Trust Gateway: 5-Minute Visual Guide & Concepts

Welcome to the **Trust Gateway Visual Guide**. This document explains **why** Trust Gateway exists, **how** it works under the hood, and **where** each concept lives in the codebase.

---

## ⚡ The Core Problem: Why Trust Gateway?

Traditional AI agent frameworks give LLMs direct access to database credentials, cloud APIs, and shell commands. If an agent hallucinates or encounters a prompt injection attack, it can execute catastrophic operations directly.

```mermaid
graph TD
    subgraph Traditional_Unsafe_Agent ["❌ Traditional Unsafe Agent Architecture"]
        A1["AI Agent (LLM)"] -->|Direct API Keys & DB Access| B1["Production Database / Cloud APIs"]
        A1 -->|Prompt Injection / Hallucination| C1["🔥 Catastrophic Unchecked Mutation!"]
    end
```

### The Trust Gateway Solution

`trust-gateway` separates an agent's **intent to act** from the **authority to execute**. The LLM can only *propose* actions. The Gateway evaluates policies, mints cryptographic grants, and the Executor verifies the grant before touching real systems.

```mermaid
graph TD
    subgraph Zero_Trust_Gateway ["🛡️ Trust Gateway Zero-Trust Architecture"]
        A2["AI Agent (LLM)"] -->|1. Propose Action| B2["Trust Gateway"]
        B2 -->|2. Evaluate Policy| C2{"Policy Engine"}
        C2 -->|3a. Deny| D2["❌ Action Rejected"]
        C2 -->|3b. Approve| E2["🔑 Mint ExecutionGrant JWT"]
        E2 -->|4. Signed Grant| F2["Executor Host"]
        F2 -->|5. Verify Ed25519 & SHA-256 input_hash| G2["⚡ Target API / Sandbox"]
        G2 -->|6. PII Egress Filter| H2["✅ Sanitized Result"]
    end
```

---

## 🔄 End-to-End Sequence Diagram

Here is the exact step-by-step control flow for an agent action request:

```mermaid
sequenceDiagram
    autonumber
    actor Agent as 🤖 AI Agent (LLM)
    participant Gateway as 🛡️ Trust Gateway
    participant Policy as ⚖️ Policy Engine (policy.toml)
    participant Human as 👤 Human Approver (Portal / Client UI)
    participant Executor as ⚡ Executor Host
    participant API as 🌐 Target API / Tool

    Agent->>Gateway: Propose Action payload (tool, args, context)
    Gateway->>Policy: Evaluate ProposedAction vs policy.toml
    
    alt Require Human Approval
        Policy-->>Gateway: Result: RequireApproval
        Gateway->>Human: Trigger Human Approval prompt
        Human-->>Gateway: Approve Action
    else Auto-Approved
        Policy-->>Gateway: Result: Allow
    end

    Gateway->>Gateway: Calculate SHA-256(input_hash) & Mint Ed25519 ExecutionGrant JWT
    alt Managed Dispatch (Production Default)
        Gateway->>Executor: GrantedAction (Grant JWT + Canonical Args) via NATS
    else Portable Grant (REST / MCP clients)
        Gateway-->>Agent: Return ExecutionGrant JWT
        Agent->>Executor: Execute(Grant JWT + Canonical Args)
    end
    
    Executor->>Executor: Verify Ed25519 Signature
    Executor->>Executor: Verify SHA-256(input_hash) matches args
    Executor->>Executor: Check Single-Use Nonce (Replay Prevention)
    
    Executor->>API: Execute Tool Mutation
    API-->>Executor: Raw Result
    Executor->>Executor: Apply PII & Secret Egress Scrubbing
    Executor-->>Agent: Return Sanitized Execution Result
```

> **Managed dispatch** is the production default: the Gateway dispatches directly to the Executor via NATS. **Portable grant** mode is available for REST and MCP integrations.

---

## 🧩 Core Concepts & Rosetta Stone

| Concept | What It Does | Codebase Location |
| :--- | :--- | :--- |
| **`ProposedAction`** | The raw intent submitted by the agent ("What I want to do"). | [`crates/trust-model`](../../crates/trust-model) |
| **`Policy Engine`** | Evaluates action attributes against `policy.toml` priority rules. | [`crates/trust-policy`](../../crates/trust-policy) |
| **`ExecutionGrant`** | Short-lived (30s-60s) Ed25519-signed JWT proving authority to execute. | [`crates/trust-grants`](../../crates/trust-grants) |
| **`input_hash`** | SHA-256 digest of canonicalized JSON arguments preventing parameter tampering. | [`crates/trust-canonical`](../../crates/trust-canonical) |
| **`Single-Use Nonce`** | Tracking mechanism ensuring an `ExecutionGrant` can never be reused. | [`gateway/src/grant.rs`](../../gateway/src/grant.rs) |
| **`Executor Host`** | Isolated execution runtime verifying Ed25519 signatures before dispatching tools. | [`executor_host/`](../../executor_host) |
| **`PII Egress Filter`** | Executor-side mandatory regex filter scrubbing API keys, emails, and secrets from results before they leave the execution boundary. | [`crates/trust-egress`](../../crates/trust-egress) |

---

## 🚀 Quick Verification

To see the complete zero-trust control flow run standalone in 3 seconds:

```bash
cd trust-gateway
cargo run -p quickstart-standalone
```
