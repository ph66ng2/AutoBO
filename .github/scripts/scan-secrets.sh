#!/usr/bin/env bash
# Falha se o índice versionar segredo. Não imprime o conteúdo encontrado.
set -euo pipefail

tracked_env=$(git ls-files | grep -E '(^|/)\.env$' || true)
if [[ -n "$tracked_env" ]]; then
  printf 'Arquivo .env versionado.\n' >&2
  exit 1
fi

if git grep -I -q -E 'BEGIN (RSA |OPENSSH |EC )?PRIVATE KEY'; then
  printf 'Chave privada versionada.\n' >&2
  exit 1
fi

if git grep -I -q -E 'sb_secret_|sk_live_'; then
  printf 'Segredo privilegiado versionado.\n' >&2
  exit 1
fi
