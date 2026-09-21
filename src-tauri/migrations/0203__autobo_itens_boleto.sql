CREATE TABLE IF NOT EXISTS autobo_itens_boleto (
    id                      BIGSERIAL PRIMARY KEY,
    boleto_id               BIGINT       NOT NULL REFERENCES autobo_boletos(id) ON DELETE CASCADE,
    descricao               VARCHAR(500) NOT NULL,
    quantidade              DECIMAL(10,2) NOT NULL DEFAULT 1,
    unidade                 VARCHAR(10)  NOT NULL DEFAULT 'UN',
    valor_unitario          DECIMAL(12,2) NOT NULL,
    subtotal                DECIMAL(12,2) NOT NULL,
    produto_autoos_id       BIGINT
);

ALTER TABLE autobo_itens_boleto ADD COLUMN IF NOT EXISTS descricao VARCHAR(500);
ALTER TABLE autobo_itens_boleto ADD COLUMN IF NOT EXISTS valor_unitario DECIMAL(12,2);
ALTER TABLE autobo_itens_boleto ADD COLUMN IF NOT EXISTS produto_autoos_id BIGINT;
ALTER TABLE autobo_itens_boleto ADD COLUMN IF NOT EXISTS subtotal DECIMAL(12,2);

CREATE INDEX IF NOT EXISTS idx_autobo_item_boleto ON autobo_itens_boleto(boleto_id);
