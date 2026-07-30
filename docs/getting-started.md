# Getting Started

Welcome to Trust Gateway! This page helps you find the right starting point.

## I want to...

### See what Trust Gateway does (2 minutes)

Run the standalone demo — no NATS, no Docker, no external dependencies beyond Rust:

```bash
make quickstart
```

See the [README](../README.md) for expected output and a walkthrough of what happens.

### Integrate my agent via REST (10 minutes)

Follow the [REST/curl tutorial](tutorials/rest-curl-agent.md) — send `curl` requests to a running gateway and observe policy decisions, grant issuance, and execution.

### Integrate my agent via Python (10 minutes)

See the [Python agent example](../examples/python-agent/) — a complete Python client that proposes actions and receives grants through the REST API.

### Connect an MCP client (15 minutes)

Follow the [MCP client tutorial](tutorials/mcp-client.md) — connect Claude Desktop, Cursor, or a custom MCP client to discover governed tools.

### Write a custom policy (10 minutes)

Follow the [policy authoring guide](how-to/write-policy.md) — learn how `policy.toml` rules work and create your own governance rules.

### Build and contribute to the Rust codebase (15 minutes)

Follow the [contributor guide](QUICKSTART.md) — set up your environment, build, test, and understand the workspace.

### Understand the architecture (5 minutes)

Start with the [Visual Guide](concepts/VISUAL_GUIDE.md) for a diagram-driven overview, then read the [Architecture](concepts/ARCHITECTURE.md) doc for details.
