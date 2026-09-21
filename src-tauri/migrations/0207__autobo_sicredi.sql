-- Sicredi integration: banking identifiers, financial state machine fields,
-- retry metadata, and per-convenio liquidation sync cursor.

-- ─── Boleto: Sicredi identifiers (separate from local seu_numero) ─────────────
ALTER TABLE autobo_boletos ADD COLUMN IF NOT EXISTS sicredi_ambiente VARCHAR(20);
ALTER TABLE autobo_boletos ADD COLUMN IF NOT EXISTS sicredi_codigo_beneficiario VARCHAR(5);
ALTER TABLE autobo_boletos ADD COLUMN IF NOT EXISTS sicredi_seu_numero VARCHAR(10);
ALTER TABLE autobo_boletos ADD COLUMN IF NOT EXISTS sicredi_id_titulo_empresa VARCHAR(25);
ALTER TABLE autobo_boletos ADD COLUMN IF NOT EXISTS sicredi_transaction_id VARCHAR(36);

-- PDF artifact axis (independent of financial status)
ALTER TABLE autobo_boletos ADD COLUMN IF NOT EXISTS pdf_pendente BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE autobo_boletos ADD COLUMN IF NOT EXISTS pdf_oficial_path TEXT;

-- State machine / retry
ALTER TABLE autobo_boletos ADD COLUMN IF NOT EXISTS status_anterior VARCHAR(40);
ALTER TABLE autobo_boletos ADD COLUMN IF NOT EXISTS operacao_pendente VARCHAR(20);
ALTER TABLE autobo_boletos ADD COLUMN IF NOT EXISTS tentativas_operacao INTEGER NOT NULL DEFAULT 0;
ALTER TABLE autobo_boletos ADD COLUMN IF NOT EXISTS proxima_tentativa_em TIMESTAMP;
ALTER TABLE autobo_boletos ADD COLUMN IF NOT EXISTS ultima_tentativa_em TIMESTAMP;
ALTER TABLE autobo_boletos ADD COLUMN IF NOT EXISTS ultimo_erro_codigo VARCHAR(80);
ALTER TABLE autobo_boletos ADD COLUMN IF NOT EXISTS ultimo_erro_mensagem VARCHAR(500);
ALTER TABLE autobo_boletos ADD COLUMN IF NOT EXISTS ultimo_http_status INTEGER;

-- Allow full Sicredi especieDocumento enums
ALTER TABLE autobo_boletos ALTER COLUMN especie_documento TYPE VARCHAR(40);

-- Encargos are explicit — no silent defaults at API boundary
ALTER TABLE autobo_boletos ALTER COLUMN tipo_juros DROP NOT NULL;
ALTER TABLE autobo_boletos ALTER COLUMN tipo_multa DROP NOT NULL;

-- Unique banking ids per convenio (partial: only when set)
CREATE UNIQUE INDEX IF NOT EXISTS uq_boleto_sicredi_seu_numero
    ON autobo_boletos (sicredi_ambiente, sicredi_codigo_beneficiario, sicredi_seu_numero)
    WHERE sicredi_seu_numero IS NOT NULL
      AND sicredi_ambiente IS NOT NULL
      AND sicredi_codigo_beneficiario IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS uq_boleto_sicredi_id_titulo
    ON autobo_boletos (sicredi_ambiente, sicredi_codigo_beneficiario, sicredi_id_titulo_empresa)
    WHERE sicredi_id_titulo_empresa IS NOT NULL
      AND sicredi_ambiente IS NOT NULL
      AND sicredi_codigo_beneficiario IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS uq_boleto_sicredi_nosso_numero
    ON autobo_boletos (sicredi_ambiente, sicredi_codigo_beneficiario, nosso_numero)
    WHERE nosso_numero IS NOT NULL
      AND sicredi_ambiente IS NOT NULL
      AND sicredi_codigo_beneficiario IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_boleto_operacao_retry
    ON autobo_boletos (status, proxima_tentativa_em)
    WHERE status = 'AGUARDANDO_RETRY';

-- ─── Liquidation sync cursor (per ambiente + beneficiário) ────────────────────
CREATE TABLE IF NOT EXISTS sicredi_sincronizacao_liquidacoes (
    id                      BIGSERIAL PRIMARY KEY,
    ambiente                VARCHAR(20)  NOT NULL,
    codigo_beneficiario     VARCHAR(5)   NOT NULL,
    ultimo_dia_fechado      DATE         NOT NULL,
    atualizado_em           TIMESTAMP    NOT NULL DEFAULT NOW(),
    UNIQUE (ambiente, codigo_beneficiario)
);

-- ─── Config keys for Sicredi (replace legacy names) ───────────────────────────
INSERT INTO autobo_configuracoes (chave, valor, descricao) VALUES
    ('sicredi.cooperativa', '', 'Código da cooperativa (4 dígitos)'),
    ('sicredi.codigo_beneficiario', '', 'Código do beneficiário / convênio (5 dígitos)'),
    ('sicredi.codigo_acesso', '', 'Password OAuth (Internet Banking) — armazenado no keyring'),
    ('sicredi.prefixo_instalacao', '', 'Prefixo persistente da instalação para idTituloEmpresa'),
    ('sicredi.data_entrada_producao', '', 'YYYY-MM-DD — início do cursor se não houver boletos'),
    ('sicredi.especie_documento', 'DUPLICATA_MERCANTIL_INDICACAO', 'Espécie documental Sicredi padrão'),
    ('sicredi.campo_mensagem_json', '', 'mensagem ou mensagens — definido pelo teste de contrato')
ON CONFLICT (chave) DO NOTHING;

-- posto / ambiente / api_key vêm da 0205. api_key fica no keyring.
UPDATE autobo_configuracoes
SET descricao = 'API Key x-api-key — armazenada no keyring (valor no DB indica configurado)'
WHERE chave = 'sicredi.api_key';

UPDATE autobo_configuracoes
SET descricao = 'sandbox ou producao'
WHERE chave = 'sicredi.ambiente';

UPDATE autobo_configuracoes
SET descricao = 'Posto / agência (2 dígitos)'
WHERE chave = 'sicredi.posto';
