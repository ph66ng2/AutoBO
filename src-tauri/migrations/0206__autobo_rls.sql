-- Desktop-only tables. RLS on, no Data API policies.
-- Grants to anon/authenticated/service_role are revoked in the app when those roles exist.

ALTER TABLE autobo_pagadores ENABLE ROW LEVEL SECURITY;
ALTER TABLE autobo_nfes ENABLE ROW LEVEL SECURITY;
ALTER TABLE autobo_boletos ENABLE ROW LEVEL SECURITY;
ALTER TABLE autobo_itens_boleto ENABLE ROW LEVEL SECURITY;
ALTER TABLE autobo_comunicacoes ENABLE ROW LEVEL SECURITY;
ALTER TABLE autobo_configuracoes ENABLE ROW LEVEL SECURITY;
ALTER TABLE autobo_migrations ENABLE ROW LEVEL SECURITY;
