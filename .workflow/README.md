# Workflow AutoBO

`workflow.json` organiza a convergência AutoOS + AutoBO e as trilhas posteriores.

A branch principal é `origin/main`. O merge é humano: nenhum ticket faz merge, release ou alteração de produção sozinho. Enquanto `BO-GOV-001` não estiver `merged`, os demais tickets não geram branch, worktree nem pull request.

O marco `BO-SUITE-GATE-001` significa **Produto em Sincronia**: uma OS produz exatamente uma intenção financeira, o AutoBO publica o estado de volta e retries não duplicam dados. Depois desse gate:

- AutoOS segue para PowerSync, assinatura e fotos cloud.
- AutoBO segue para Sicredi e inicia fiscal por `BO-FISC-ADR-001` e `BO-FISC-CORE-001`; só então integra NFS-e (`BO-FISC-EXT-001`) e NF-e de mercadorias (`BO-FISC-NFE-001`).
- Ambos só mudam o contrato compartilhado por nova versão explícita.

O gate de sincronização não promete emissão de nota fiscal. Ele prova apenas a troca idempotente da intenção comercial e de seus estados. A emissão fiscal é um marco posterior: `autobo_nfes` continua sendo o registro de notas importadas, enquanto a emissão ganhará registro e máquina de estados próprios para não misturar documentos recebidos com documentos gerados pelo AutoBO.
