CREATE TABLE IF NOT EXISTS autobo_nfes (
    id                      BIGSERIAL PRIMARY KEY,
    pagador_id              BIGINT       REFERENCES autobo_pagadores(id),
    numero_nf               VARCHAR(20)  NOT NULL,
    serie                   VARCHAR(10),
    chave_acesso            VARCHAR(44)  UNIQUE,
    data_emissao            DATE         NOT NULL,
    valor_total             DECIMAL(12,2) NOT NULL,
    valor_produtos          DECIMAL(12,2),
    valor_servicos          DECIMAL(12,2),
    natureza_operacao       VARCHAR(100),
    xml_conteudo            TEXT,
    status                  VARCHAR(20)  NOT NULL DEFAULT 'IMPORTADA',
    boleto_id               BIGINT,
    importado_em            TIMESTAMP    NOT NULL DEFAULT NOW(),
    importado_por           VARCHAR(100)
);
CREATE INDEX IF NOT EXISTS idx_autobo_nfe_chave   ON autobo_nfes(chave_acesso);
CREATE INDEX IF NOT EXISTS idx_autobo_nfe_numero  ON autobo_nfes(numero_nf);
CREATE INDEX IF NOT EXISTS idx_autobo_nfe_pagador ON autobo_nfes(pagador_id);
CREATE INDEX IF NOT EXISTS idx_autobo_nfe_status  ON autobo_nfes(status);
