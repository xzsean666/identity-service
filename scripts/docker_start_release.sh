#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if [[ ! -x release/identity-service ]]; then
  printf 'missing executable release/identity-service\n' >&2
  printf 'run ./scripts/build_release.sh before starting the release Docker image.\n' >&2
  exit 1
fi

if [[ ! -f secrets/jwt_private.pem || ! -f secrets/jwt_public.pem ]]; then
  printf 'missing JWT key files under secrets/\n' >&2
  printf 'generate them with the OpenSSL commands documented in docs/BUILD.md.\n' >&2
  exit 1
fi

export IDENTITY_DOCKER_USER="${IDENTITY_DOCKER_USER:-$(id -u):$(id -g)}"

exec docker compose -f docker-compose.release.yml up --build "$@"
