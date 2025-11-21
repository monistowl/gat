#!/usr/bin/env bash
set -euo pipefail

# Regenerate all docs before exposing them to MCP clients.
cargo xtask --features docs doc all

# Start the MCP docs server so agents can discover the updated tree.
gat-mcp-docs --docs docs --addr 127.0.0.1:4321
