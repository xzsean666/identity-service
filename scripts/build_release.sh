#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

package_name="identity-service"
release_dir="$repo_root/release"
temporary_dir="$(mktemp -d)"
release_staging_dir="$temporary_dir/release"

cp Cargo.toml "$temporary_dir/Cargo.toml.backup"
if [[ -f Cargo.lock ]]; then
  cp Cargo.lock "$temporary_dir/Cargo.lock.backup"
fi

build_finished="false"

cleanup() {
  local exit_code=$?

  if [[ "$build_finished" != "true" ]]; then
    cp "$temporary_dir/Cargo.toml.backup" Cargo.toml
    if [[ -f "$temporary_dir/Cargo.lock.backup" ]]; then
      cp "$temporary_dir/Cargo.lock.backup" Cargo.lock
    fi
  fi

  rm -rf "$temporary_dir"
  exit "$exit_code"
}

trap cleanup EXIT

read_package_version() {
  awk '
    $0 == "[package]" {
      in_package = 1
      next
    }

    in_package && /^\[/ {
      exit
    }

    in_package && /^version = / {
      gsub(/"/, "", $3)
      print $3
      exit
    }
  ' Cargo.toml
}

update_package_version() {
  local next_version="$1"
  local next_cargo_toml="$temporary_dir/Cargo.toml.next"

  awk -v next_version="$next_version" '
    BEGIN {
      in_package = 0
      updated = 0
    }

    $0 == "[package]" {
      in_package = 1
      print
      next
    }

    in_package && /^version = / && updated == 0 {
      print "version = \"" next_version "\""
      updated = 1
      next
    }

    in_package && /^\[/ {
      in_package = 0
    }

    {
      print
    }

    END {
      if (updated != 1) {
        exit 42
      }
    }
  ' Cargo.toml > "$next_cargo_toml"

  mv "$next_cargo_toml" Cargo.toml
}

current_version="$(read_package_version)"
if [[ ! "$current_version" =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)$ ]]; then
  printf 'Unsupported package version "%s". Expected MAJOR.MINOR.PATCH.\n' "$current_version" >&2
  exit 1
fi

major_version="${BASH_REMATCH[1]}"
minor_version="${BASH_REMATCH[2]}"
patch_version="${BASH_REMATCH[3]}"
next_patch_version=$((10#$patch_version + 1))
next_version="$major_version.$minor_version.$next_patch_version"

git_commit="$(git rev-parse --short HEAD 2>/dev/null || printf 'unknown')"
git_dirty_before_build="false"
if [[ -n "$(git status --short 2>/dev/null)" ]]; then
  git_dirty_before_build="true"
fi

update_package_version "$next_version"

cargo build --release --bins

target_root="${CARGO_TARGET_DIR:-$repo_root/target}"
if [[ "$target_root" != /* ]]; then
  target_root="$repo_root/$target_root"
fi
target_release_dir="$target_root/release"

mkdir -p "$release_staging_dir"

binaries=("identity-service" "migrate")
for binary_name in "${binaries[@]}"; do
  binary_path="$target_release_dir/$binary_name"
  if [[ ! -x "$binary_path" ]]; then
    printf 'Expected release binary not found: %s\n' "$binary_path" >&2
    exit 1
  fi

  cp "$binary_path" "$release_staging_dir/$binary_name"
done

printf '%s\n' "$next_version" > "$release_staging_dir/VERSION"

rustc_host="$(rustc -vV | awk -F': ' '/^host:/ { print $2 }')"
built_at_utc="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

{
  printf 'package=%s\n' "$package_name"
  printf 'version=%s\n' "$next_version"
  printf 'profile=release\n'
  printf 'git_commit=%s\n' "$git_commit"
  printf 'git_dirty_before_build=%s\n' "$git_dirty_before_build"
  printf 'built_at_utc=%s\n' "$built_at_utc"
  printf 'rustc_host=%s\n' "$rustc_host"
  printf 'binaries=%s\n' "${binaries[*]}"
} > "$release_staging_dir/BUILD_INFO"

if command -v sha256sum >/dev/null 2>&1; then
  (
    cd "$release_staging_dir"
    sha256sum "${binaries[@]}" VERSION BUILD_INFO > SHA256SUMS
  )
fi

rm -rf "$release_dir"
mv "$release_staging_dir" "$release_dir"

build_finished="true"

printf 'Release build complete.\n'
printf 'Version: %s -> %s\n' "$current_version" "$next_version"
printf 'Output: %s\n' "$release_dir"
