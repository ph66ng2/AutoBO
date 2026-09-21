CREATE TABLE IF NOT EXISTS autobo_boletos (
    id                      BIGSERIAL PRIMARY KEY,
    pagador_id              BIGINT       NOT NULL REFERENCES autobo_pagadores(id),
    nfe_id                  BIGINT       REFERENCES autobo_nfes(id),
    seu_numero              VARCHAR(50)  NOT NULL,
    nosso_numero            VARCHAR(20),
    valor_nominal           DECIMAL(12,2) NOT NULL,
    valor_pago              DECIMAL(12,2),
    data_emissao            DATE         NOT NULL DEFAULT CURRENT_DATE,
    data_vencimento         DATE         NOT NULL,
    data_pagamento          DATE,
    tipo_cobranca           VARCHAR(20)  NOT NULL DEFAULT 'SIMPLES',
    especie_documento       VARCHAR(10)  NOT NULL DEFAULT 'DM',
    tipo_juros              VARCHAR(30)  NOT NULL DEFAULT 'PERCENTUAL_MES',
    percentual_juros_mes    DECIMAL(5,2),
    tipo_multa              VARCHAR(20)  NOT NULL DEFAULT 'PERCENTUAL',
    percentual_multa        DECIMAL(5,2),
    dias_protesto           INTEGER,
    mensagem                TEXT,
    linha_digitavel         VARCHAR(60),
    codigo_barras           VARCHAR(54),
    txid                    VARCHAR(100),
    qr_code                 TEXT,
    status                  VARCHAR(20)  NOT NULL DEFAULT 'RASCUNHO',
    email_enviado           BOOLEAN      NOT NULL DEFAULT FALSE,
    whatsapp_enviado        BOOLEAN      NOT NULL DEFAULT FALSE,
    origem                  VARCHAR(20)  NOT NULL DEFAULT 'NFE',
    gerado_por              VARCHAR(100),
    criado_em               TIMESTAMP    NOT NULL DEFAULT NOW(),
    atualizado_em           TIMESTAMP    NOT NULL DEFAULT NOW()
);

-- Schema legado (V1xx): a tabela já existia sem nfe_id/origem/criado_em.
ALTER TABLE autobo_boletos ADD COLUMN IF NOT EXISTS nfe_id BIGINT REFERENCES autobo_nfes(id);
ALTER TABLE autobo_boletos ADD COLUMN IF NOT EXISTS origem VARCHAR(20) NOT NULL DEFAULT 'NFE';
ALTER TABLE autobo_boletos ADD COLUMN IF NOT EXISTS criado_em TIMESTAMP NOT NULL DEFAULT NOW();

CREATE INDEX IF NOT EXISTS idx_autobo_boleto_pagador    ON autobo_boletos(pagador_id);
CREATE INDEX IF NOT EXISTS idx_autobo_boleto_nfe        ON autobo_boletos(nfe_id);
CREATE INDEX IF NOT EXISTS idx_autobo_boleto_status     ON autobo_boletos(status);
CREATE INDEX IF NOT EXISTS idx_autobo_boleto_vencimento ON autobo_boletos(data_vencimento);
CREATE INDEX IF NOT EXISTS idx_autobo_boleto_origem     ON autobo_boletos(origem);
