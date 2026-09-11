#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: scripts/verify-release.sh --tag vVERSION --directory DIRECTORY

Verify a locally downloaded AIW source release and rebuild it with the locked
dependency graph. This command never downloads or publishes release material.
EOF
}

tag=''
release_dir=''
while [ "$#" -gt 0 ]; do
  case "$1" in
    --tag)
      tag=${2:?--tag requires a value}
      shift 2
      ;;
    --directory)
      release_dir=${2:?--directory requires a value}
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      usage >&2
      exit 2
      ;;
  esac
done

[ -n "$tag" ] && [ -n "$release_dir" ] || { usage >&2; exit 2; }
version=${tag#v}
[ "$tag" = "v$version" ] && [ -n "$version" ] || {
  echo "release tag must be vVERSION" >&2
  exit 2
}

release_dir=$(cd "$release_dir" && pwd)
archive="aiw-$version-source.tar.gz"
[ -f "$release_dir/$archive" ] && [ -f "$release_dir/SHA256SUMS" ] && [ -f "$release_dir/RELEASE.txt" ] || {
  echo "release directory is missing the source archive, SHA256SUMS, or RELEASE.txt" >&2
  exit 1
}
grep -Fx "tag=$tag" "$release_dir/RELEASE.txt" >/dev/null || {
  echo "RELEASE.txt does not describe $tag" >&2
  exit 1
}

if command -v sha256sum >/dev/null 2>&1; then
  (cd "$release_dir" && sha256sum -c SHA256SUMS)
elif command -v shasum >/dev/null 2>&1; then
  (cd "$release_dir" && shasum -a 256 -c SHA256SUMS)
else
  echo "need sha256sum or shasum to verify SHA256SUMS" >&2
  exit 1
fi

temporary=$(mktemp -d "${TMPDIR:-/tmp}/aiw-release-verify.XXXXXX")
trap 'rm -rf "$temporary"' EXIT
tar -xzf "$release_dir/$archive" -C "$temporary"
source_root="$temporary/aiw-$version"
[ -f "$source_root/Cargo.toml" ] && [ -f "$source_root/Cargo.lock" ] || {
  echo "source archive is missing Cargo metadata" >&2
  exit 1
}

(cd "$source_root" && cargo build --release --locked && test "$(target/release/aiw --version)" = "aiw $version")
echo "Verified $archive and rebuilt aiw $version from its locked source"
