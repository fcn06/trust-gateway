# System Boundaries & Limitations — What Trust Gateway Is Not

To evaluate **Trust Gateway** accurately, it is essential to understand its design scope, non-goals, and operating assumptions.

---

## 🚫 What Trust Gateway Is Not

### 1. Not an LLM Prompt Firewall
Trust Gateway does not inspect or sanitize prompt text sent into LLMs. It operates exclusively at the **tool proposal boundary** (after the LLM has already decided to invoke a tool). While it neutralizes prompt injection by denying unauthorized tool calls or tampered arguments, it does not detect prompt injections occurring within natural language conversation threads.

### 2. Not a Judge of Agent Reasoning Correctness
Trust Gateway evaluates proposed tool invocations against deterministic policies (`policy.toml`, identity attributes, human approval requirements). It cannot determine whether an agent's multi-step plan is logically optimal, contextually wise, or free of hallucinations, so long as each individual action satisfies authorized policy.

### 3. Not a Replacement for SaaS Authorization or Network Isolation
Trust Gateway issues cryptographic `ExecutionGrants` for executors. It relies on underlying network topology and deployment invariants to ensure:
- AI agents have **no direct network access** to target SaaS APIs or databases.
- SaaS credentials reside **exclusively on isolated Executor Hosts**.

If an agent has direct egress network routes and valid SaaS API keys, Trust Gateway cannot prevent direct out-of-band mutations.

### 4. Not a Magical Fix for Arbitrary Native Scripts
Executing arbitrary, uncontained native bash or python scripts on an executor host remains dangerous. Trust Gateway ensures that execution requests carry cryptographically verified grants, but containment of arbitrary code execution requires sandboxing mechanisms (e.g. Wasmtime, Docker containers, gVisor).

### 5. Not an LLM Output Hallucination Filter
Egress filtering in Trust Gateway (`trust-egress`) performs deterministic PII, secret, and pattern scrubbing on tool execution results before returning them to callers. It does not re-verify whether downstream LLMs accurately interpret those execution results.

### 6. Standalone Demo vs Production Relying Party (RP) Identity
The standalone demo and community gateway use a simplified HMAC session token model for rapid developer onboarding. In a full production enterprise deployment, the Gateway verifies individual user session JWTs signed by an OIDC Relying Party (RP) or Identity Provider, extracts `user_id` and tenant claims, and dynamically enforces per-user policy permissions and multi-tenant isolation.

---

## 🔒 Required Deployment Invariants

For Trust Gateway's security model to hold, the deployment environment **must** enforce these 5 physical invariants:

1. **No Agent Credentials**: AI agents do not hold standing SaaS credentials or database connection strings.
2. **Executor Isolation**: Executor hosts are the only system components permitted to perform side-effecting mutations on target systems.
3. **Network Egress Isolation**: Direct network paths between agent runtime nodes and SaaS endpoints are blocked by firewalls or VPC rules.
4. **Mandatory Grant Verification**: Executor hosts must refuse any tool invocation that lacks a valid, unexpired, single-use `ExecutionGrant` JWT.
5. **Gateway-Only Key Protection**: Ed25519 grant signing keys reside exclusively within the Trust Gateway control plane.
