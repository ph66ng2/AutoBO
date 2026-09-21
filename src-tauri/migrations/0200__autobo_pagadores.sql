CREATE TABLE IF NOT EXISTS autobo_pagadores (
    id                      BIGSERIAL PRIMARY KEY,
    tipo_pessoa             VARCHAR(2)   NOT NULL,
    documento               VARCHAR(14)  NOT NULL UNIQUE,
    nome                    VARCHAR(200),
    razao_social            VARCHAR(200),
    nome_fantasia           VARCHAR(200),
    telefone                VARCHAR(15),
    email                   VARCHAR(150),
    cep                     VARCHAR(8),
    logradouro              VARCHAR(200),
    numero                  VARCHAR(10),
    complemento             VARCHAR(100),
    bairro                  VARCHAR(100),
    cidade                  VARCHAR(100),
    uf                      CHAR(2),
    prazo_vencimento_dias   INTEGER      NOT NULL DEFAULT 5,
    tipo_cobranca_padrao    VARCHAR(20)  NOT NULL DEFAULT 'SIMPLES',
    percentual_juros_mes    DECIMAL(5,2) NOT NULL DEFAULT 1.00,
    percentual_multa        DECIMAL(5,2) NOT NULL DEFAULT 2.00,
    dias_protesto           INTEGER,
    mensagem_padrao         TEXT,
    ativo                   BOOLEAN      NOT NULL DEFAULT TRUE,
    bloqueado               BOOLEAN      NOT NULL DEFAULT FALSE,
    motivo_bloqueio         TEXT,
    criado_em               TIMESTAMP    NOT NULL DEFAULT NOW(),
    atualizado_em           TIMESTAMP    NOT NULL DEFAULT NOW()
);

ALTER TABLE autobo_pagadores ADD COLUMN IF NOT EXISTS nome VARCHAR(200);
ALTER TABLE autobo_pagadores ADD COLUMN IF NOT EXISTS razao_social VARCHAR(200);
ALTER TABLE autobo_pagadores ADD COLUMN IF NOT EXISTS nome_fantasia VARCHAR(200);
ALTER TABLE autobo_pagadores ADD COLUMN IF NOT EXISTS telefone VARCHAR(15);
ALTER TABLE autobo_pagadores ADD COLUMN IF NOT EXISTS email VARCHAR(150);

CREATE UNIQUE INDEX IF NOT EXISTS autobo_pagadores_documento_uidx ON autobo_pagadores (documento);
CREATE INDEX IF NOT EXISTS idx_autobo_pagador_documento ON autobo_pagadores(documento);
