# Matriz de providers fiscais

NFS-e e NF-e modelo 55 são avaliadas em separado. Um provider não herda a cobertura do outro. A escolha espera o walkthrough da BMITAG e a validação contábil. A decomposição de uma operação mista segue a matriz homologada, não uma regra universal.

Legenda: obrigatório bloqueia a escolha; desejável não bloqueia; bloqueador impede o uso mesmo que o restante exista.

A idempotência da plataforma é:

```text
UNIQUE (
  company_id,
  issuer_establishment_id,
  environment,
  document_type,
  fiscal_reference
)
```

No provider, o obrigatório é idempotência nativa ou um mecanismo confiável de consulta e reconciliação por referência ou protocolo.

## NFS-e (serviços)

| Capacidade | Classe |
| --- | --- |
| Idempotência nativa ou consulta/reconciliação por referência ou protocolo | obrigatório |
| Consulta do estado depois de timeout | obrigatório |
| Estado indeterminado sem reemissão automática | obrigatório |
| XML autorizado, chave e representação auxiliar somente após a emissão | obrigatório |
| Preservar solicitação, protocolo, resposta sanitizada e rejeição em toda tentativa | obrigatório |
| Prazo de cancelamento, motivo obrigatório, consulta do evento e comportamento fora do prazo | obrigatório |
| Substituição como evento próprio: nova NFS-e, cancelamento da anterior por substituição e vínculo entre as duas | obrigatório |
| Intervenção municipal quando a substituição ou o cancelamento exigirem | bloqueador se o município exigir e o provider não cobrir |
| Rejeição tributária ou cadastral distinta de falha técnica | obrigatório |
| Credencial de teste separada da live, só no servidor | obrigatório |
| Isolamento por `company_id` e `issuer_establishment_id`, testado | obrigatório |
| CNAE / atividade de serviço | obrigatório |
| `tax_schema_version` e layout vigente, inclusive IBS/CBS e CNPJ alfanumérico | obrigatório |
| ISS retido e MEI | bloqueador se o cliente precisar e o provider não cobrir |
| Contingência municipal | desejável |

## NF-e modelo 55 (mercadorias)

| Capacidade | Classe |
| --- | --- |
| Idempotência nativa ou consulta/reconciliação por referência ou protocolo | obrigatório |
| Consulta, rejeição e reconciliação de timeout | obrigatório |
| XML autorizado, chave e eventos ligados ao snapshot da emissão | obrigatório |
| Representação auxiliar recuperável após autorização, sem substituir o XML | obrigatório |
| Cancelamento e consulta do evento | obrigatório |
| Carta de correção, respeitando os campos permitidos | obrigatório |
| NF-e complementar e documentos referenciados | bloqueador se a matriz BMITAG exigir |
| Inutilização de numeração | obrigatório |
| Intervenção estadual quando exigida | bloqueador se a UF exigir e o provider não cobrir |
| Sandbox separado de produção | obrigatório |
| Isolamento por `company_id` e `issuer_establishment_id`, testado | obrigatório |
| NCM, CFOP, CST/CSOSN, origem e unidade lidos do snapshot, não do catálogo vivo | obrigatório |
| `tax_schema_version` e `provider_contract_version` | obrigatório |
| Contingência da UF alvo | bloqueador se a UF do cliente exigir e o provider não cobrir |

## Operação mista

A homologação mista da BMITAG só fecha quando a matriz contábil manda decompor e as duas portas passam nos obrigatórios, cada uma no seu documento. A operação agrega esses documentos e deriva o próprio estado deles. Não cancela nem reenvia sozinha o documento que já foi autorizado.

| Combinação dos filhos | Estado derivado da operação |
| --- | --- |
| NFS-e e NF-e emitidas | `FULLY_ISSUED` |
| Uma emitida e a outra rejeitada | `PARTIALLY_ISSUED` |
| Uma emitida e a outra indeterminada | `REQUIRES_RECONCILIATION` |
| Uma cancelada e a outra ainda emitida | `PARTIALLY_CANCELED` |
| As duas canceladas | `CANCELED` |
