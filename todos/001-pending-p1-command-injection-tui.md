---
status: completed
priority: p1
issue_id: "001"
tags: [security, code-review, tui]
dependencies: []
---

# Command Injection Vulnerability in TUI Command Runner

## Problem Statement

The TUI command runner at `crates/gat-tui/src/command_runner.rs:28-51` executes user-controlled commands without validation or sanitization, creating a command injection vulnerability.

**Why it matters:** If an attacker can control the command vector passed to `spawn_command()`, they can execute arbitrary system commands.

## Resolution

Implemented command allowlisting with shell metacharacter validation:

1. **Command Allowlist**: Only `gat` and `echo` commands permitted
   - `gat` - the main CLI tool (all TUI modals use this)
   - `echo` - used for dry-run mode output

2. **Shell Metacharacter Blocking**: Arguments cannot contain:
   - `;`, `|`, `&` - command chaining
   - `$`, `` ` `` - variable/command substitution
   - `(`, `)`, `{`, `}` - subshells
   - `<`, `>` - redirections
   - `\n`, `\r` - newlines

3. **Unit Tests Added**: 6 tests covering:
   - Valid gat commands
   - Valid echo commands
   - Rejection of arbitrary commands (sh, rm, cat)
   - Rejection of shell metacharacters
   - Rejection of empty commands
   - Valid gat arguments with flags and files

**Key code:**
```rust
fn validate_command(cmd: &[String]) -> Result<()> {
    // Only allow 'gat' or 'echo' commands
    let binary_name = Path::new(&cmd[0]).file_name()...;
    if binary_name != "gat" && binary_name != "echo" {
        return Err(anyhow!("Command not allowed"));
    }
    // Block shell metacharacters in arguments
    for arg in cmd.iter().skip(1) {
        if arg.chars().any(|c| SHELL_METACHARACTERS.contains(&c)) {
            return Err(anyhow!("shell metacharacters not allowed"));
        }
    }
    Ok(())
}
```

## Acceptance Criteria

- [x] Commands validated against allowlist before execution
- [x] Shell metacharacters blocked in arguments
- [x] Unit tests for blocked commands
- [x] Integration test for allowed commands

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2025-12-06 | Finding identified | Security audit discovered unvalidated command execution |
| 2025-12-06 | Implemented fix | Allowlist + metacharacter blocking is defense in depth |
