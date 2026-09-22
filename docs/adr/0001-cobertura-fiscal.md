# ADR 0001 — Cobertura fiscal do AutoBO

Status: proposta para revisão humana. Arquitetura, divisão AutoBO/AutoPlatform e separação NFS-e/NF-e estão propostas. Provider não é escolhido nesta ADR. Não há contratação, chamada de API nem documento real.

## Contexto

O AutoBO importa NF-e em `autobo_nfes`. Essa tabela é origem de cobrança, não registro de emissão. O AutoBO é cliente desktop. O núcleo fiscal — domínio, estados, snapshots, armazenamento, credenciais e chamadas a providers — pertence ao AutoPlatform.

Para a BMITAG, a Lei Complementar 116, item 14.01, sustenta ISS sobre o serviço de manutenção e ICMS sobre peças e componentes empregados. Isso não é uma regra universal para qualquer venda mista.

## Decisão

### Documentos e operação

| Situação no escopo inicial da BMITAG | Documento |
| --- | --- |
| Só serviços | NFS-e |
| Só mercadorias | NF-e modelo 55 |
| Operação mista com incidências distintas | NFS-e para os serviços e NF-e 55 para as mercadorias |

Para o escopo inicial e os cenários fiscais homologados da BMITAG, uma operação mista é decomposta em intenções fiscais conforme sua natureza tributária. Quando houver incidências distintas, são emitidas NFS-e para os serviços e NF-e modelo 55 para as mercadorias. A matriz fiscal validada pela contabilidade determina essa decomposição. Se um dos lados não tem item, só a intenção correspondente é solicitada.

Uma operação mista é um agregado:

```text
FiscalOperation
├── FiscalDocument: NFS-e
└── FiscalDocument: NF-e
```

Estados da operação, derivados dos documentos filhos e não alterados à parte: `DRAFT`, `READY_FOR_REVIEW`, `PROCESSING`, `FULLY_ISSUED`, `PARTIALLY_ISSUED`, `REQUIRES_RECONCILIATION`, `FAILED`, `PARTIALLY_CANCELED`, `CANCELED`.

Uma NFS-e já `ISSUED` não é desfeita porque a NF-e falhou. Cada documento evolui sozinho. A operação só reflete a combinação.

Nenhum provider é escolhido antes do walkthrough real da BMITAG e da validação contábil.

### Momento e o que a indisponibilidade bloqueia

Pagamento, conclusão da OS e emissão são eventos independentes. A venda torna-se `FISCAL_ELIGIBLE` conforme a matriz homologada. Isso não é o mesmo que "faturar" nem o mesmo que receber.

A indisponibilidade fiscal não impede o registro da operação, a execução da oficina, a movimentação interna ou a cobrança. Ações legalmente condicionadas à autorização fiscal, como circulação ou entrega de mercadoria, podem permanecer bloqueadas até autorização, emissão em contingência válida ou liberação prevista na matriz fiscal homologada. O Portal da NF-e trata emissão, cancelamento e eventos em contingência por regras próprias.

### Confirmação e outbox

1. O AutoBO confirma enviando `document_id`, `expected_snapshot_version` e `expected_snapshot_hash`.
2. Se o snapshot mudou desde a abertura da tela, o AutoPlatform recusa e pede nova revisão.
3. O AutoPlatform valida a permissão da ação (`FISCAL_ISSUE` ou `FISCAL_CANCEL`).
4. Na mesma transação, cria o comando fiscal e o evento de outbox.
5. Um worker chama o provider.
6. A tentativa, o protocolo e o resultado ficam registrados.
7. O AutoBO acompanha o estado.

Fechar o AutoBO, perder internet ou reiniciar o servidor não perde a solicitação.

Permissões distintas: `FISCAL_VIEW`, `FISCAL_REVIEW`, `FISCAL_ISSUE`, `FISCAL_CANCEL`, `FISCAL_MANAGE_CREDENTIALS`. Cancelar exige permissão mais forte do que ver ou emitir.

### Três ciclos de vida

Não se faz `REJECTED → DRAFT` sobrescrevendo o que foi enviado.

`FiscalIntent`: `DRAFT` → `READY_FOR_REVIEW`.

`FiscalDocument`: `ISSUE_REQUESTED` → `PROCESSING` → `ISSUED` / `REJECTED` / `TECHNICAL_FAILURE` / `INDETERMINATE`, e depois `CANCEL_REQUESTED`, `CANCELED` ou, na NFS-e, `SUBSTITUTED`.

`FiscalAttempt`: `CREATED` → `SENT` → `RESPONSE_RECEIVED` / `FAILED` / `TIMED_OUT`.

Correção depois de rejeição:

1. O documento fica `REJECTED`.
2. A pessoa corrige os dados.
3. Nasce uma nova revisão do snapshot.
4. Essa revisão passa de novo pela confirmação humana.
5. A nova tentativa usa a revisão nova e preserva a anterior.

- `REJECTED`: a autoridade fiscal recusou por motivo tributário ou cadastral.
- `TECHNICAL_FAILURE`: erro interno, provider fora do ar ou credencial inválida.
- `INDETERMINATE`: a requisição foi enviada e não se sabe se houve autorização. Não há reemissão nem cancelamento automático.
- Eventos antigos ou atrasados não sobrescrevem um estado fiscal mais recente.
- `ISSUED` não é terminal: ainda pode haver cancelamento ou, na NFS-e, substituição por evento próprio.

### Estabelecimento, série e número

`company_id` não identifica o emissor. Uma empresa pode ter matriz e filiais com CNPJ, inscrições, séries e certificados diferentes. Todo documento carrega `issuer_establishment_id`, `environment` (`HOMOLOGATION` ou `PRODUCTION`), `document_series`, `document_number` e `authority_identifier`.

`IssuerFiscalProfile` pertence ao estabelecimento emissor. Não guarda certificado nem senha. Guarda só `certificate_secret_ref` e `provider_credential_ref`. Os valores ficam em cofre, com validade, rotação e auditoria.

A numeração é transacional:

```text
FiscalSequence(
  issuer_establishment_id,
  document_type,
  environment,
  series,
  next_number
)
```

A sequência controla a reserva concorrente dos números. Dois workers não consomem o mesmo número. Números reservados ou consumidos sem autorização geram uma pendência de reconciliação e, quando exigido, um evento de inutilização separado, preservando protocolo e resultado. A sequência não executa nem registra essa inutilização.

### Fonte dos dados e snapshot

- `FiscalCatalog`: NCM, CEST, origem, unidade e regras padrão.
- `IssuerFiscalProfile`: regime, CRT, inscrições e CNAE do estabelecimento, mais as referências de segredo.
- `FiscalDocumentSnapshot`: os dados exatos daquela revisão de emissão.

Alterar o NCM de um produto depois não altera nota já emitida nem revisão já confirmada.

Cada documento carrega `tax_schema_version` e `provider_contract_version`. Regras têm vigência inicial e final. CNPJ é texto, nunca número. Adapters são testados contra o layout vigente, inclusive a atualização de 2026 da NFS-e nacional para IBS/CBS e CNPJ alfanumérico. Versões antigas permanecem para documentos já emitidos.

O AutoBO não mantém um segundo cadastro tributário mutável. `autobo_nfes` continua só com notas importadas.

Esta ADR não exige mudança no AutoOS. Se um ticket futuro precisar de campo tributário novo na OS, ele abre um ticket espelho no AutoOS antes de qualquer implementação.

### Idempotência

A idempotência não depende de o provider oferecê-la. O AutoPlatform garante:

```text
UNIQUE (
  company_id,
  issuer_establishment_id,
  document_type,
  fiscal_reference
)
```

1. Persiste a intenção antes de chamar o provider.
2. Gera uma referência fiscal estável.
3. Registra cada tentativa.
4. Em timeout, consulta por referência ou protocolo.
5. Impede nova emissão enquanto o resultado estiver `INDETERMINATE`.

### XML, PDF e armazenamento

O XML fiscal autorizado, a chave e a representação auxiliar somente existem após a emissão. Payloads de solicitação, protocolos, respostas sanitizadas e rejeições são preservados em todas as tentativas.

DANFE e representação visual não são o registro fiscal autoritativo. O XML autorizado é o elemento principal.

XML e PDF ficam em storage privado, com URL assinada e temporária e hash do conteúdo. Credenciais, certificado e senhas ficam só no cofre do servidor e nunca vão para o AutoBO. A auditoria registra quem confirmou, emitiu e cancelou. Logs não levam XML completo, CPF/CNPJ desnecessário nem credencial. O isolamento por `company_id` e `issuer_establishment_id` é testado, não apenas declarado.

### Portas

NFS-e e NF-e modelo 55 são portas separadas no AutoPlatform. Substituição é evento da NFS-e: o sistema nacional gera uma nova nota, cancela a anterior por substituição e mantém as duas vinculadas. A NF-e não herda essa abstração. Nela os caminhos são cancelamento, carta de correção, NF-e complementar, nova NF-e referenciando a anterior quando a matriz exigir, e inutilização de numeração. A carta de correção não altera valores, impostos, remetente, destinatário ou data.

O AutoBO não implementa o domínio nem o adapter. A matriz está em `docs/providers/matriz-fiscal.md`.

## Quem implementa

| Ticket no AutoPlatform | Espelho local | Papel |
| --- | --- | --- |
| AP-FISC-CORE-001 | `BO-AP-FISC-CORE-EXT-001` | Domínio, estados, snapshots, sequência, armazenamento e outbox |
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
| Manutenção BMITAG com serviço e peça | A matriz decompõe em NFS-e e NF-e 55 dentro de uma `FiscalOperation` |
| As duas emitidas | Operação `FULLY_ISSUED` |
| NFS-e emitida e NF-e rejeitada | Operação `PARTIALLY_ISSUED`; a NFS-e permanece válida; a NF-e mostra a correção |
| NFS-e cancelada e NF-e ainda emitida | Operação `PARTIALLY_CANCELED`; a NF-e autorizada permanece |
| Uma emitida e a outra indeterminada | Operação `REQUIRES_RECONCILIATION`; sem cancelamento nem reenvio automático |
| Autoridade recusa um documento | `REJECTED`; nova revisão do snapshot e nova confirmação antes de outra tentativa |
| Provider fora ou credencial inválida | `TECHNICAL_FAILURE` naquele documento |
| Timeout | `INDETERMINATE`; consulta por referência ou protocolo |
| Cancelamento de NFS-e ou NF-e | Evento próprio, com prazo e motivo |
| Substituição | Só na NFS-e, como evento que vincula a nota nova à anterior |
| Carta de correção | Só na NF-e, sem alterar valor, imposto, partes ou data |
| Entrega de mercadoria sem NF-e autorizada nem contingência válida | Circulação pode permanecer bloqueada; o restante da operação já pode estar registrado |
