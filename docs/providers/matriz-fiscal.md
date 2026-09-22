# Matriz de providers fiscais

NFS-e e NF-e modelo 55 são avaliadas em separado. Um provider não herda a cobertura do outro. A escolha espera o walkthrough da BMITAG e a validação contábil. A decomposição de uma operação mista segue a matriz homologada, não uma regra universal.

Legenda: obrigatório bloqueia a escolha; desejável não bloqueia; bloqueador impede o uso mesmo que o restante exista.

A idempotência da plataforma é `UNIQUE (company_id, document_type, fiscal_reference)`. No provider, o obrigatório é idempotência nativa ou um mecanismo confiável de consulta e reconciliação por referência ou protocolo.

## NFS-e (serviços)

| Capacidade | Classe |
| --- | --- |
| Idempotência nativa ou consulta/reconciliação por referência ou protocolo | obrigatório |
| Consulta do estado depois de timeout | obrigatório |
| Estado indeterminado sem reemissão automática | obrigatório |
| XML autorizado, chave e representação auxiliar somente após a emissão | obrigatório |
| Preservar solicitação, protocolo, resposta sanitizada e rejeição em toda tentativa | obrigatório |
| Prazo de cancelamento e comportamento fora do prazo | obrigatório |
| Motivo obrigatório de cancelamento | obrigatório |
| Substituição como evento próprio, com ligação ao documento anterior | obrigatório |
| Consulta do evento por protocolo | obrigatório |
| Intervenção municipal quando a substituição ou o cancelamento exigirem | bloqueador se o município exigir e o provider não cobrir |
| Rejeição tributária ou cadastral distinta de falha técnica | obrigatório |
| Credencial de teste separada da live, só no servidor | obrigatório |
| Isolamento por `company_id`, testado | obrigatório |
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
| Prazo de cancelamento, motivo obrigatório e comportamento fora do prazo | obrigatório |
| Substituição ou evento equivalente com ligação ao documento anterior | obrigatório |
| Consulta do evento por protocolo | obrigatório |
| Intervenção estadual quando exigida | bloqueador se a UF exigir e o provider não cobrir |
| Inutilização quando a faixa foi consumida sem autorização | obrigatório |
| Sandbox separado de produção | obrigatório |
| Isolamento por `company_id`, testado | obrigatório |
| NCM, CFOP, CST/CSOSN, origem e unidade lidos do snapshot, não do catálogo vivo | obrigatório |
| `tax_schema_version` e `provider_contract_version` | obrigatório |
| Contingência da UF alvo | bloqueador se a UF do cliente exigir e o provider não cobrir |
| Carta de correção | desejável |

## Operação mista

A homologação mista da BMITAG só fecha quando a matriz contábil manda decompor e as duas portas passam nos obrigatórios, cada uma no seu documento.
