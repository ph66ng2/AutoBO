# ADR 0001 — Cobertura fiscal do AutoBO

Status: proposta para revisão humana. Arquitetura, divisão AutoBO/AutoPlatform e separação NFS-e/NF-e estão propostas. Provider não é escolhido nesta ADR. Não há contratação, chamada de API nem documento real.

## Contexto

O AutoBO importa NF-e em `autobo_nfes`. Essa tabela é origem de cobrança, não registro de emissão. O AutoBO é cliente desktop. O núcleo fiscal — domínio, estados, snapshots, armazenamento, credenciais e chamadas a providers — pertence ao AutoPlatform.

Para a BMITAG, a Lei Complementar 116, item 14.01, sustenta ISS sobre o serviço de manutenção e ICMS sobre peças e componentes empregados. Isso não é uma regra universal para qualquer venda mista.

## Decisão

### Documentos

| Situação no escopo inicial da BMITAG | Documento |
| --- | --- |
| Só serviços | NFS-e |
| Só mercadorias | NF-e modelo 55 |
| Operação mista com incidências distintas | NFS-e para os serviços e NF-e 55 para as mercadorias |

Para o escopo inicial e os cenários fiscais homologados da BMITAG, uma operação mista é decomposta em intenções fiscais conforme sua natureza tributária. Quando houver incidências distintas, são emitidas NFS-e para os serviços e NF-e modelo 55 para as mercadorias. A matriz fiscal validada pela contabilidade determina essa decomposição. Se um dos lados não tem item, só a intenção correspondente é solicitada.

Nenhum provider é escolhido antes do walkthrough real da BMITAG e da validação contábil.

### Momento e autorização

A emissão não ocorre na conclusão da OS e não dispara sozinha depois do pagamento. A indisponibilidade fiscal não pode parar a operação da oficina.

O momento é o faturamento de uma venda ou recebível já existente. O fluxo é:

1. O AutoBO confirma.
2. O AutoPlatform valida a permissão `FISCAL`.
3. Na mesma transação, cria o comando fiscal e o evento de outbox.
4. Um worker chama o provider.
5. A tentativa, o protocolo e o resultado ficam registrados.
6. O AutoBO acompanha o estado.

Fechar o AutoBO, perder internet ou reiniciar o servidor não perde a solicitação. Até a confirmação humana o documento permanece `DRAFT` ou `READY_FOR_REVIEW`.

### Fonte dos dados e snapshot

O AutoPlatform é a fonte autoritativa, separada em três registros:

- `FiscalCatalog`: NCM, CEST, origem, unidade e regras padrão.
- `IssuerFiscalProfile`: regime, CRT, inscrição estadual, inscrição municipal, CNAE, certificados e credenciais.
- `FiscalDocumentSnapshot`: os dados exatos usados naquela emissão.

Alterar o NCM de um produto depois não altera nota já emitida nem rascunho já confirmado. O snapshot congela o que foi usado.

Cada documento carrega `tax_schema_version` e `provider_contract_version`. Regras têm vigência inicial e final. CNPJ é texto, nunca número. Adapters são testados contra o layout vigente, inclusive a atualização de 2026 da NFS-e nacional para IBS/CBS e CNPJ alfanumérico. Versões antigas permanecem para documentos já emitidos.

O AutoBO não mantém um segundo cadastro tributário mutável. `autobo_nfes` continua só com notas importadas.

Esta ADR não exige mudança no AutoOS. Se um ticket futuro precisar de campo tributário novo na OS, ele abre um ticket espelho no AutoOS antes de qualquer implementação.

### Idempotência

A idempotência não depende de o provider oferecê-la. O AutoPlatform garante:

```text
UNIQUE (company_id, document_type, fiscal_reference)
```

1. Persiste a intenção antes de chamar o provider.
2. Gera uma referência fiscal estável.
3. Registra cada tentativa.
4. Em timeout, consulta por referência ou protocolo.
5. Impede nova emissão enquanto o resultado estiver `INDETERMINATE`.

### Estados

`DRAFT`, `READY_FOR_REVIEW`, `ISSUE_REQUESTED`, `PROCESSING`, `ISSUED`, `REJECTED`, `TECHNICAL_FAILURE`, `INDETERMINATE`, `CANCEL_REQUESTED`, `CANCELED`, `SUBSTITUTED`.

- `REJECTED`: a autoridade fiscal recusou por motivo tributário ou cadastral.
- `TECHNICAL_FAILURE`: erro interno, provider fora do ar ou credencial inválida.
- `INDETERMINATE`: a requisição foi enviada e não se sabe se houve autorização. Não há reemissão automática.
- Eventos antigos ou atrasados não sobrescrevem um estado fiscal mais recente.
- Um documento `ISSUED` pode depois ser cancelado ou substituído por evento fiscal próprio. `ISSUED` não é um estado terminal absoluto.
- Cancelamento e substituição preservam a ligação entre os documentos. Não se sobrescreve o XML autorizado.

### XML, PDF e armazenamento

O XML fiscal autorizado, a chave e a representação auxiliar somente existem após a emissão. Payloads de solicitação, protocolos, respostas sanitizadas e rejeições são preservados em todas as tentativas.

DANFE e representação visual não são o registro fiscal autoritativo. O XML autorizado é o elemento principal.

XML e PDF ficam em storage privado, com URL assinada e temporária e hash do conteúdo. Credenciais, certificado e senhas ficam só no servidor e nunca vão para o AutoBO. A auditoria registra quem confirmou, emitiu e cancelou. Logs não levam XML completo, CPF/CNPJ desnecessário nem credencial. O isolamento por `company_id` é testado, não apenas declarado.

### Portas

NFS-e e NF-e modelo 55 são portas separadas no AutoPlatform. O AutoBO não implementa o domínio nem o adapter. A matriz está em `docs/providers/matriz-fiscal.md`.

## Quem implementa

| Ticket no AutoPlatform | Espelho local | Papel |
| --- | --- | --- |
| AP-FISC-CORE-001 | `BO-AP-FISC-CORE-EXT-001` | Domínio, estados, snapshots, armazenamento e outbox |
| AP-FISC-NFSE-001 | `BO-AP-FISC-NFSE-EXT-001` | Adapter de NFS-e |
| AP-FISC-NFE-001 | `BO-AP-FISC-NFE-EXT-001` | Adapter de NF-e 55 |
| — | `BO-FISC-UI-001` | Fila de revisão e confirmação humana |
| — | `BO-FISC-STATUS-001` | Acompanhamento, rejeição, PDF/XML e histórico |
| — | `BO-FISC-IMPORT-001` | Manutenção do fluxo existente de `autobo_nfes` |

Os espelhos `BO-AP-*-EXT-*` permanecem `blocked` e não geram código neste repositório.

## Walkthrough

| Cenário | Resultado |
| --- | --- |
| Venda só de serviço, matriz homologada | Uma NFS-e, depois de confirmação humana |
| Venda só de peça | Uma NF-e 55, depois de confirmação humana |
| Manutenção BMITAG com serviço e peça | A matriz decompõe em NFS-e e NF-e 55 correlacionadas |
| Autoridade recusa | `REJECTED`, motivo acionável, intenção preservada |
| Provider fora ou credencial inválida | `TECHNICAL_FAILURE`, sem novo documento |
| Timeout | `INDETERMINATE`, consulta por referência ou protocolo, sem reemissão |
| Cancelamento | Evento próprio, com prazo e motivo; o XML autorizado permanece |
| Substituição | Novo documento ligado ao anterior |
