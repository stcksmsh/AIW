# Changelog

## 0.2.0 — 2026-09-10

Adds portable Codex and Claude project skills, merged SessionStart/SubagentStart
hooks, opt-in strict Stop enforcement, a local user installer, and the `integrate`
command. Generated bootstrap blocks now stay at the start of vendor instruction
files so they remain visible under bounded instruction loading. Repeated Codex and
Claude integration now removes only AIW-owned command handlers, preserving
co-installed handlers even when they share an AIW hook group.

## 0.1.0 — 2026-09-10

Initial reference implementation: workspace schema 1, task DAG and atomic claims,
bounded load/handoff, Git discovery, incremental lexical indexing, captured command
execution, content-bound verification, Codex/Claude bootstrap adapters, schema
export and local diagnostics. See docs/dogfood.md for measured acceptance evidence.

CLI versions follow semantic versioning. Workspace schema versions evolve
separately; incompatible format changes require explicit migration.
