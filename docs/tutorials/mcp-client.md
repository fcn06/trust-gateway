# Tutorial: Connect an MCP Client

> **Status:** This tutorial is a work in progress. The MCP transport is functional but the integration guide is being written.

This tutorial will show how to connect MCP-compatible clients (Claude Desktop, Cursor, or custom LLM agents) to Trust Gateway's governed tool surface.

---

## Overview

Trust Gateway exposes an MCP (Model Context Protocol) server that allows AI clients to:

1. **Discover governed tools** via `tools/list` — each tool is policy-gated
2. **Call tools** via `tools/call` — every call goes through the policy engine before execution
3. **Receive governed results** — output is scrubbed for PII before being returned

### MCP Endpoints

| Endpoint | Protocol |
|---|---|
| `GET /v1/mcp/sse` | Server-Sent Events transport |
| `POST /v1/mcp/messages` | Streamable HTTP transport |

---

## Prerequisites

- A running Trust Gateway instance with MCP enabled
- An MCP-compatible client

---

## Configuration

<!-- TODO: Add Claude Desktop MCP config example -->
<!-- TODO: Add Cursor MCP config example -->
<!-- TODO: Add custom client example -->

*This section is under development. See [`docs/concepts/ARCHITECTURE.md`](../concepts/ARCHITECTURE.md) for the current MCP integration architecture.*

---

## Next Steps

- [REST/curl tutorial](rest-curl-agent.md) — a fully functional tutorial using the REST API
- [Visual Guide](../concepts/VISUAL_GUIDE.md) — architecture overview including MCP transport
