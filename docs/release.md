# AIW 1.0.0 release and installation

AIW 1.0.0 is Apache-2.0 licensed and distributed only through GitHub Releases.
Crates.io publication is disabled. AIW has no runtime dependency on a model,
account, service, server, network connection, or consumer project.

## Supported platforms

| Platform | Rust toolchain | Personal-skill installer | CI validation |
| --- | --- | --- | --- |
| Linux | Rust 1.93 or newer | `scripts/install-user.sh` | format, Clippy, tests, release build, installer syntax |
| macOS | Rust 1.93 or newer | `scripts/install-user.sh` | format, Clippy, tests, release build, installer syntax |
| Windows | Rust 1.93 or newer | `scripts/install-user.ps1` | format, Clippy, tests, release build, PowerShell parser |

The supported agent adapters are Codex and Claude. Their installed skills and
hooks are noncanonical views over `.ai/state.json`, `.ai/policy.md`, and
`.ai/decisions/`.

## Verify and install a release

For release version `VERSION`, download these assets from the matching `vVERSION`
GitHub Release into one directory:

```text
aiw-VERSION-source.tar.gz
SHA256SUMS
RELEASE.txt
```

`RELEASE.txt` records the tag and exact commit used to build the source archive.
The archive checksum is the integrity boundary; verify it before extracting.

On Linux:

```sh
sha256sum -c SHA256SUMS
tar -xzf aiw-VERSION-source.tar.gz
cd aiw-VERSION
./scripts/install-user.sh
aiw --version
aiw doctor
```

On macOS, use `shasum` in place of `sha256sum`:

```sh
shasum -a 256 -c SHA256SUMS
tar -xzf aiw-VERSION-source.tar.gz
cd aiw-VERSION
./scripts/install-user.sh
aiw --version
aiw doctor
```

On Windows, calculate the SHA-256 value before extracting. It must equal the
hash for `aiw-VERSION-source.tar.gz` in `SHA256SUMS`:

```powershell
Get-FileHash .\aiw-VERSION-source.tar.gz -Algorithm SHA256
tar -xzf .\aiw-VERSION-source.tar.gz
Set-Location .\aiw-VERSION
powershell -ExecutionPolicy Bypass -File .\scripts\install-user.ps1
aiw --version
aiw doctor
```

The installers build from the included source using `Cargo.lock`, install the CLI
with `cargo install --path . --locked --force`, and install the same
`aiw-workspace` skill for Codex and Claude. `--force` is also the update path:
repeat the verification steps for a newer release, run the matching installer,
then check `aiw --version` and `aiw doctor`. The installed CLI operates offline;
Rust may need to fetch uncached build dependencies while installing a source
release.

To independently repeat the release validation after extraction, run:

```sh
./scripts/verify-release.sh --tag vVERSION --directory ..
```

This verifies `SHA256SUMS`, checks that `RELEASE.txt` names the requested tag, and
rebuilds the extracted source with `cargo build --release --locked`.

## Create release assets locally

An authorized releaser first commits the release, creates `vVERSION` on that exact
commit, and checks out the tag without uncommitted or untracked files. The source
archive is deterministic for a given Git tag because `git archive` reads only the
tagged tree and `gzip -n` omits compression timestamps.

```sh
scripts/package-release.sh --tag vVERSION --output-dir release
scripts/verify-release.sh --tag vVERSION --directory release
```

The commands produce and verify `aiw-VERSION-source.tar.gz`, `SHA256SUMS`, and
`RELEASE.txt`. Tag pushes run the same package-and-verify path in CI, retain the
three files as a workflow artifact, and publish them to the matching GitHub
Release. Publishing is performed only after the same workflow has verified the
archive and locked rebuild.
