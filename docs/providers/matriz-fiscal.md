# Matriz de providers fiscais

NFS-e e NF-e modelo 55 são avaliadas em separado. Um provider não herda a cobertura do outro.

Legenda: obrigatório bloqueia a escolha; desejável não bloqueia; bloqueador impede o uso mesmo que o restante exista.

## NFS-e (serviços)

| Capacidade | Classe |
| --- | --- |
| Emissão idempotente por referência e empresa | obrigatório |
| Consulta do estado depois de timeout | obrigatório |
| Estado indeterminado sem reemissão automática | obrigatório |
| PDF e XML somente após emitida | obrigatório |
| Cancelamento com janela explícita | obrigatório |
| Rejeição com motivo acionável | obrigatório |
| Credencial de teste separada da live | obrigatório |
| Isolamento por empresa | obrigatório |
| CNAE / atividade de serviço | obrigatório |
| ISS retido e MEI | bloqueador se o cliente precisar e o provider não cobrir |
| Contingência municipal | desejável |

## NF-e modelo 55 (mercadorias)

| Capacidade | Classe |
| --- | --- |
| Autorização idempotente por referência e empresa | obrigatório |
| Consulta, rejeição e reconciliação de timeout | obrigatório |
| XML, chave e eventos ligados ao registro interno | obrigatório |
| Representação auxiliar recuperável após autorização | obrigatório |
| Cancelamento | obrigatório |
| Inutilização quando a faixa foi consumida sem autorização | obrigatório |
| Sandbox separado de produção | obrigatório |
| Isolamento por empresa | obrigatório |
| NCM, CFOP, CST/CSOSN, origem e unidade vindos do AutoPlatform | obrigatório |
| Contingência da UF alvo | bloqueador se a UF do cliente exigir e o provider não cobrir |
| Carta de correção | desejável |

## OS mista

O provider de NFS-e não é candidato a NF-e 55. A homologação mista só fecha quando as duas portas passam nos obrigatórios acima, cada uma no seu documento.
