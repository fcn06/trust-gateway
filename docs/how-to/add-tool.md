# How-To: Register a Custom Executor Tool

> **Status:** This guide is a work in progress.

This guide will explain how to implement a custom executor tool and register it with Trust Gateway.

---

## Overview

Executor tools are the functions that Trust Gateway authorizes agents to call. Each tool:

1. Is registered with a unique name
2. Has an execution profile (`native-tool`, `connector`, `vp`)
3. Implements the `Executor` trait from `trust-executor-sdk`
4. Receives a verified `GrantedAction` with cryptographically bound arguments

---

## The Executor Trait

```rust
use trust_executor_sdk::Executor;

#[async_trait]
pub trait Executor: Send + Sync {
    async fn execute(&self, action: GrantedAction) -> Result<ExecutionResult, ExecutorError>;
}
```

---

## Steps

<!-- TODO: Step-by-step guide for implementing a custom executor -->
<!-- TODO: Example of registering with the executor_host -->
<!-- TODO: Testing instructions -->

*This guide is under development. See the [Rust executor example](../../examples/rust-executor/) and the [`trust-executor-sdk` crate](../../crates/trust-executor-sdk/) for reference.*

---

## Next Steps

- [Write a policy](write-policy.md) — configure governance rules for your tool
- [Require approval](require-approval.md) — add human-in-the-loop for sensitive tools
