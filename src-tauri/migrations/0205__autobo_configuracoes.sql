CREATE TABLE IF NOT EXISTS autobo_configuracoes (
    id                      BIGSERIAL PRIMARY KEY,
    chave                   VARCHAR(100) NOT NULL UNIQUE,
    valor                   TEXT         NOT NULL,
    descricao               VARCHAR(300),
    atualizado_em           TIMESTAMP    NOT NULL DEFAULT NOW()
);

ALTER TABLE autobo_configuracoes ADD COLUMN IF NOT EXISTS id BIGSERIAL;
ALTER TABLE autobo_configuracoes ADD COLUMN IF NOT EXISTS descricao VARCHAR(300);

-- Seed data: all initial configuration keys
INSERT INTO autobo_configuracoes (chave, valor, descricao) VALUES
    ('empresa.cnpj', '', 'CNPJ da empresa emitente (apenas números)'),
    ('empresa.razao_social', '', 'Razão social'),
    ('empresa.nome_fantasia', '', 'Nome fantasia'),
    ('empresa.logradouro', '', 'Endereço'),
    ('empresa.numero', '', 'Número'),
    ('empresa.complemento', '', 'Complemento'),
    ('empresa.bairro', '', 'Bairro'),
    ('empresa.cidade', '', 'Cidade'),
    ('empresa.uf', '', 'UF'),
    ('empresa.cep', '', 'CEP'),
    ('sicredi.agencia', '', 'Agência Sicredi'),
    ('sicredi.posto', '', 'Posto Sicredi'),
    ('sicredi.conta', '', 'Conta corrente'),
    ('sicredi.client_id', '', 'Client ID OAuth'),
    ('sicredi.client_secret', '', 'Client Secret OAuth'),
    ('sicredi.api_key', '', 'API Key x-api-key'),
    ('sicredi.ambiente', 'sandbox', 'sandbox ou producao'),
    ('smtp.host', '', 'Servidor SMTP'),
    ('smtp.porta', '587', 'Porta SMTP'),
    ('smtp.usuario', '', 'Usuário SMTP'),
    ('smtp.senha', '', 'Senha SMTP (será criptografada)'),
    ('smtp.remetente', '', 'Email remetente'),
    ('whatsapp.ativo', 'false', 'WhatsApp ativo (true/false)'),
    ('whatsapp.token', '', 'Token da API WhatsApp (será criptografado)'),
    ('whatsapp.url_api', '', 'URL da API WhatsApp'),
    ('boleto.mensagem_padrao', 'Boleto referente aos serviços prestados.', 'Mensagem padrão no boleto'),
    ('boleto.dias_vencimento_padrao', '5', 'Dias padrão para vencimento')
ON CONFLICT (chave) DO NOTHING;
