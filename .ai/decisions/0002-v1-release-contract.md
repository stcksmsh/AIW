# ADR 0002: AIW v1 release contract

Status: accepted for the v1 release plan (2026-09-11).

## Decisions

- AIW v1 remains a local-first deterministic CLI and workspace format. It has no
  runtime dependency on a model, account, server, network connection, consumer
  project, or other tool.
- The supported CLI platform target is Linux, macOS, and Windows. Codex and Claude
  are supported adapter targets, but their instructions, skills, and hooks remain
  noncanonical views over the same workspace state.
- Distribution will use GitHub Releases with versioned source or artifacts and
  SHA-256 checksums. Crates.io publication remains disabled. The release must
  document installation, verification, and update paths before it is published.
- The project is licensed under Apache-2.0.
- The canonical version-1 workspace schema and documented semantics are the
  interoperability contract. Unsupported canonical versions fail closed. Text
  views, derived-index provenance, and disposable runtime artifacts are not frozen
  public protocols.

## Non-goals

V1 does not add model-backed memory, embeddings, provider SDKs, services, remote
state, automatic worker orchestration, automatic Git mutation, or a dependency on
another project. Structural retrieval remains a later local capability unless
release evidence proves it is necessary for reliable recovery.

## Consequences

Release work must supply a Windows installation path in addition to the existing
Unix installer, preserve offline core operation after installation, and produce
checksummed artifacts from a verified tagged commit. AIW's task contracts and
release evidence must remain independent from consumer repositories.
