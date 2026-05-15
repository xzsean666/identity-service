#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PROJECT_DIR="$ROOT_DIR/frontend-test"
PROJECT_NAME="${CF_PAGES_PROJECT_NAME:-identity-service-frontend-test}"
BRANCH_NAME="${CF_PAGES_BRANCH:-${1:-}}"

if command -v wrangler >/dev/null 2>&1; then
  WRANGLER_CMD=(wrangler)
elif command -v npx >/dev/null 2>&1; then
  WRANGLER_CMD=(npx --yes wrangler)
else
  echo "wrangler or npx is required to deploy this Pages project." >&2
  exit 1
fi

cd "$PROJECT_DIR"

deploy_args=(pages deploy public --project-name "$PROJECT_NAME")
if [[ -n "$BRANCH_NAME" ]]; then
  deploy_args+=(--branch "$BRANCH_NAME")
fi

echo "Deploying Cloudflare Pages project '$PROJECT_NAME' from '$PROJECT_DIR/public'"
exec "${WRANGLER_CMD[@]}" "${deploy_args[@]}"
