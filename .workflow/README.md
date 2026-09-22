# Workflow AutoBO

`workflow.json` organiza o AutoBO standalone e, depois, sua integração com o AutoOS.

A branch principal é `origin/main`. O merge é humano: nenhum ticket faz merge, release ou alteração de produção sozinho. Enquanto `BO-GOV-001` não estiver `merged`, os demais tickets não geram branch, worktree nem pull request.

## Fronteira com o AutoPlatform

O AutoBO é cliente desktop do AutoPlatform. Identidade, contratos canônicos, estoque autoritativo, vendas, pagamentos, fiscal, Sicredi, webhooks, workers e segredos privilegiados ficam server-side. Tickets `BO-AP-*-EXT-*` representam essas entregas externas: permanecem `blocked`, não geram código neste repositório e só viram `merged` com evidência do ticket AutoPlatform correspondente e confirmação humana.

Um agente nunca deve contornar um espelho externo criando migrations compartilhadas, workers privilegiados ou adapters de provider dentro do Tauri.

## Ordem dos marcos

1. `BO-QA-003`: CI obrigatório.
2. Auth compartilhado e AutoBO standalone: catálogo, estoque e vendas.
3. Fiscal operacional com confirmação humana, sem esperar a integração com AutoOS.
4. PaymentIntent e Sicredi standalone.
5. `BO-SUITE-GATE-001`: Produto em Sincronia.
6. Reservas de OS, migração BMITAG e beta.

O marco `BO-SUITE-GATE-001` significa **Produto em Sincronia**: uma OS produz exatamente uma intenção financeira, o AutoBO publica o estado de volta e retries não duplicam dados. Esse gate não bloqueia fiscal nem Sicredi standalone. Depois desse gate:

- AutoOS segue para PowerSync, assinatura e fotos cloud.
- AutoBO passa a aceitar reservas e consumo de estoque originados por OS, preservando o funcionamento standalone.
- Ambos só mudam o contrato compartilhado por nova versão explícita.

O gate de sincronização não promete emissão de nota fiscal. Ele prova apenas a troca idempotente da intenção comercial e de seus estados. A emissão fiscal é um marco posterior: `autobo_nfes` continua sendo o registro de notas importadas, enquanto a emissão ganhará registro e máquina de estados próprios para não misturar documentos recebidos com documentos gerados pelo AutoBO.
