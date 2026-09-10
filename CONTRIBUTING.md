# Contributing

Start with `aiw load` (or `cargo run -- load`) and the relevant operation contract.
Keep source edits scoped and persist meaningful discoveries in `.ai/`.

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --release --locked
```

Integration tests use temporary repositories and subprocesses. Git must be
installed for Git/worktree tests. Unix-only tests cover process groups and symlinks;
CI also exercises the portable suite on macOS and Windows. No provider credentials
or network calls are needed in tests. Rust dependencies need downloading once.

After changing the state model, regenerate `schemas/workspace-v1.schema.json`
with `cargo run -- schema`, inspect the diff and update the specification. Do not
silently change version-1 semantics after release. Prefer tests of observable
contracts and failure recovery over tests mirroring private functions.

No software license has been selected by the owner. An explicit permissive
license such as MIT or Apache-2.0 is worth considering before public distribution,
but that decision is reserved for the owner. `publish = false` prevents accidental
crates.io publication. CI builds and tests; it does not publish or release.
