---
status: completed
priority: p1
issue_id: "002"
tags: [security, code-review, gui, tauri]
dependencies: []
---

# Content Security Policy Disabled in Tauri GUI

## Problem Statement

The Tauri GUI configuration at `crates/gat-gui/src-tauri/tauri.conf.json:21` has Content Security Policy (CSP) set to `null`, disabling XSS protections.

**Why it matters:** Without CSP, any XSS vulnerability in the frontend can be fully exploited - attackers can load scripts from any domain, execute inline scripts, and make requests to any origin.

## Resolution

Implemented strict CSP with minimal required exceptions:

```json
{
  "security": {
    "csp": "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline' https://cdn.jsdelivr.net; font-src 'self' https://cdn.jsdelivr.net; img-src 'self' data: blob:; connect-src ipc: http://ipc.localhost"
  }
}
```

**Policy breakdown:**
- `default-src 'self'` - Only load resources from same origin by default
- `script-src 'self'` - Only allow scripts from the app bundle (no inline, no external)
- `style-src 'self' 'unsafe-inline' https://cdn.jsdelivr.net` - Allow local styles, inline styles (required by Svelte), and KaTeX CSS from jsdelivr
- `font-src 'self' https://cdn.jsdelivr.net` - Allow local fonts and KaTeX fonts
- `img-src 'self' data: blob:` - Allow local images, data URIs (for SVG export), and blob URLs
- `connect-src ipc: http://ipc.localhost` - Allow Tauri IPC communication

**External resource allowlisted:**
- `cdn.jsdelivr.net` - KaTeX math rendering CSS (used in EducationDrawer.svelte)

## Acceptance Criteria

- [x] CSP enabled with restrictive policy
- [x] External resources explicitly allowlisted (jsdelivr for KaTeX)
- [x] GUI compiles successfully

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2025-12-06 | Finding identified | CSP null means no XSS protection |
| 2025-12-06 | Implemented CSP | Analyzed frontend for required external resources |
