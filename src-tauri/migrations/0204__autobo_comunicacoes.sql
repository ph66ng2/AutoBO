CREATE TABLE IF NOT EXISTS autobo_comunicacoes (
    id                      BIGSERIAL PRIMARY KEY,
    boleto_id               BIGINT       NOT NULL REFERENCES autobo_boletos(id) ON DELETE CASCADE,
    tipo                    VARCHAR(30)  NOT NULL,
    canal                   VARCHAR(20)  NOT NULL,
    destinatario            VARCHAR(150) NOT NULL,
    assunto                 VARCHAR(200),
    conteudo                TEXT,
    status_envio            VARCHAR(20)  NOT NULL DEFAULT 'PENDENTE',
    tentativas              INTEGER      NOT NULL DEFAULT 0,
    erro                    TEXT,
    enviado_em              TIMESTAMP,
    criado_em               TIMESTAMP    NOT NULL DEFAULT NOW()
);

ALTER TABLE autobo_comunicacoes ADD COLUMN IF NOT EXISTS destinatario VARCHAR(150) NOT NULL DEFAULT '';
ALTER TABLE autobo_comunicacoes ADD COLUMN IF NOT EXISTS assunto VARCHAR(200);
ALTER TABLE autobo_comunicacoes ADD COLUMN IF NOT EXISTS conteudo TEXT;
ALTER TABLE autobo_comunicacoes ADD COLUMN IF NOT EXISTS status_envio VARCHAR(20) NOT NULL DEFAULT 'PENDENTE';
ALTER TABLE autobo_comunicacoes ADD COLUMN IF NOT EXISTS tentativas INTEGER NOT NULL DEFAULT 0;
ALTER TABLE autobo_comunicacoes ADD COLUMN IF NOT EXISTS enviado_em TIMESTAMP;

CREATE INDEX IF NOT EXISTS idx_autobo_com_boleto ON autobo_comunicacoes(boleto_id);
CREATE INDEX IF NOT EXISTS idx_autobo_com_status ON autobo_comunicacoes(status_envio);
