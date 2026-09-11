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

AIW is licensed under Apache-2.0. `publish = false` prevents accidental crates.io
publication; the approved v1 distribution channel is GitHub Releases with
checksummed source releases. CI builds and tests but does not publish or create a
release. Before an authorized release, create an exact `vVERSION` tag on a clean
commit, then run:

```sh
scripts/package-release.sh --tag vVERSION --output-dir release
scripts/verify-release.sh --tag vVERSION --directory release
```

The source archive, `SHA256SUMS`, and `RELEASE.txt` are the release assets. Attach
them only after the commands succeed; publication remains an owner action.
