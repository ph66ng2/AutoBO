# ADR 0001 — Cobertura fiscal do AutoBO

Status: proposta para revisão humana. Esta ADR não contrata provider, não chama API fiscal e não emite documento real.

## Contexto

O AutoBO importa NF-e em `autobo_nfes`. Essa tabela é origem de cobrança, não registro de emissão. O AutoBO é cliente desktop do AutoPlatform: identidade, contratos, estoque autoritativo, vendas, fiscal e Sicredi ficam server-side. Um provider de NFS-e não cobre NF-e modelo 55 de peças.

## Decisão

### Documentos

| Situação | Documento |
| --- | --- |
| Só serviços | NFS-e |
| Só mercadorias | NF-e modelo 55 |
| OS ou venda mista | Dois documentos: NFS-e para os serviços e NF-e 55 para as peças |

Não existe documento único que misture serviço e mercadoria. Se um dos lados não tem item, só o documento correspondente é solicitado.

### Momento e autorização

A emissão não ocorre na conclusão da OS e não dispara sozinha depois do pagamento. A indisponibilidade fiscal não pode parar a operação da oficina.

O momento é o faturamento de uma venda ou recebível já existente. O AutoBO só envia `issue` ou `cancel` depois de permissão `FISCAL` e confirmação humana na fila de revisão. Até essa confirmação o documento permanece rascunho.

### Fonte dos dados tributários

O AutoPlatform é a fonte autoritativa de NCM, CFOP, CST/CSOSN, origem, unidade e CNAE. O AutoBO não mantém um segundo cadastro tributário mutável.

`autobo_nfes` continua só com notas importadas. Emissão, chave, PDF, XML, rejeição e histórico vivem no núcleo fiscal do AutoPlatform, isolados por `company_id`.

Esta ADR não exige mudança no AutoOS. Se um ticket futuro precisar de campo tributário novo na OS, ele abre um ticket espelho no AutoOS antes de qualquer implementação.

### Estados

Estados provider-neutral: rascunho, solicitação, processamento, emitida, falha, indeterminada, cancelamento solicitado, cancelada.

- Retry da mesma referência devolve o mesmo documento.
- Timeout ou resposta perdida fica `indeterminada` até reconciliação. Não há reemissão automática.
- Rejeição guarda motivo acionável e não apaga a intenção.
- Cancelamento e substituição são eventos novos. Não se sobrescreve o documento emitido.
- PDF e XML só são obtidos depois de `emitida`.
- Estado final não regride.

### Portas de provider

NFS-e e NF-e modelo 55 são portas separadas. O domínio do AutoBO não depende do payload de um provider. A Ext, se homologada, cobre apenas NFS-e de teste e não vira solução de NF-e de peças.

## Consequências

`BO-FISC-CORE-001` consome o núcleo server-side e a revisão humana. `BO-FISC-EXT-001` e `BO-FISC-NFE-001` homologam adapters distintos. A matriz de capacidades está em `docs/providers/matriz-fiscal.md`.

## Walkthrough

| Cenário | Resultado |
| --- | --- |
| Venda só de serviço | Uma NFS-e, depois de confirmação humana |
| Venda só de peça | Uma NF-e 55, depois de confirmação humana |
| OS com serviço e peça | Uma NFS-e e uma NF-e 55, correlacionadas à mesma venda |
| Provider rejeita | Intenção permanece, motivo visível, nenhum segundo documento |
| Timeout | Estado indeterminado, sem reemissão automática |
| Cancelamento | Evento de cancelamento solicitado; o emitido não é apagado |
| Substituição | Novo documento ligado ao anterior, não um update do XML antigo |
