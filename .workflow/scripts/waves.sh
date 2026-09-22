#!/usr/bin/env bash
# Planejador de ondas do workflow local.
# Uso: .workflow/scripts/waves.sh plan .workflow/workflow.json
set -euo pipefail

if [[ $# -ne 2 || "$1" != "plan" ]]; then
  printf 'Uso: %s plan workflow.json\n' "${0##*/}" >&2
  exit 64
fi

workflow="$2"
[[ -f "$workflow" ]] || { printf 'Arquivo não encontrado: %s\n' "$workflow" >&2; exit 66; }
command -v jq >/dev/null || { printf 'Dependência ausente: jq\n' >&2; exit 69; }

jq -e '
  (.project | type == "string") and
  (.baseBranch | type == "string") and
  (.tickets | type == "array") and
  (all(.tickets[];
    (.id | type == "string" and length > 0) and
    (.title | type == "string") and
    (.status | type == "string") and
    (.blockedBy | type == "array" and all(.[]; type == "string"))
  ))
' "$workflow" >/dev/null || {
  printf 'Formato inválido: project, baseBranch e tickets são obrigatórios; cada ticket precisa de id, title, status e blockedBy.\n' >&2
  exit 65
}

duplicate_ids=$(jq -r '[.tickets[].id] | group_by(.) | map(select(length > 1) | .[0]) | join(",")' "$workflow")
[[ -z "$duplicate_ids" ]] || { printf 'IDs duplicados: %s\n' "$duplicate_ids" >&2; exit 65; }

missing=$(jq -r '
  [.tickets[].id] as $ids |
  [.tickets[].blockedBy[]? | select(. as $id | ($ids | index($id) | not))] |
  unique | join(",")
' "$workflow")
[[ -z "$missing" ]] || { printf 'Dependências ausentes: %s\n' "$missing" >&2; exit 65; }

mapfile -t rows < <(jq -r '.tickets[] | [.id, .status, .title, (.blockedBy | join(","))] | @tsv' "$workflow")

declare -A STATUS TITLE BLOCKERS INDEG
declare -a IDS
declare -A CHILDREN

for row in "${rows[@]}"; do
  IFS=$'\t' read -r id status title blockers <<<"$row"
  IDS+=("$id")
  STATUS["$id"]="$status"
  TITLE["$id"]="$title"
  BLOCKERS["$id"]="$blockers"
  INDEG["$id"]=0
done

for id in "${IDS[@]}"; do
  [[ -z "${BLOCKERS[$id]}" ]] && continue
  IFS=',' read -ra blockers <<<"${BLOCKERS[$id]}"
  for blocker in "${blockers[@]}"; do
    INDEG["$id"]=$((INDEG["$id"] + 1))
    CHILDREN["$blocker"]+="${id}"$'\n'
  done
done

queue=()
for id in "${IDS[@]}"; do
  [[ "${INDEG[$id]}" -eq 0 ]] && queue+=("$id")
done

processed=0
head=0
while [[ "$head" -lt "${#queue[@]}" ]]; do
  id="${queue[$head]}"
  head=$((head + 1))
  processed=$((processed + 1))
  [[ -z "${CHILDREN[$id]:-}" ]] && continue
  while IFS= read -r child; do
    [[ -z "$child" ]] && continue
    INDEG["$child"]=$((INDEG["$child"] - 1))
    [[ "${INDEG[$child]}" -eq 0 ]] && queue+=("$child")
  done <<<"${CHILDREN[$id]}"
done

if [[ "$processed" -ne "${#IDS[@]}" ]]; then
  printf 'Ciclo de dependências detectado. Planejamento interrompido.\n' >&2
  exit 65
fi

declare -A WAVE
pending=("${IDS[@]}")
progress=1
while [[ "$progress" -eq 1 ]]; do
  progress=0
  next=()
  for id in "${pending[@]}"; do
    if [[ "${STATUS[$id]}" == "merged" || "${STATUS[$id]}" == "blocked" ]]; then
      next+=("$id")
      continue
    fi
    ready=1
    max_wave=0
    if [[ -n "${BLOCKERS[$id]}" ]]; then
      IFS=',' read -ra blockers <<<"${BLOCKERS[$id]}"
      for blocker in "${blockers[@]}"; do
        if [[ "${STATUS[$blocker]}" == "merged" ]]; then
          continue
        fi
        if [[ -n "${WAVE[$blocker]:-}" ]]; then
          [[ "${WAVE[$blocker]}" -gt "$max_wave" ]] && max_wave="${WAVE[$blocker]}"
          continue
        fi
        ready=0
      done
    fi
    if [[ "$ready" -eq 1 ]]; then
      WAVE["$id"]=$((max_wave + 1))
      progress=1
    else
      next+=("$id")
    fi
  done
  pending=("${next[@]}")
done

project=$(jq -r '.project' "$workflow")
base=$(jq -r '.baseBranch' "$workflow")
printf 'Projeto: %s\n' "$project"
printf 'Base: %s\n\n' "$base"

if [[ "${#WAVE[@]}" -eq 0 ]]; then
  printf 'Nenhuma onda interna.\n'
else
  max_seen=0
  for id in "${!WAVE[@]}"; do
    [[ "${WAVE[$id]}" -gt "$max_seen" ]] && max_seen="${WAVE[$id]}"
  done
  wave=1
  while [[ "$wave" -le "$max_seen" ]]; do
    printf 'Onda %s\n' "$wave"
    for id in "${IDS[@]}"; do
      [[ "${WAVE[$id]:-}" == "$wave" ]] || continue
      note=""
      if [[ -n "${BLOCKERS[$id]}" && "${BLOCKERS[$id]}" == *,* ]]; then
        note=$(printf '  [%s bloqueadores: %s]' "$(awk -F, '{print NF}' <<<"${BLOCKERS[$id]}")" "${BLOCKERS[$id]//,/, }")
      fi
      released=""
      if [[ "$wave" -eq 1 ]]; then
        released="  [liberado]"
      fi
      printf '  %s  %s%s%s\n' "$id" "${TITLE[$id]}" "$released" "$note"
    done
    printf '\n'
    wave=$((wave + 1))
  done
fi

printf 'Sem onda\n'
any_unwaved=0
for id in "${IDS[@]}"; do
  [[ -n "${WAVE[$id]:-}" || "${STATUS[$id]}" == "merged" ]] && continue
  any_unwaved=1
  reason="status ${STATUS[$id]}"
  if [[ "${STATUS[$id]}" != "blocked" && -n "${BLOCKERS[$id]}" ]]; then
    reason="depende de ${BLOCKERS[$id]//,/, }"
  elif [[ -n "${BLOCKERS[$id]}" ]]; then
    reason="bloqueio externo; depende de ${BLOCKERS[$id]//,/, }"
  fi
  printf '  %s  %s  (%s)\n' "$id" "${TITLE[$id]}" "$reason"
done
[[ "$any_unwaved" -eq 1 ]] || printf '  nenhum\n'
printf '\n'

printf 'Dois ou mais bloqueadores\n'
any_multi=0
for id in "${IDS[@]}"; do
  [[ "${BLOCKERS[$id]}" == *,* ]] || continue
  any_multi=1
  printf '  %s  %s\n' "$id" "${BLOCKERS[$id]//,/, }"
done
[[ "$any_multi" -eq 1 ]] || printf '  nenhum\n'
