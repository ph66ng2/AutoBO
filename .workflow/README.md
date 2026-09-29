# Workflow AutoBO

`workflow.json` preserva tickets, status e dependências locais do AutoBO. A branch principal é `origin/main`; PRs apontam para `main` e somente uma pessoa faz merge.

## Papel no marco atual

O primeiro marco de produto é [AutoOS Fiscal PROD BMITAG](https://github.com/ph66ng2/AutoPlatform/blob/main/.workflow/FISCAL_PROD_ROADMAP.md). AutoBO sai do caminho crítico **como aplicação**: permanece como código doador, fonte de regras e histórico de boleto e importação NF-e. Backend fiscal fica no AutoPlatform; fila, revisão, estados e documentos ficam no AutoOS. Não iniciar uma UI Fiscal nova no AutoBO para migrá-la depois.

O objeto `roadmap` do catálogo registra `deferredIds` e `superseded` com destinos explícitos. Essas classificações não são `status`: tickets antigos não viram `merged` ou `blocked` apenas porque a estratégia mudou. O planejador ainda calcula elegibilidade por `blockedBy` local; antes de iniciar qualquer onda, confira também a prioridade no `roadmap`. Tickets já concluídos, inclusive governança, QA, arquitetura fiscal, importação e Auth, permanecem como histórico e código existente.

## Fronteiras

- `autobo_nfes` continua a representar notas **importadas**; não misturar histórico importado com documentos emitidos no AutoPlatform.
- AutoBO não ganha migrations compartilhadas, providers fiscais privilegiados, segredos de emissão nem segunda autoridade de estoque para este produto.
- Módulos de boleto, importação e regras úteis serão avaliados para absorção posterior no AutoOS por tickets próprios. `BO-MIG-001` fica adiado até existir plano concreto.
- Relações externas descritas em tickets espelho continuam exigindo evidência e confirmação humana. Não são satisfeitas automaticamente pelo nome de ticket parecido.

## Comandos e execução

```bash
.workflow/scripts/waves.sh plan .workflow/workflow.json
```

Um ticket só entra em onda quando seus bloqueadores locais estão `merged`; `blocked` continua sem onda. O planejador atual pode falhar sem ondas internas com `WAVE: unbound variable`; corrigir em PR próprio antes de depender desse caso. Uma onda tecnicamente livre não autoriza executar tickets adiados ou substituídos. Nenhum merge, release ou operação real é automático.
