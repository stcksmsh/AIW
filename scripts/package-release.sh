#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: scripts/package-release.sh --tag vVERSION --output-dir DIRECTORY

Create the deterministic AIW source-release archive and SHA-256 manifest from
the exact, clean commit named by a release tag. This command never publishes.
EOF
}

tag=''
output_dir=''
while [ "$#" -gt 0 ]; do
  case "$1" in
    --tag)
      tag=${2:?--tag requires a value}
      shift 2
      ;;
    --output-dir)
      output_dir=${2:?--output-dir requires a value}
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

[ -n "$tag" ] && [ -n "$output_dir" ] || { usage >&2; exit 2; }

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"
version=$(sed -nE 's/^version = "([^"]+)"$/\1/p' Cargo.toml | head -n 1)
[ "$tag" = "v$version" ] || {
  echo "release tag $tag does not match Cargo.toml version $version" >&2
  exit 1
}

tag_commit=$(git rev-parse --verify "refs/tags/$tag^{commit}")
head_commit=$(git rev-parse HEAD)
[ "$tag_commit" = "$head_commit" ] || {
  echo "release tag $tag must name HEAD" >&2
  exit 1
}

[ -z "$(git status --porcelain --untracked-files=all)" ] || {
  echo "release checkout must be clean" >&2
  exit 1
}

mkdir -p "$output_dir"
[ -z "$(find "$output_dir" -mindepth 1 -maxdepth 1 -print -quit)" ] || {
  echo "release output directory must be empty: $output_dir" >&2
  exit 1
}

archive="aiw-$version-source.tar.gz"
git archive --format=tar --prefix="aiw-$version/" "$tag" | gzip -n > "$output_dir/$archive"
printf 'tag=%s\ncommit=%s\n' "$tag" "$tag_commit" > "$output_dir/RELEASE.txt"

if command -v sha256sum >/dev/null 2>&1; then
  (cd "$output_dir" && sha256sum "$archive" > SHA256SUMS)
elif command -v shasum >/dev/null 2>&1; then
  (cd "$output_dir" && shasum -a 256 "$archive" > SHA256SUMS)
else
  echo "need sha256sum or shasum to write SHA256SUMS" >&2
  exit 1
fi

echo "Created $output_dir/$archive, RELEASE.txt, and SHA256SUMS"
