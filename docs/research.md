# Interoperability research

Official documentation inspected on 2026-09-10. These are adapter design inputs,
not dependencies of the workspace format. No copied vendor prompt is canonical.

| Surface | Finding and v0.1 decision | Primary source |
| --- | --- | --- |
| Codex | Repository AGENTS.md participates in hierarchical instructions; an AGENTS.override.md can supersede it. Generate a short bootstrap block; doctor reports a root override. Do not modify user configuration. | [Codex instructions](https://learn.chatgpt.com/docs/agent-configuration/agents-md) |
| Claude Code | CLAUDE.md holds project guidance; skills support on-demand workflows and hooks run lifecycle actions. Generate only a bootstrap block. No automatic execution hooks or machine-local auto-memory dependency. | [Memory](https://code.claude.com/docs/en/memory), [extensions](https://code.claude.com/docs/en/features-overview), [hooks](https://code.claude.com/docs/en/hooks) |
| Gemini CLI | GEMINI.md is the default context file; configurable filenames and imports exist. A future adapter can point to AIW without changing state. | [Context files](https://geminicli.com/docs/cli/gemini-md/) |
| Cursor | Supports AGENTS.md and scoped rules in `.cursor/rules`. The neutral bootstrap may be useful; no first-class Cursor integration is claimed. | [Rules](https://prod.cursor.com/docs/rules) |
| OpenCode | Uses instruction files including AGENTS.md. Keep interoperability at the CLI/file layer. | [Rules](https://opencode.ai/docs/rules/) |
| Cline / Roo | Have their own project rules/custom instructions. Future small pointer adapters; avoid copying project memory into each format. | [Cline rules](https://docs.cline.bot/customization/cline-rules), [Roo instructions](https://roocodeinc.github.io/Roo-Code/features/custom-instructions/) |
| MCP | A tool integration protocol. A server would add process/protocol overhead to operations already callable from a shell. Defer. | [Specification](https://modelcontextprotocol.io/specification/2026-07-28) |
| LSP / rust-analyzer | Structured code intelligence is valuable for a future semantic adapter, but persistent server lifecycle and workspace setup are beyond the lexical slice. | [LSP](https://microsoft.github.io/language-server-protocol/), [rust-analyzer](https://rust-analyzer.github.io/book/) |
| tree-sitter | Incremental parsing could support syntactic structure with explicit parser provenance; no guessed symbols in v0.1. | [Introduction](https://tree-sitter.github.io/tree-sitter/) |
| SCIP / LSIF | Useful interchange for precomputed code intelligence; premature for this small local index. | [SCIP](https://github.com/scip-code/scip), [LSIF](https://microsoft.github.io/language-server-protocol/overviews/lsif/overview/) |
| Cargo / rustc / Clippy | Cargo's `compiler-message` wraps structured compiler diagnostics; parse primary spans, level and message. Ignore artifact chatter. Callers request JSON explicitly. | [Cargo external tools](https://doc.rust-lang.org/cargo/reference/external-tools.html) |
| SARIF | A diagnostic interchange candidate for future import/export. Official HTML fetch failed; the TC repository was accessible. No unsupported conformance claim. | [OASIS specification repository](https://github.com/oasis-tcs/sarif-spec) |
| JUnit XML / pytest | pytest can emit JUnit XML. A future file-result adapter can consume it; generic capture is sufficient for this slice. | [pytest output](https://docs.pytest.org/en/stable/how-to/output.html) |

Gradle, pytest, CMake/Make, npm/pnpm and Docker can all be invoked as literal argv
through `run` and task verification. v0.1 does **not** claim dedicated parsers for
them. `runner::collect` is the small normalized diagnostic boundary to extend;
new adapters must retain raw artifacts, bound output, and name their provenance.

No external documentation mirror is implemented. Consult project state, source,
installed documentation and local caches first, then targeted authoritative docs.
