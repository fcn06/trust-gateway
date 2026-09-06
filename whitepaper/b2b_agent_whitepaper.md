# Interaction Contracts for Autonomous B2B Agents: Architecture, Threat Model, and Open Questions

**Status:** Working draft, personal project — not peer-reviewed, not a standard, not a finished system  
**Author:** fcn06 (Lianxi / Trust Gateway project, [lianxi.io](https://lianxi.io))  
**Date:** September 2026  
**Where this lives:** GitHub ([fcn06/trust_gateway](https://github.com/fcn06/trust_gateway)) and lianxi.io — this is not submitted anywhere, and I have no current plans to submit it to arXiv or a standards body until there's real implementation and outside review behind every claim in it.  

---

## How to read this document

This is a technical report about an architecture I've been building called the **Negotiated Interaction Contract Protocol (NICP)**, together with the **Trust Gateway** that enforces it. It grew out of a shorter dev.to article ("What If the Enterprise API of the Future Is an Agent?") and goes into the implementation detail that piece didn't have room for.

A few honesty notes up front, because an earlier draft of this document didn't have enough of them:

- **This is one person's project, not a working group.** I'm not going to pretend otherwise by using "we" or inventing an organizational name. Where I say "I," I mean me; where I say "the project," I mean this codebase.
- **Not everything described here is built, and not everything built is public.** Throughout this document I've tagged each major claim with where it actually stands:
  - 🟢 **Implemented and open source** — you can read the code.
  - 🟡 **Implemented, not yet public** — it exists in my private build, working but not yet released or independently reviewed.
  - ⚪ **Design goal, not yet built** — this is the target behavior, not a claim about what exists today.
- **Nothing here has had independent security review.** The threat model and the "invariants" in Section 7 are things I'm *designing toward*, not things I can claim are proven. I'd genuinely like people who do this for a living to try to break them.
- **The market narrative in Section 1.3 is accurate as far as I can verify**, but I'm one person reading public reporting, not an analyst with inside information — treat it as informed context, not authoritative market analysis.

If you're the kind of reader who wants to poke holes in an architecture, this document is written for you. If you're evaluating whether to trust this in production — please don't yet. It isn't there.

---

## Table of Contents

1. [The Problem](#1-the-problem)
2. [Architecture Overview](#2-architecture-overview)
3. [The Interaction Contract](#3-the-interaction-contract)
4. [Stateful Authorization and Risk](#4-stateful-authorization-and-risk)
5. [The Semantic Verification Pipeline](#5-the-semantic-verification-pipeline)
6. [Cryptographic Attestation and Execution Grants](#6-cryptographic-attestation-and-execution-grants)
7. [Threat Model and Design Invariants](#7-threat-model-and-design-invariants)
8. [Operational Trace and Audit Receipts](#8-operational-trace-and-audit-receipts)
9. [How This Relates to Other Work](#9-how-this-relates-to-other-work)
10. [Where This Goes Next](#10-where-this-goes-next)
11. [References](#11-references)

---

## 1. The Problem

### 1.1 Point-to-point integration doesn't scale

Connecting two enterprises today usually means custom API adapters, bespoke schemas, mutual auth, rate limits, webhooks, and SDKs, built and maintained per pair. With more counterparties, the number of bilateral integrations grows roughly with the square of the number of participants — everyone who has worked in enterprise integration has lived this. Global schema standards (EDIFACT, SWIFT, RosettaNet, FHIR) reduce the heterogeneity but take years to adopt and are expensive to implement.

### 1.2 Why I don't think agents should get direct execution access

LLM agent frameworks make it tempting to let an external agent talk to an internal agent that itself holds API keys or database credentials. I think that's a bad idea, for reasons that aren't hypothetical:

1. **LLMs can't give you a hard guarantee about their own boundaries.** A slightly unusual phrasing can push a model past a limit it was "told" to respect.
2. **Cross-boundary prompt injection is real.** Content embedded in a partner's proposal, purchase order, or quote metadata can attempt to redirect the receiving agent's behavior.
3. **Free-form negotiation isn't auditable.** Two agents talking in natural language don't produce a record either side can point to later and say "this is exactly what we agreed."
4. **Confused deputy problems are easy to create.** An internal agent with broad system access, acting on behalf of an external, less-trusted counterparty, is a classic setup for that failure mode.

### 1.3 What's actually happening in the market right now

This section describes real, checkable events, not my own predictions.

Through 2025 and into 2026, several companies tried to build **centralized agentic-checkout gateways** — Google and Shopify's Universal Commerce Protocol, Microsoft's Copilot Checkout, Perplexity's Instant Buy, Amazon's Buy for Me, and OpenAI's ChatGPT Instant Checkout among them.

OpenAI's version is the clearest data point: it launched in September 2025 and was retired in March 2026, about six months later. Walmart reported conversion rates roughly three times lower for in-chat checkout than for its own site, and reporting at the time pointed to broader issues — inaccurate product data, low merchant participation, and a general reluctance from merchants to hand over the customer relationship, margin, and post-purchase experience to an intermediary. The pattern that emerged: people are comfortable discovering products through AI, much less comfortable completing a purchase inside someone else's chat window.

On September 2, 2026, Anthropic released **Claude Commerce Agents**, an open reference blueprint (Apache 2.0) for building shopping and merchant agents that live inside a merchant's own website, app, and back office, rather than inside a third-party aggregator. Anthropic's own material is explicit that the reference implementation does not place orders or charge cards — those actions stay with the merchant's existing systems, and the agent's role is scoped to recommending, cart-staging, and handing off to normal checkout.

I read this as a useful, independent confirmation of the thesis I'd already been building toward: **merchants and enterprises want the agent to live on their own side of the boundary, not to cede transaction authority to it.** I want to be careful about how I frame this, though — Anthropic didn't design Claude Commerce Agents with a gap in mind for this project to fill, and I have no relationship with them around this work. What I think is true is narrower and more useful: their reference implementation demonstrates real market demand for exactly the kind of thing this project is trying to build — a way to give a merchant-embedded agent actual transactional authority, safely, once it's ready to go further than recommend-and-handoff. That's a complementary relationship I'm claiming, not an endorsed one.

### 1.4 The principle this project is built around

The idea I keep coming back to:

```text
Effective authority = negotiated contract ∩ enterprise policy ∩ identity authority ∩ delegated authority
```

Agents get full flexibility to negotiate, discover, and propose. They get none of the underlying authority to make something happen — that's evaluated separately, deterministically, by something that isn't an LLM.

```text
   Autonomous B2B agent (proposes intent and terms)
                │
                ▼
   Negotiated contract (cryptographically sealed once agreed)
                │
                ▼
   Trust Gateway (checks contract against policy, deterministically)
                │
                ▼
   Execution grant (scoped to one exact action)
                │
                ▼
   Isolated executor (verifies the grant, then acts)
                │
                ▼
   Enterprise systems (ERP, payments, inventory, ...)
```

---

## 2. Architecture Overview

### 2.1 Two planes, deliberately separated

The system is split into a **semantic plane** (probabilistic, not trusted with authority) and a **control plane** (deterministic, holds all the authority).

**Semantic plane** — the enterprise's own B2B agent plus whatever external agent it's talking to. Talks over agent-to-agent protocols. Discovers capabilities, proposes terms, drafts schema mappings. 🟢 *Has no credentials to internal systems and cannot trigger a mutating call directly — this separation is enforced structurally, not by convention.*

**Control plane** — the Trust Gateway, the contract engine, the policy layer, and isolated executors. No LLM sits anywhere in this decision path. 🟡 *Implemented in my private build in Rust, targeting WebAssembly for the executor sandbox. This part is the most mature piece of the system, but I haven't had it independently reviewed.*

```text
SEMANTIC PLANE (probabilistic, no authority)
  External agent  <──── A2A / DIDComm ────>  Enterprise B2B agent
                                                      │
                                          mutual signing ceremony
                                                      ▼
CONTROL PLANE (deterministic, holds all authority)
  Trust Gateway  ──►  Execution Grant  ──►  Isolated Executor  ──►  ERP / payments / inventory
```

### 2.2 A note on network topology

In my own deployment, the public-facing edge (a stateless ingress that terminates external connections and holds no credentials) is physically separate from the core enclave that houses the Trust Gateway, policy engine, and executor sandboxes. That's a sensible pattern for this kind of system generally, not something specific to this protocol — I mention it here mainly so the diagrams later make sense, not as a claim that this is the only correct topology. 🟡

### 2.3 Why I think the agent should sit inside the enterprise, not in front of it

Given what happened with the centralized checkout gateways (§1.3), I think the more durable model is an enterprise running its own B2B agent as the front door across all its inbound channels — web, app, human sales, partner agents, supplier agents — rather than routing all of that through someone else's aggregator. That keeps customer ownership, pricing, and the relationship inside the enterprise, and it means the reasoning model underneath (Claude, an open-weights model, whatever) is swappable without touching the execution guarantees below it. This is a design preference I hold, not a proven claim about how the market will settle.

---

## 3. The Interaction Contract

### 3.1 What it actually is

An Interaction Contract is a signed, versioned, machine-readable description of what two organizations have agreed their agents can do together: who the counterparties are, what capabilities are in scope, what constraints apply (money, geography, time), what data can move, what obligations exist, and how the agreement can end. Once signed by both sides it's frozen — a new version is a new contract, referencing the old one, not a silent edit.

One clarification I think matters: this is a **machine-enforceable technical agreement**, not a legal contract. The two may eventually need to intersect once commercial terms are involved, but I'm not conflating them here.

I'm also genuinely unsettled on one modeling question: whether this should be one object, or an envelope over several independently versioned pieces (identity, capability agreement, schema mapping, policy constraints, delegation, obligations, attestations) that can each change on their own timeline — the schema mapping, for instance, plausibly needs to evolve independently of the commercial relationship it sits inside. I haven't picked a final answer. 🟡 *My current implementation treats it as one versioned object; I suspect the envelope model is more correct long-term.*

```rust
pub struct InteractionContract {
    pub contract_id: String,
    pub version: u32,
    pub state: ContractState,
    pub issuer: PartyIdentity,
    pub counterparty: PartyIdentity,
    pub purpose: Purpose,
    pub capabilities: Vec<ContractCapability>,
    pub constraints: ContractConstraints,
    pub data_policy: DataPolicy,
    pub obligations: Vec<Obligation>,
    pub commercial_terms: Option<CommercialTerms>,
    pub validity: ContractValidity,
    pub evidence: ContractEvidence,
    pub parent_contract_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```
🟢 *This struct reflects the actual open-source data model.*

To make this concrete, here is an example of what an agreed contract draft looks like when serialized:

```yaml
# Example minimal Interaction Contract draft (YAML representation)
contract_id: "ctr_0191c7a4-82a1-7000-84c1-6e792c300001"
version: 1
issuer:
  did: "did:web:buyer.corp.example"
counterparty:
  did: "did:web:supplier.logistics.example"
purpose:
  code: "procure_freight_service"
capabilities:
  - capability_id: "io.company.orders@v1"
    operations: ["quote", "create", "status", "cancel"]
constraints:
  max_transaction_value:
    amount_minor: 2500000  # Stored as minor units: €25,000.00
    currency: "EUR"
  geography: ["EU"]
validity:
  not_before: "2026-10-01T00:00:00.000Z"
  not_after: "2026-10-31T23:59:59.000Z"
```

A fair objection here: isn't this just OAuth scopes plus a JSON Schema plus a policy engine? What I think makes it a different kind of object is that it binds several dimensions together — counterparties, purpose, capabilities, semantics, constraints, data policy, obligations, validity, and attestation — into one versioned, hashable, jointly-signed artifact, rather than leaving those as separately-managed pieces of infrastructure. Whether that's worth a new name or just a well-organized combination of existing primitives is a fair thing to disagree with me about.

### 3.2 Making the contract byte-identical everywhere

Two parties signing "the same contract" only means something if they're signing the exact same bytes, regardless of platform, language, or key ordering. I use the JSON Canonicalization Scheme (RFC 8785): sort object keys, represent money as integer minor units rather than floats, normalize timestamps to UTC ISO 8601, and strip signature fields and runtime-only metadata before hashing.

```text
Raw contract  →  strip signatures & runtime state  →  sort keys, normalize numbers and timestamps
             →  canonical byte array  →  SHA-256  →  contract_hash
```

🟢 *This canonicalization and hashing step is implemented and is the one piece of the system I'm most confident actually works as described, because it's the simplest part — it's a pure function with no distributed-systems complexity.*

I also support representing the same contract as JSON-LD (for public discovery and W3C Verifiable Credential interop), as CBOR (for low-latency wire transport), and as YAML (for human review and GitOps-style policy repos) — all of which need to produce the identical canonical hash regardless of which format you started from. 🟡

**Casing Normalization Note:** To keep hashes identical across formats, the canonical dictionary is normalized to `snake_case` keys prior to RFC 8785 sorting. When parsing JSON-LD using standard W3C `camelCase` identifiers (e.g., `capabilityId`, `maxTransactionValue`, `notBefore`), the parser deterministically projects property keys onto the canonical dictionary using the bijective `@context` mapping table before computing `contract_hash`. 🟡

### 3.3 Signing options

I support three ways to cryptographically bind a contract, because different enterprises will have different identity infrastructure already in place:

| Option | Trust model | Fits |
|---|---|---|
| UCAN | Trustless, cryptographic capability delegation | Sovereign/decentralized agent setups |
| DIDComm v2 / Verifiable Credentials | Decentralized identity, DID-based | Multi-party B2B where DIDs are already in use |
| X.509 / enterprise PKI | Traditional CA hierarchy | Enterprises already running PKI (most large ones) |

⚪ All three bindings are design targets. I've only exercised the PKI path end-to-end in my own testing so far; the UCAN and DIDComm paths are less proven.

### 3.4 Contract lifecycle

A contract moves through a fixed set of states — draft, proposed, counter-proposed, accepted, attested, active, and then one of revoked, suspended, or expired. Once it reaches a terminal state it's immutable; changes require a new version referencing the old contract. Negotiations that stall (no mutual attestation within a configurable window, default one hour) automatically move to "abandoned" and get cleaned up rather than accumulating indefinitely. 🟢 *The state machine and timeout/pruning behavior are implemented.*

---

## 4. Stateful Authorization and Risk

A plain per-request check isn't enough here. An agent with a legitimate €10,000 single-transaction limit could still drain a large amount of money by making that request a hundred times in quick succession. So the Trust Gateway needs to track state across requests, not just evaluate each one in isolation.

### 4.1 Effective authority, again

```text
effective_authority = min(contract_limit, enterprise_policy_limit, identity_delegation_limit)
```

If the contract allows up to €50,000 but enterprise policy caps autonomous agents at €10,000, the effective limit is €10,000. Straightforward, but worth stating explicitly because it's the thing that stops a negotiated agreement from ever becoming *more* permissive than what the enterprise actually allows.

### 4.2 Velocity and cumulative exposure

I track (a) total value moved in a sliding time window, (b) call frequency against a given capability, and (c) cumulative lifetime value under a specific contract — and reject or escalate once any of these thresholds is crossed, even if each individual request was within its own limit. 🟡 *Implemented against a distributed key-value store with atomic compare-and-swap, to avoid race conditions when the Gateway is running as multiple instances. I haven't load-tested this under real concurrent adversarial conditions.*

### 4.3 Human escalation

High-value, novel-counterparty, or sensitive-data transactions get routed to a human approval step rather than an automatic grant, with a "the agent that proposed it can't be the one that approves it" rule and hardware-backed (WebAuthn/FIDO2) sign-off for anything above a configurable threshold. ⚪ Design goal — the escalation path exists in my implementation, but I haven't yet exercised the multi-party approval quorum logic in anything beyond a single test scenario.

### 4.4 One thing I'm not fully solving yet: composition

A person combining several individually-safe grants can still end up somewhere unsafe — read access to a catalog, permission to submit a purchasing recommendation, and permission to send an external message might each be fine alone and risky together. My current effective-authority formula evaluates one action against contract/policy/identity limits; it doesn't yet reason about combinations of actions across a longer transaction history, cumulative risk from unrelated capabilities, or separation-of-duties at that broader scale. I think this eventually pushes the Gateway toward genuinely stateful, history-aware authorization rather than the simpler per-action model I have now, and I don't have that built. ⚪

---

## 5. The Semantic Verification Pipeline

### 5.1 Why I don't let an LLM translate schemas at transaction time

If a model translates `{"order_qty": 500}` into `{"quantity": "500"}`, or reads a date as day/month when it meant month/day, that's a silent, high-stakes error that a protocol-level firewall won't catch — because syntactically nothing looks wrong. Two agents can also convince each other that `cancel_order` means the same thing on both sides ("void the order outright" vs. "file a cancellation request pending approval") when it doesn't. I think this kind of **semantic false agreement** is a harder and more dangerous problem than anything cryptographic in this system, because it fails silently rather than loudly.

### 5.2 What I do instead: propose, verify, freeze

Rather than letting an LLM re-derive a schema mapping at execution time, the idea is to treat mapping generation as a one-time compilation step:

1. **Propose** — an LLM looks at both schemas and proposes a mapping.
2. **Verify** — the mapping goes through automated checks: required fields present, no lossy numeric conversions, round-trip identity for critical identifiers (mapping an ID forward and back returns the same ID), and property-based fuzz testing against a large number of synthetic payloads.
3. **Freeze** — once verified, the mapping is compiled into a small, sandboxed WebAssembly module with no filesystem or network access, hashed, and that hash (`mapping_hash`) is stamped directly into the capability definition within the `InteractionContract`. From that point on, the runtime executor loads the exact compiled Wasm bytecode matching that content hash, completely bypassing any live model inference.

```text
Schema A + Schema B → LLM proposes mapping → automated verification (types, round-trip, fuzzing)
                                                        │
                                                 compiled to sandboxed Wasm, content-hashed
                                                        │
                                          referenced in the contract; runtime uses the compiled artifact, not the LLM
```

⚪ This whole pipeline is a design goal. I've built and tested the "propose" and "freeze into Wasm" steps individually; I haven't yet built the full automated verification gate (the round-trip and fuzzing checks) as a single pipeline that blocks activation on failure. In other words: the idea is sound and partially prototyped, but don't read this section as "solved."

---

## 6. Cryptographic Attestation and Execution Grants

This is the part of the system I have the most actual confidence in, because it's the part that's been running the longest in my own testing.

### 6.1 Mutual attestation

A contract doesn't become active on one party's say-so. Both sides compute the same canonical hash independently, sign it, and exchange signatures; the contract only activates once both signatures verify against the identical hash. If the two computed hashes don't match, activation fails outright — there's no "close enough." 🟢

### 6.2 Execution grants

When an agent wants to actually do something under an active contract, the Trust Gateway checks it against the contract and current policy/velocity state, and if it passes, issues a short-lived, narrowly scoped execution grant (I use a signed JWT) that includes:

- a short time-to-live (tens of seconds, not minutes),
- the exact hash of the canonical arguments (`input_hash`) — the grant is valid for this specific call, with these specific arguments, not "a call like this one,"
- the contract ID and its hash, so every execution traces back to the exact agreement it happened under.

🟢 *This binding — grant valid for one action, argument-hash-locked, short TTL — is implemented and is the core security property of the system.*

Here is the exact claim structure of an issued `ExecutionGrant` JWT payload:

```json
{
  "iss": "did:web:company.example:trust-gateway",
  "sub": "did:key:zAgentProcurementWorker",
  "aud": "did:web:company.example:executor-host",
  "jti": "grant_0191c7b5-22a4-7000-91c2-3e817c200042",
  "iat": 1790870400,
  "exp": 1790870460,
  "tool_id": "io.company.orders.create@v1",
  "input_hash": "sha256:d59b207559e355c70752b047a0640df14541bfd6e3be4ff28e67a48d88e6de02",
  "contract_id": "ctr_0191c7a4-82a1-7000-84c1-6e792c300001",
  "contract_hash": "sha256:7f83b1657ff1fc53b92dc18148a1d65dfc2d4b1fa3d677284addd200126d9069"
}
```

**One thing I want to be precise about, because it's easy to overstate:** a short TTL and a unique grant ID make replay much harder, but I'm not yet claiming airtight, verified single-use enforcement across every distributed executor topology. That would require me to show the replay cache is consistent and race-free across all deployment configurations, and I haven't done that verification work. Treat "short-lived and single-purpose" as accurate; treat "provably replay-proof in all conditions" as not yet earned.

### 6.3 The executor

The executor is deliberately dumb: it verifies the grant's signature, checks it hasn't expired, checks the argument hash matches exactly, checks the tool ID matches, and only then executes. It doesn't call an LLM, doesn't interpret policy, doesn't make judgment calls. 🟢

---

## 7. Threat Model and Design Invariants

I'll use the STRIDE categories loosely rather than trying to make this exhaustive.

| Attack | Where it hits | What's supposed to stop it |
|---|---|---|
| Prompt injection in a negotiation message | Semantic plane | The proposal still has to pass through the deterministic Gateway; an LLM being tricked into proposing an unlimited value doesn't matter if policy still clips it to the real limit |
| Replaying a previously valid signature | Attestation | Signatures bind to the exact contract hash plus a fresh nonce tied to the negotiation session; previously-seen attestations are rejected |
| Silently modifying a stored contract after activation | Storage | The Gateway recomputes the stored contract's hash on every evaluation and compares it to the attested hash; any drift suspends the contract immediately |
| Bypassing the Gateway and hitting the executor directly | Execution | Executors have no ambient credentials and only accept requests carrying a valid, signature-verified grant with a matching argument hash |
| Many small transactions adding up to something large | Cumulative risk | Sliding-window velocity and cumulative-exposure limits, not just per-transaction limits (see §4.2) |

### 7.1 Design invariants I'm aiming to hold

I originally labeled these "formal, mathematically proven invariants." That was overclaiming — I haven't proven any of them in the mathematical sense, and nobody outside this project has tried to break them. Here's the honest version: these are the properties the architecture is *designed* to guarantee. I believe the design is sound; I don't yet have evidence beyond my own testing.

1. No agent can mutate an enterprise system without going through a verified execution grant.
2. Agents hold no long-lived operational credentials.
3. A contract can never grant more than enterprise policy independently allows.
4. A contract can't activate without mutual, verified attestation over the identical hash.
5. Every execution grant is bound to the exact argument hash of the action it authorizes.
6. Every grant has a unique ID and a short TTL, intended to make reuse hard (see the honesty note in §6.2 about the limits of this claim).
7. An agent can't negotiate terms beyond what its own delegated authority allows.
8. Capabilities reference a fixed registry, not agent-invented tool names.
9. Executors never run an LLM or evaluate policy themselves.
10. Contracts that are revoked, suspended, or expired can't mint new grants.
11. Schema transformations run through versioned, content-addressed compiled artifacts, not live inference, once frozen (§5.2).
12. Failures return typed error codes without leaking internal state, prompts, or stack traces.

I'd genuinely like feedback from people who do adversarial security work on which of these are actually well-founded and which are wishful thinking.

### 7.2 What this list doesn't cover

Two things I know are missing and haven't solved: **failure and dispute semantics** (what happens when an order is created, inventory reservation fails, payment already cleared, and the network response never arrives — the classic distributed-systems problem of retries, idempotency, partial failure, and reconciliation, which this architecture doesn't make disappear), and **discovery trust** (how does Company A know it actually reached Company B's real agent, rather than an impersonation — which points back to identity infrastructure this document mostly assumes rather than solves).

---

## 8. Operational Trace and Audit Receipts

### 8.1 Operational Trace Sequence

The lifecycle of an interaction contract from capability discovery to execution and audit logging proceeds as follows:

```text
Buyer Agent              Supplier B2B Agent           Trust Gateway              Isolated Executor
     │                           │                          │                            │
     │ 1. Discover Capabilities  │                          │                            │
     ├──────────────────────────►│                          │                            │
     │ 2. Propose Contract Draft │                          │                            │
     ├──────────────────────────►│                          │                            │
     │ 3. Counter / Accept       │                          │                            │
     │◄──────────────────────────┤                          │                            │
     │ 4. Mutual Ed25519 Signing │                          │                            │
     │◄═════════════════════════►│                          │                            │
     │ 5. Activate Contract      │                          │                            │
     ├───────────────────────────┼─────────────────────────►│                            │
     │                           │   Validate signatures,   │                            │
     │                           │   mark state = ACTIVE    │                            │
     │ 6. Propose Action         │                          │                            │
     ├───────────────────────────┼─────────────────────────►│                            │
     │                           │   Check limits & velocity│                            │
     │                           │   Mint ExecutionGrant    │                            │
     │                           │◄─────────────────────────┤                            │
     │ 7. Invoke Action          │                          │                            │
     │    (Grant + Arguments)    │                          │                            │
     ├───────────────────────────┼──────────────────────────┼───────────────────────────►│
     │                           │                          │    Verify Grant signature  │
     │                           │                          │    Verify input_hash match │
     │                           │                          │    Execute side effect     │
     │ 8. Sealed Audit Receipt logged to append-only stream │◄───────────────────────────┤
     │◄─────────────────────────────────────────────────────┼────────────────────────────┤
```

### 8.2 Audit Receipts and Dispute Resolution

Every completed execution produces a signed audit receipt — not a mutable log row, but an append-only record — linking the grant, the contract, the exact input and output hashes, the parties' DIDs, and (where relevant) financial and ERP reconciliation references:

```json
{
  "receipt_id": "rcpt_0191c7be-4010-7000-85f2-9a81e3400088",
  "grant_jti": "grant_0191c7b5-22a4-7000-91c2-3e817c200042",
  "contract_id": "ctr_0191c7a4-82a1-7000-84c1-6e792c300001",
  "contract_hash": "sha256:7f83b1657ff1fc53b92dc18148a1d65dfc2d4b1fa3d677284addd200126d9069",
  "tool_id": "io.company.orders.create@v1",
  "input_hash": "sha256:d59b207559e355c70752b047a0640df14541bfd6e3be4ff28e67a48d88e6de02",
  "output_hash": "sha256:4a6f20e791b8d23e59048a16db32ec840134f0d2c9498b3f462f7902d2aa86c5",
  "parties": {
    "buyer_did": "did:web:buyer.corp.example",
    "supplier_did": "did:web:supplier.logistics.example"
  },
  "execution_outcome": { "status": "SUCCESS", "duration_ms": 142 },
  "timestamps": {
    "grant_issued_at": "2026-09-30T14:20:00.000Z",
    "executed_at": "2026-09-30T14:20:00.142Z"
  }
}
```

The intended dispute-resolution story: if Company A claims a transaction was unauthorized, or Company B claims goods were never reserved, the chain from receipt → grant → contract → mutual signatures should settle it — each link is a hash comparison, not a matter of anyone's word. 🟡 *The receipt schema and hash chain are implemented; I haven't run this through an actual disputed-transaction scenario end to end.*

---

## 9. How This Relates to Other Work

A quick, honest comparison — this is my own understanding, not something I've verified with the teams involved, and it will likely be out of date by the time you read it.

- **MCP** is about an agent talking to tools and resources. It's not trying to solve what I'm describing here, and NICP could plausibly sit on top of an MCP-connected agent rather than replace anything about MCP.
- **A2A** is about agent-to-agent discovery, capability exchange, and task handoff. I think of NICP as a layer above A2A: assume A2A handles how two agents talk to each other; NICP is about what relationship they're allowed to establish and what stops either side from acting outside it.
- **Payment-network agent protocols** (Visa/Mastercard agent-payment initiatives) solve credential tokenization and payment authorization specifically — a narrower, adjacent problem to the broader capability-and-constraint negotiation this project is trying to address.
- **Anthropic's Claude Commerce Agents** is the most directly relevant reference point, and I've described how I read it in §1.3: it validates that merchants want the agent on their own side of the boundary, and it deliberately stops short of granting execution authority — which is the gap this project is trying to fill for cases that need to go further than recommend-and-handoff.

I'm not trying to compete with any of these, and I'd rather be wrong about how NICP fits next to them than overstate the fit.

---

## 10. Where This Goes Next

Concretely, and modestly:

- Keep open-sourcing the pieces that are actually stable — the contract canonicalization/hashing engine and the state machine are the best candidates right now.
- Get real, external scrutiny on the threat model and the invariants list in §7 before making any stronger claims about them.
- Actually build the pieces marked ⚪ in this document before describing them as anything more than intentions.
- If and when there's working code and outside interest behind a specific piece of this (the execution-grant binding pattern seems like the strongest candidate), look at whether an informal write-up to a relevant IETF or OpenID working group mailing list makes sense — as a discussion contribution, not a unilateral draft submission claiming priority.

I'd rather this document age well than sound impressive today.

---

## 11. References

1. RFC 8785 — JSON Canonicalization Scheme (JCS)
2. RFC 8949 — Concise Binary Object Representation (CBOR)
3. W3C Decentralized Identifiers (DIDs) v1.0
4. W3C Verifiable Credentials Data Model v2.0
5. UCAN Specification v0.10
6. RFC 7519 — JSON Web Token (JWT)
7. RFC 8032 — EdDSA
8. Anthropic, "Claude Commerce Agents" reference blueprint, released September 2, 2026
9. Public reporting on OpenAI's ChatGPT Instant Checkout launch (September 2025) and retirement (March 2026) — CNBC, The Information, Forrester
