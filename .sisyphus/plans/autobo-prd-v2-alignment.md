# Alinhar AutoBO com PRD v2

## TL;DR

> **Construir o AutoBO do zero seguindo o PRD v2**: o código anterior foi removido e precisa ser recriado. O PRD exige importação de NF-e XML e entrada manual, integração opcional com AutoOS (leitura de `produtos` apenas), e geração de boletos via API Sicredi.
>
> **Deliverables**:
> - Migrations V200-V205 seguindo o modelo de dados do PRD
> - Parser de NF-e XML (3.1 e 4.0) com validações
> - Comando `importar_nfe` e `importar_nfe_lote`
> - Fluxo `gerar_boleto` reescrito (origem NF-e ou manual, sem `movimento_ids`)
> - Comando `listar_produtos_autoos` (opcional, leitura apenas)
> - Criptografia de senhas/tokens sensíveis
> - Frontend atualizado para o novo fluxo
>
> **Estimated Effort**: Large
> **Parallel Execution**: YES - 5 waves (0-4)
> **Critical Path**: Task 0 → Task 1 → Task 3 → Task 10 → Task 14/15 → Final

---

## Context

### Original Request
O projeto AutoBO precisa ser reconstruído do zero seguindo o PRD v2. O código anterior (que acoplava com `movimentacoes_estoque` do AutoOS) foi removido. O PRD v2 exige um sistema standalone onde o financeiro importa NF-e XML ou digita dados manualmente para gerar boletos via API Sicredi. A única integração com AutoOS é leitura opcional da tabela `produtos`.

### Interview Summary
**Key Discussions**:
- Usuário confirmou que dados atuais podem ser descartados (banco de desenvolvimento)
- API Sicredi: credenciais ainda não obtidas (implementar com suporte a sandbox)
- NF-e: suportar versões 3.1 e 4.0 do layout XML
- A única integração com AutoOS é leitura opcional da tabela `produtos`

**Research Findings**:
- Banco de dados tem 7 migrações do AutoOS (_sqlx_migrations) + 6 do AutoBO (autobo_migrations)
- O db.rs já usa sistema de migração manual (autobo_migrations) — pode ser mantido
- O banco tem 1 cliente, 0 produtos, 0 saídas de estoque, 0 pagadores
- Tabelas atuais: autobo_pagadores, autobo_boletos, autobo_itens_boleto, autobo_comunicacoes, autobo_configuracoes, autobo_migrations

### Metis Review
**Identified Gaps** (addressed):
- Data loss: resolved — pode descartar tudo (dev database)
- NF-e version: resolved — suportar 3.1 e 4.0
- API Sicredi: resolved — implementar com mock/sandbox
- Encryption: usar AES-256-GCM com chave de env var
- Migration rollback: V200-V205 em autobo_migrations com IF NOT EXISTS/DROP IF EXISTS

---

## Work Objectives

### Core Objective
Construir o AutoBO do zero seguindo o PRD v2: sistema standalone de geração de boletos via importação de NF-e XML ou entrada manual, com integração opcional de leitura de produtos do AutoOS.

### Concrete Deliverables
- Migrations V200-V205 com schema correto
- `nfe_parser.rs` — parser de XML NF-e 3.1 e 4.0
- `commands/nfe.rs` — `importar_nfe`, `importar_nfe_lote`
- `commands/boleto.rs` reescrito — `gerar_boleto` sem `movimento_ids`
- `commands/autoos.rs` — `listar_produtos` (opcional)
- `services/crypto.rs` — criptografia AES-256-GCM
- Frontend atualizado — novo fluxo de criação de boletos

### Definition of Done
- [ ] `cargo build` sem erros
- [ ] `cargo test` passa nos testes de CPF/CNPJ e parser XML
- [ ] App inicia e conecta ao banco sem panic
- [ ] Pagadores podem ser criados/editados/listados sem `cliente_id`
- [ ] Boleto pode ser criado via entrada manual (sem NF-e)
- [ ] NF-e XML pode ser parseado e dados extraídos corretamente
- [ ] Nenhuma referência a `movimentacoes_estoque` em queries do AutoBO
- [ ] Nenhuma referência a `cliente_id` em `autobo_pagadores`
- [ ] `produtos` do AutoOS é lido opcionalmente (sem crash se tabela não existir)
- [ ] Senhas SMTP e tokens WhatsApp são criptografados no banco

### Must Have
- Parser de NF-e XML (versões 3.1 e 4.0)
- Fluxo de geração de boleto via NF-e XML (upload → parse → revisão → gerar)
- Fluxo de geração de boleto manual (preencher dados → gerar)
- Tabela `autobo_nfes` para armazenar XMLs importados
- `autobo_pagadores` com `documento` UNIQUE (sem `cliente_id`)
- `autobo_boletos` com campos `nfe_id` e `origem`
- `autobo_itens_boleto` com `descricao` e `valor_unitario` (sem `movimentacao_id`)
- Criptografia de dados sensíveis nas configurações
- Detecção de tabela `produtos` para habilitar/desabilitar integração AutoOS

### Must NOT Have (Guardrails)
- NÃO modificar tabelas do AutoOS (zero ALTER, INSERT, UPDATE em `movimentacoes_estoque`, `clientes`, etc.)
- NÃO JOIN com `movimentacoes_estoque` ou `clientes`
- NÃO campo `cliente_id` em `autobo_pagadores`
- NÃO campo `movimentacao_id` em `autobo_itens_boleto`
- NÃO campo `movimento_ids` em `BoletoInput`
- NÃO `fila.rs` (no old code to reference — building fresh without it)
- NÃO migration que ALTERa tabela do AutoOS
- NÃO referência a `movimentacoes_estoque` em nenhum comando do AutoBO
- NÃO armazenar senhas/tokens em texto plano no banco

---

## Verification Strategy (MANDATORY)

> **ZERO HUMAN INTERVENTION** — ALL verification is agent-executed. No exceptions.

### Test Decision
- **Infrastructure exists**: YES (vitest configured in package.json)
- **Automated tests**: YES (tests-after) — implementar testes após a funcionalidade
- **Framework**: vitest (frontend), cargo test (backend)
- **Agent-Executed QA**: Playwright para UI, curl para API, cargo test para Rust

### QA Policy
Every task MUST include agent-executed QA scenarios.
Evidence saved to `.sisyphus/evidence/task-{N}-{scenario-slug}.{ext}`.

- **Tauri Commands**: Bash (cargo test) — rodar testes unitários Rust
- **Database**: Bash (psql) — verificar schema, constraints, dados seed
- **Frontend**: Playwright — navegar, interagir, assert DOM
- **NF-e Parser**: Bash (cargo test) — testar parse de XMLs de exemplo

---

## Execution Strategy

### Parallel Execution Waves

```
Wave 0 (Start Immediately — project scaffold):
└── Task 0: Scaffold Tauri project + base dependencies [quick]

Wave 1 (After Wave 0 — foundation + core modules):
├── Task 1: Create migrations V200-V205 [deep]
├── Task 2: Create crypto module [quick]
└── Task 3: Create db.rs with migration system + AutoOS detection [quick]

Wave 2 (After Wave 1 — core business logic, MAX PARALLEL):
├── Task 4: Implement nfe_parser.rs (NF-e XML 3.1 + 4.0) [deep]
├── Task 5: Create pagadores module (schema + commands) [unspecified-high]
├── Task 6: Create boletos + itens_boleto module (schema + commands) [unspecified-high]
├── Task 7: Create nfes module (schema + import commands) [unspecified-high]
├── Task 8: Create configuracoes module with encryption [unspecified-high]
└── Task 9: Create autoos module (optional produtos read) [quick]

Wave 3 (After Wave 2 — integration + frontend):
├── Task 10: Create gerar_boleto command (NF-e + manual flows) [deep]
├── Task 11: Create listar/cancelar boleto commands [unspecified-high]
├── Task 12: Create dashboard + scheduler for new schema [unspecified-high]
├── Task 13: Create frontend types + db.ts for new API [visual-engineering]
└── Task 14: Create NovoBoleto page (NF-e upload + manual) [visual-engineering]

Wave 4 (After Wave 3 — tests + QA):
├── Task 15: E2E test: manual boleto flow [deep]
└── Task 16: E2E test: NF-e import flow [deep]

Wave FINAL (After ALL tasks):
├── F1: Plan compliance audit [oracle]
├── F2: Code quality review [unspecified-high]
├── F3: Real manual QA [unspecified-high]
└── F4: Scope fidelity check [deep]
```

### Dependency Matrix

| Task | Depends On | Blocks |
|------|-----------|--------|
| 0 | - | 1,2,3 |
| 1 | 0 | 4,5,6,7,8 |
| 2 | 0 | 8 |
| 3 | 1 | 5,6,7,8,9 |
| 4 | 1,2 | 10 |
| 5 | 3 | 10,14 |
| 6 | 3 | 10,14 |
| 7 | 3 | 10 |
| 8 | 2,3 | 10 |
| 9 | 3 | 14 |
| 10 | 4,5,6,7,8 | 15,16 |
| 11 | 5,6 | 15 |
| 12 | 5,6,8 | 15 |
| 13 | 5,9 | 14 |
| 14 | 10,13 | 16 |
| 15 | 10,11 | F1-F4 |
| 16 | 10,14 | F1-F4 |

### Agent Dispatch Summary

- **Wave 0**: 1 task — T0→`quick`
- **Wave 1**: 3 tasks — T1→`deep`, T2→`quick`, T3→`quick`
- **Wave 2**: 6 tasks — T4→`deep`, T5-T8→`unspecified-high`, T9→`quick`
- **Wave 3**: 5 tasks — T10→`deep`, T11-T12→`unspecified-high`, T13-T14→`visual-engineering`
- **Wave 4**: 2 tasks — T15→`deep`, T16→`deep`
- **FINAL**: 4 tasks — F1→`oracle`, F2→`unspecified-high`, F3→`unspecified-high`, F4→`deep`

---

## TODOs

- [x] 0. Scaffold Tauri project + base dependencies

  **What to do**:
  - Create Tauri 2.x project with `cargo tauri init` or equivalent
  - Set up `Cargo.toml` with all PRD-required dependencies: tauri 2.x, sqlx 0.7 (postgres, chrono, migrate), tokio, serde, serde_json, quick-xml, lettre, reqwest, tracing, tracing-subscriber, tracing-appender, aes-gcm, base64, sha2, argon2, uuid, directories
  - Create `src-tauri/tauri.conf.json` with identifier `com.bmitag.autobo`
  - Create `src-tauri/src/main.rs` with Tauri setup boilerplate (tracing, IPC handlers)
  - Create `src-tauri/src/db.rs` with manual migration runner (`autobo_migrations` table approach, per PRD section 3.3 principle: "Migrations versionadas")
  - Create `src-tauri/src/services/mod.rs` and `src-tauri/src/commands/mod.rs` module stubs
  - Create `package.json` with React 18 + TypeScript + Tailwind + shadcn/ui (copy from AutoOS pattern per PRD section 9.1)
  - Create `src/` directory structure per PRD section 13: `src/components/ui/`, `src/components/boleto/`, `src/components/nfe/`, `src/components/pagador/`, `src/hooks/`, `src/pages/`, `src/types/`, `src/lib/`
  - Create `src-tauri/migrations/` directory (empty initially, Task 1 populates it)

  **Must NOT do**:
  - Do NOT include any old AutoOS-coupled code (no `movimentacoes_estoque`, `clientes`, `fila.rs`)

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO — this is the foundation
  - **Parallel Group**: Wave 0
  - **Blocks**: All subsequent tasks
  - **Blocked By**: None

  **References**:
  **API/Type References**:
  - `PRD_AutoBO_v2.md` section 3.1 — Stack table (all dependencies)
  - `PRD_AutoBO_v2.md` section 13 — Folder structure
  - `PRD_AutoBO_v2.md` section 3.3 — Migrations versionadas (V2xx prefix)

  **Acceptance Criteria**:
  - [ ] `cargo build` in `src-tauri/` succeeds
  - [ ] `npm install` in project root succeeds
  - [ ] `src-tauri/src/main.rs` exists and compiles
  - [ ] `src-tauri/src/db.rs` exists with migration runner function
  - [ ] `src-tauri/migrations/` directory exists
  - [ ] Frontend scaffold compiles (`npm run dev` starts)

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Project builds and runs
    Tool: Bash
    Steps:
      1. cd src-tauri && cargo build
      2. cd .. && npm install
    Expected Result: Both commands succeed without errors
    Failure Indicators: Compilation errors, missing dependencies
    Evidence: .sisyphus/evidence/task-0-scaffold-build.txt

  Scenario: Migration runner is functional
    Tool: Bash
    Steps:
      1. Verify src-tauri/src/db.rs contains `run_migrations_manual` function
      2. Verify `autobo_migrations` table creation SQL exists
    Expected Result: Migration system ready to create tables
    Failure Indicators: Missing function, wrong SQL
    Evidence: .sisyphus/evidence/task-0-migration-runner.txt
  ```

  **Commit**: YES
  - Message: `feat(init): scaffold AutoBO Tauri project per PRD v2`
  - Files: Cargo.toml, package.json, src-tauri/*, src/*

- [x] 1. Create migrations V200-V205

  **What to do**:
  - Create 6 new migration files following PRD v2 schema:
    - `0200__autobo_pagadores.sql` — `documento` as UNIQUE, NO `cliente_id`, add `nome`, `razao_social`, `nome_fantasia` per PRD
    - `0201__autobo_nfes.sql` — new table for NF-e storage with `chave_acesso` UNIQUE, `status`, `boleto_id`
    - `0202__autobo_boletos.sql` — add `nfe_id`, `origem` (NFE/MANUAL), `nosso_numero` NULLABLE initially; remove fields that reference estoque
    - `0203__autobo_itens_boleto.sql` — replace `movimentacao_id` with `descricao`, `valor_unitario`, `produto_autoos_id` (nullable); remove `produto_codigo`, `subtotal`, `ordem_servico_ref`
    - `0204__autobo_comunicacoes.sql` — add `tipo` enum values (BOLETO_GERADO, LEMBRETE_VENCIMENTO, AVISO_VENCIDO)
    - `0205__autobo_configuracoes.sql` — add `empresa.cnpj` and encryption-ready seed data
  - All tables use `autobo_` prefix and `IF NOT EXISTS` / `DROP IF EXISTS` for safety
  - Each migration must be idempotent (can run twice without error)

  **Must NOT do**:
  - Do NOT reference `movimentacoes_estoque`, `clientes`, or any AutoOS table in new migrations
  - Do NOT create migration 0100-style (ALTER on AutoOS tables)

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 2, 4)
  - **Parallel Group**: Wave 1
  - **Blocks**: Tasks 4, 5, 6, 7, 8
  - **Blocked By**: Task 0

  **References**:
  **API/Type References**:
  - `PRD_AutoBO_v2.md` sections 5.1-5.6 — Exact table schemas
  - `PRD_AutoBO_v2.md` section 3.3 — Migration prefix V2xx, no collision with V1xx

  **Acceptance Criteria**:
  - [ ] 6 migration files exist in `src-tauri/migrations/` with prefix 0200-0205
  - [ ] No `cliente_id` in `autobo_pagadores`
  - [ ] `autobo_nfes` table has `chave_acesso VARCHAR(44) UNIQUE`
  - [ ] `autobo_boletos` has `nfe_id BIGINT REFERENCES autobo_nfes(id)` and `origem VARCHAR(20) NOT NULL DEFAULT 'NFE'`
  - [ ] `autobo_itens_boleto` has `descricao`, `valor_unitario`, `produto_autoos_id BIGINT` (nullable) and does NOT have `movimentacao_id`
  - [ ] No ALTER on any AutoOS table
  - [ ] `autobo_configuracoes` has `empresa.cnpj` seed row

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Migrations run successfully on clean database
    Tool: Bash (psql)
    Preconditions: AutoOS database exists with _sqlx_migrations
    Steps:
      1. Delete old autobo_* tables if they exist: DROP TABLE IF EXISTS autobo_pagadores, autobo_boletos, autobo_itens_boleto, autobo_comunicacoes, autobo_configuracoes CASCADE
      2. Delete old autobo_migrations rows: DELETE FROM autobo_migrations WHERE version >= 100
      3. Run AutoBO app: cargo run
      4. Query: SELECT version, description FROM autobo_migrations ORDER BY version
    Expected Result: Rows with versions 200, 201, 202, 203, 204, 205 present
    Failure Indicators: Missing versions, SQL errors, panic on startup
    Evidence: .sisyphus/evidence/task-1-migrations-clean-db.txt

  Scenario: Migrations are idempotent (running twice doesn't error)
    Tool: Bash (psql)
    Preconditions: Migrations already applied once
    Steps:
      1. Delete autobo_migrations rows: DELETE FROM autobo_migrations
      2. Run AutoBO app again
      3. Query: SELECT COUNT(*) FROM autobo_pagadores (should succeed, table exists)
    Expected Result: App starts without panic, tables still exist (IF NOT EXISTS)
    Failure Indicators: SQL errors on second run, duplicate key errors
    Evidence: .sisyphus/evidence/task-1-idempotent-migrations.txt
  ```

  **Commit**: YES (groups with Wave 1)
  - Message: `feat(migrations): create V200-V205 migrations per PRD v2`
  - Files: `src-tauri/migrations/0200__*.sql` through `0205__*.sql`
  - Pre-commit: `cargo check`

- [x] 2. Create crypto module (quick-xml already in scaffold)

  **What to do**:
  - `quick-xml` dependency was already added in Task 0 scaffold — verify it's present in `Cargo.toml`
  - Add `aes-gcm` dependency to `Cargo.toml` for AES-256-GCM encryption
  - Create `src-tauri/src/services/crypto.rs` with functions:
    - `encrypt(plaintext: &str, key: &[u8]) -> Result<String, CryptoError>` — encrypt with AES-256-GCM, return base64
    - `decrypt(ciphertext: &str, key: &[u8]) -> Result<String, CryptoError>` — decrypt base64 ciphertext
    - `get_encryption_key() -> Vec<u8>` — read key from `AUTOBO_ENCRYPTION_KEY` env var, generate if missing
  - All sensitive fields (`smtp.senha`, `whatsapp.token`, `sicredi.api_key`) will use encrypt/decrypt
  - Add `keyring` dependency removal check (currently in Cargo.toml but not used for this)

  **Must NOT do**:
  - Do NOT hardcode encryption keys
  - Do NOT use reversible XOR or ROT13 "encryption"

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 1, 3, 4)
  - **Parallel Group**: Wave 1
  - **Blocks**: Tasks 5, 9
  - **Blocked By**: None

  **References**:
  **API/Type References**:
  - `PRD_AutoBO_v2.md` section 10.3 — "Dados Sensíveis" requirement
  - `PRD_AutoBO_v2.md` section 3.1 — Stack dependencies (Cargo.toml)
  - `src-tauri/src/services/crypto.rs` — File to be created (from Task 0 scaffold)

  **Acceptance Criteria**:
  - [ ] `quick-xml` present in `Cargo.toml` (from scaffold)
  - [ ] `aes-gcm` added to `Cargo.toml`
  - [ ] `src-tauri/src/services/crypto.rs` exists with `encrypt` and `decrypt` functions
  - [ ] `encrypt("hello", key)` produces base64 string, `decrypt(encrypt("hello", key), key)` returns `"hello"`
  - [ ] `get_encryption_key()` reads from env var

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Encryption round-trip works
    Tool: Bash (cargo test)
    Steps:
      1. Run: cargo test crypto
    Expected Result: All crypto tests pass (encrypt → decrypt round-trip)
    Failure Indicators: Test failures, panic, wrong decrypted value
    Evidence: .sisyphus/evidence/task-2-crypto-tests.txt

  Scenario: Invalid ciphertext fails gracefully
    Tool: Bash (cargo test)
    Steps:
      1. Run: cargo test crypto_invalid
    Expected Result: Decrypt of invalid data returns error (not panic)
    Failure Indicators: Panic on invalid input
    Evidence: .sisyphus/evidence/task-2-crypto-error-handling.txt
  ```

  **Commit**: YES (groups with Wave 1)
  - Message: `feat(crypto): create crypto module with AES-256-GCM encryption`
  - Files: `Cargo.toml`, `src-tauri/src/services/crypto.rs`, `src-tauri/src/services/mod.rs`

- [x] 3. Create db.rs migration system with V2xx + AutoOS detection

  **What to do**:
  - The Task 0 scaffold creates a basic `db.rs` with migration runner — this task expands it for V2xx support
  - Migration version parsing: filenames like `0200__autobo_pagadores.sql` → version `200`
  - The migration runner must handle the old 0100-0105 entries in `autobo_migrations` table (skip them since files are gone)
  - Add `verificar_integracao_autoos` function per PRD section 4.4: `SELECT 1 FROM information_schema.tables WHERE table_name = 'produtos'`
  - Expose `produtos_disponivel()` as a public function for the frontend to check

  **Must NOT do**:
  - Do NOT switch back to `sqlx::migrate!` macro (it conflicts with AutoOS)
  - Do NOT touch `_sqlx_migrations` table

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO — depends on Task 1 (migrations exist)
  - **Parallel Group**: Wave 1 (after Task 1)
  - **Blocks**: Tasks 5, 6, 7, 8
  - **Blocked By**: Task 1

  **References**:
  **Pattern References**:
  - `PRD_AutoBO_v2.md` section 3.3 — Migrations versionadas (V2xx prefix, `autobo_migrations` table)

  **API/Type References**:
  - `PRD_AutoBO_v2.md` section 4.4 — `verificar_integracao_autoos` function

  **Acceptance Criteria**:
  - [ ] Migration files `0200__*.sql` through `0205__*.sql` are read and executed in order
  - [ ] `verificar_integracao_autoos()` returns `true` when `produtos` table exists
  - [ ] `verificar_integracao_autoos()` returns `false` gracefully when `produtos` table doesn't exist
  - [ ] App starts without panic even with old 0100-0105 entries in `autobo_migrations`

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: App starts with old migration history
    Tool: Bash (cargo run)
    Preconditions: autobo_migrations has entries for 0100-0105
    Steps:
      1. Run cargo run
      2. Check logs for "Banco de dados inicializado com sucesso"
      3. Query: SELECT version FROM autobo_migrations ORDER BY version
    Expected Result: App starts without panic. New versions 200-205 are added alongside old 100-105 entries.
    Failure Indicators: Panic on startup, migration errors
    Evidence: .sisyphus/evidence/task-3-old-history-compat.txt
  ```

  **Commit**: YES (groups with Wave 1)
  - Message: `refactor(db): update migration system for V2xx and add AutoOS detection`
  - Files: `src-tauri/src/db.rs`

- [x] 4. Implement nfe_parser.rs (NF-e XML 3.1 + 4.0)

  **What to do**:
  - Create `src-tauri/src/services/nfe_parser.rs`
  - Implement `parse_nfe_xml(xml_content: &str) -> Result<NFeDados, NFeError>` that:
    - Detects NF-e version (3.1 or 4.0) from XML namespace
    - Extracts: `numero_nf`, `serie`, `chave_acesso`, `data_emissao`, `valor_total`, `valor_produtos`, `natureza_operacao`
    - Extracts destinatário: `documento` (CPF/CNPJ), `nome`/`razao_social`, `email`, `telefone`, endereço completo
    - Extracts items: `descricao` (xProd), `quantidade` (qCom), `unidade` (uCom), `valor_unitario` (vUnCom), `subtotal` (vProd)
  - Define `NFeDados` and `NFeItem` structs
  - Implement validations per PRD section 6.1:
    - Chave de acesso: 44 digits, numeric
    - Chave not duplicated in `autobo_nfes`
    - CPF/CNPJ mathematically valid (reuse existing validators)
    - Valor total > 0
    - Data emissão not future
    - XML namespace valid
  - Implement `importar_nfe` command (returns DTO, does NOT save to DB yet per PRD)
  - Implement `importar_nfe_lote` for batch processing
  - Add `quick-xml` usage for XML parsing with namespace handling

  **Must NOT do**:
  - Do NOT connect to SEFAZ or external services
  - Do NOT generate DANFE or PDF
  - Do NOT validate against SEFAZ schema beyond well-formedness

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO — depends on Tasks 1, 2
  - **Parallel Group**: Wave 2
  - **Blocks**: Task 11
  - **Blocked By**: Tasks 1, 2

  **References**:
  **API/Type References**:
  - `PRD_AutoBO_v2.md` section 6.1 — NF-e parser specification
  - `PRD_AutoBO_v2.md` section 5.2 — `autobo_nfes` table schema
  - `src-tauri/src/commands/boleto.rs` — Existing CPF/CNPJ validators (reuse `validar_cpf`, `validar_cnpj`)

  **Acceptance Criteria**:
  - [ ] `parse_nfe_xml` correctly extracts all fields from a NF-e 4.0 XML
  - [ ] `parse_nfe_xml` correctly extracts all fields from a NF-e 3.1 XML
  - [ ] Invalid XML returns descriptive error without panic
  - [ ] Duplicate chave detection query works
  - [ ] `cargo test nfe_parser` passes all tests

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Parse valid NF-e 4.0 XML
    Tool: Bash (cargo test)
    Steps:
      1. Run: cargo test nfe_parser
    Expected Result: All parser tests pass. Fields extracted correctly from sample XML.
    Failure Indicators: Missing fields, wrong values, parse errors
    Evidence: .sisyphus/evidence/task-5-nfe-parser-tests.txt

  Scenario: Reject malformed XML
    Tool: Bash (cargo test)
    Steps:
      1. Run: cargo test nfe_parser_invalid
    Expected Result: Invalid XML returns descriptive NFeError, no panic
    Failure Indicators: Panic, unwrap on None
    Evidence: .sisyphus/evidence/task-5-nfe-parser-error.txt
  ```

  **Commit**: YES
  - Message: `feat(nfe): implement NF-e XML parser for versions 3.1 and 4.0`
  - Files: `src-tauri/src/services/nfe_parser.rs`, `src-tauri/src/services/mod.rs`, `src-tauri/src/commands/nfe.rs`, `src-tauri/src/commands/mod.rs`

- [x] 5. Create pagadores module (schema + commands)

  **What to do**:
  - Create `commands/pagador.rs` implementing PRD v2 pagadores:
    - `PagadorInput` and `PagadorRow` types in `commands/types.rs`: `documento` UNIQUE, `nome`, `razao_social`, `nome_fantasia`, `email`, `telefone` -- NO `cliente_id`
    - `listar_pagadores`: query only `autobo_pagadores`, no JOIN with `clientes`
    - `buscar_pagador`: by `documento` or `id`, no JOIN with `clientes`
    - `salvar_pagador`: UPSERT by `documento`, no `cliente_id`
  - No `fila.rs` module (PRD v2 has no "fila de faturamento" concept)
  - Register pagador commands in `main.rs` invoke_handler

  **Must NOT do**:
  - Do NOT JOIN with `clientes` or `movimentacoes_estoque`
  - Do NOT reference `cliente_id` anywhere

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 4, 6, 7, 8)
  - **Parallel Group**: Wave 2
  - **Blocks**: Tasks 10, 14
  - **Blocked By**: Task 3

  **References**:
  **API/Type References**:
  - `PRD_AutoBO_v2.md` section 5.1 — Exact pagadores schema
  - `PRD_AutoBO_v2.md` section 6.7 — Pagadores functionality

  **Acceptance Criteria**:
  - [ ] `PagadorInput` has no `cliente_id` field
  - [ ] `PagadorRow` has `documento` UNIQUE, `nome`, `razao_social`, `nome_fantasia`
  - [ ] `listar_pagadores` query has no JOIN with `clientes`
  - [ ] `buscar_pagador` by documento works without `clientes` table
  - [ ] `fila.rs` deleted and removed from `mod.rs` and `main.rs`
  - [ ] `cargo build` succeeds

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Create pagador without cliente_id
    Tool: Bash (cargo test + psql)
    Steps:
      1. INSERT a pagador with documento '12345678000190' and nome 'Test LTDA'
      2. Query: SELECT * FROM autobo_pagadores WHERE documento = '12345678000190'
    Expected Result: Pagador created, no cliente_id column exists
    Failure Indicators: cliente_id NOT NULL constraint, missing column
    Evidence: .sisyphus/evidence/task-6-pagador-create.txt

  Scenario: No references to clientes or movimentacoes_estoque
    Tool: Bash (grep)
    Steps:
      1. grep -r "cliente_id" src-tauri/src/commands/pagador.rs
      2. grep -r "JOIN clientes" src-tauri/src/commands/
    Expected Result: 0 matches
    Failure Indicators: Any match found
    Evidence: .sisyphus/evidence/task-6-no-cliente-ref.txt
  ```

  **Commit**: YES
  - Message: `feat(pagadores): create pagadores module with documento as unique key per PRD v2`
  - Files: `src-tauri/src/commands/types.rs`, `src-tauri/src/commands/pagador.rs`, `src-tauri/src/commands/mod.rs`, `src-tauri/src/main.rs`

- [x] 6. Create boletos + itens_boleto module (types + commands)

  **What to do**:
  - Create `BoletoInput` in `commands/types.rs`:
    - `origem: String` (NFE or MANUAL), `nfe_id: Option<i64>`, `itens: Vec<ItemInput>` — NO `movimento_ids`
    - `ItemInput` struct: `descricao`, `quantidade`, `unidade`, `valor_unitario`, `subtotal`, `produto_autoos_id: Option<i64>`
  - Create `BoletoRow`: add `nfe_id`, `origem`, `nosso_numero` (nullable)
  - Create `ItemBoletoRow`: `descricao`, `valor_unitario`, `produto_autoos_id` (nullable) — NO `movimentacao_id`
  - Create SQL constants (`BOLETO_SELECT`, `ITEM_BOLETO_SELECT`) matching new schema
  - Create `autobo_comunicacoes` types per PRD

  **Must NOT do**:
  - Do NOT reference `movimentacoes_estoque` or `movimentacao_id`
  - Do NOT reference `produtos` in item insertion (use `produto_autoos_id` as nullable reference)

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 4, 5, 7, 8)
  - **Parallel Group**: Wave 2
  - **Blocks**: Tasks 10, 14
  - **Blocked By**: Task 3

  **References**:
  **API/Type References**:
  - `PRD_AutoBO_v2.md` section 5.3 — `autobo_boletos` schema
  - `PRD_AutoBO_v2.md` section 5.4 — `autobo_itens_boleto` schema

  **Acceptance Criteria**:
  - [ ] `BoletoInput` has no `movimento_ids` field
  - [ ] `BoletoInput` has `nfe_id: Option<i64>` and `origem: String`
  - [ ] `ItemInput` has `descricao`, `valor_unitario`, `produto_autoos_id: Option<i64>`
  - [ ] `ItemBoletoRow` has no `movimentacao_id`
  - [ ] `cargo build` succeeds

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Types compile without movimento_ids
    Tool: Bash (cargo check)
    Steps:
      1. Run: cargo check
    Expected Result: Compiles without errors. No reference to movimento_ids in types.
    Failure Indicators: Compilation error about missing fields
    Evidence: .sisyphus/evidence/task-7-types-compile.txt
  ```

  **Commit**: YES
  - Message: `feat(types): create boleto and item types for PRD v2 schema`
  - Files: `src-tauri/src/commands/types.rs`

- [x] 7. Create nfes module (schema + import commands)

  **What to do**:
  - Create `NFeDados`, `NFeImportadaDTO` structs in `commands/types.rs`
  - Create `commands/nfe.rs` with:
    - `importar_nfe(xml_base64: String) -> Result<NFeImportadaDTO, String>` — parse XML, validate, return DTO (NO DB save)
    - `importar_nfe_lote(xmls_base64: Vec<String>) -> Result<Vec<NFeImportadaDTO>, String>` — batch parse
  - Register commands in `main.rs`: `importar_nfe`, `importar_nfe_lote`
  - The `importar_nfe` command ONLY parses and validates — it does NOT save to DB (per PRD section 6.1)

  **Must NOT do**:
  - Do NOT save NFe data in `importar_nfe` (only parse and return DTO)
  - Do NOT call Sicredi API in this command

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 4, 5, 6, 8)
  - **Parallel Group**: Wave 2
  - **Blocks**: Task 10
  - **Blocked By**: Task 3

  **References**:
  **API/Type References**:
  - `PRD_AutoBO_v2.md` section 6.1 — Parser specification, validation rules, command signature
  - `PRD_AutoBO_v2.md` section 5.2 — `autobo_nfes` table schema

  **Acceptance Criteria**:
  - [ ] `importar_nfe` command registered in `main.rs`
  - [ ] Returns parsed DTO without saving to database
  - [ ] Validates: 44-digit chave, CPF/CNPJ, valor > 0, data not future
  - [ ] Returns descriptive error for invalid XML
  - [ ] `importar_nfe_lote` processes multiple XMLs

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Import valid NF-e returns parsed data
    Tool: Bash (cargo test)
    Steps:
      1. cargo test nfe_import
    Expected Result: Parsed DTO contains numero_nf, chave_acesso, valor_total, destinatario data, items
    Failure Indicators: Missing fields, parse error
    Evidence: .sisyphus/evidence/task-8-nfe-import.txt

  Scenario: Import invalid XML returns error
    Tool: Bash (cargo test)
    Steps:
      1. cargo test nfe_import_invalid
    Expected Result: Descriptive error message, not panic
    Failure Indicators: Panic, unwrap on None
    Evidence: .sisyphus/evidence/task-8-nfe-import-error.txt
  ```

  **Commit**: YES
  - Message: `feat(nfe): add importar_nfe and importar_nfe_lote commands`
  - Files: `src-tauri/src/commands/nfe.rs`, `src-tauri/src/commands/mod.rs`, `src-tauri/src/main.rs`

- [x] 8. Create configuracoes module with encryption

  **What to do**:
  - Create `commands/configuracoes.rs` implementing:
    - Encrypt `smtp.senha`, `whatsapp.token`, `sicredi.api_key` on write using `crypto::encrypt`
    - Decrypt on read using `crypto::decrypt`
    - Detect if a value is encrypted (prefix marker like `ENC:` or check if base64)
    - Handle migration from plaintext to encrypted values
  - Create `services/email.rs` to decrypt SMTP password before use
  - Create `services/whatsapp.rs` to decrypt WhatsApp token before use
  - Update `services/sicredi.rs` to decrypt API key before use
  - The password fields in frontend should use `type="password"` with reveal button (already in PRD)

  **Must NOT do**:
  - Do NOT store encryption key in source code
  - Do NOT log decrypted passwords/tokens

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 4, 5, 6, 7)
  - **Parallel Group**: Wave 2
  - **Blocks**: Task 10
  - **Blocked By**: Tasks 2, 3

  **References**:
  **API/Type References**:
  - `PRD_AutoBO_v2.md` section 10.3 — "Dados Sensíveis" requirement
  - `src-tauri/src/services/crypto.rs` — Encryption functions (created in Task 2)

  **Acceptance Criteria**:
  - [ ] `smtp.senha` is encrypted in database (not plaintext)
  - [ ] `whatsapp.token` is encrypted in database
  - `sicredi.api_key` is encrypted in database
  - [ ] Decrypted values are used correctly by email, WhatsApp, and Sicredi services
  - [ ] Old plaintext values are auto-migrated on first read

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Password is stored encrypted
    Tool: Bash (psql)
    Steps:
      1. Set smtp.senha via set_config command
      2. Query: SELECT valor FROM autobo_configuracoes WHERE chave = 'smtp.senha'
    Expected Result: Value does NOT contain the plaintext password, starts with encryption marker
    Failure Indicators: Plaintext password visible in database
    Evidence: .sisyphus/evidence/task-9-encryption-stored.txt

  Scenario: Decrypted password works for SMTP
    Tool: Bash (cargo test)
    Steps:
      1. Set encrypted password via config
      2. Test email sending with testar_envio_email command
    Expected Result: Email service receives decrypted password, no error
    Failure Indicators: Decryption failure, auth error
    Evidence: .sisyphus/evidence/task-9-decrypt-smtp.txt
  ```

  **Commit**: YES
  - Message: `feat(config): encrypt sensitive configuration values per PRD v2`
  - Files: `src-tauri/src/commands/configuracoes.rs`, `src-tauri/src/services/email.rs`, `src-tauri/src/services/whatsapp.rs`, `src-tauri/src/services/sicredi.rs`

- [x] 9. Create autoos module (optional produtos read)

  **What to do**:
  - Create `src-tauri/src/commands/autoos.rs` with:
    - `listar_produtos_autoos() -> Result<Vec<ProdutoAutoOS>, String>` — reads `produtos` table IF it exists
    - Use `verificar_integracao_autoos()` from `db.rs` to check if table exists
    - If `produtos` doesn't exist: return empty Vec (graceful, no error)
    - If exists: `SELECT id, codigo, nome, unidade, preco_unitario, estoque_atual, estoque_minimo, categoria FROM produtos WHERE ativo = TRUE ORDER BY nome ASC`
  - Create `ProdutoAutoOS` struct
  - Register command in `main.rs`
  - This is INFORMATIONAL ONLY — never used in boleto creation flow

  **Must NOT do**:
  - Do NOT JOIN with any other AutoOS table
  - Do NOT modify `produtos` table
  - Do NOT fail if `produtos` doesn't exist

  **Recommended Agent Profile**:
  - **Category**: `quick`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO — depends on Task 3 (db.rs)
  - **Parallel Group**: Wave 2
  - **Blocks**: Task 14
  - **Blocked By**: Task 3

  **References**:
  **API/Type References**:
  - `PRD_AutoBO_v2.md` section 4.1-4.4 — AutoOS integration specification
  - `PRD_AutoBO_v2.md` section 4.2 — Exact SQL query for produtos

  **Acceptance Criteria**:
  - [ ] `listar_produtos_autoos` returns products when `produtos` table exists
  - [ ] `listar_produtos_autoos` returns empty Vec when `produtos` table doesn't exist
  - [ ] No crash or error when AutoOS is not installed
  - [ ] Registered in `main.rs` invoke_handler

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Products list when AutoOS available
    Tool: Bash (psql + cargo run)
    Steps:
      1. Query: SELECT * FROM produtos WHERE ativo = TRUE (confirm data exists)
      2. Call listar_produtos_autoos from frontend
    Expected Result: Returns array of ProdutoAutoOS objects
    Failure Indicators: Error, empty array when data exists
    Evidence: .sisyphus/evidence/task-10-produtos-list.txt

  Scenario: Graceful handling when AutoOS not available
    Tool: Bash (psql)
    Steps:
      1. ALTER TABLE produtos RENAME TO produtos_backup
      2. Call listar_produtos_autoos
    Expected Result: Returns empty Vec, no crash
    Failure Indicators: Panic, SQL error
    Evidence: .sisyphus/evidence/task-10-produtos-graceful.txt
  ```

  **Commit**: YES
  - Message: `feat(autoos): add optional produtos read from AutoOS per PRD v2`
  - Files: `src-tauri/src/commands/autoos.rs`, `src-tauri/src/commands/mod.rs`, `src-tauri/src/main.rs`

- [x] 10. Create gerar_boleto command (NF-e + manual flows)

  **What to do**:
  - Create `commands/boleto.rs` implementing PRD v2 section 6.3:
    - Flow A (NF-e): receive `nfe_id` + pagador data + items from XML parse → create boleto
    - Flow B (Manual): receive pagador data + items manually → create boleto
    - `gerar_boleto` takes `BoletoInput` with `origem` (NFE or MANUAL), `nfe_id: Option<i64>`, `itens: Vec<ItemInput>`
    - NO `movimento_ids` parameter
  - Sequence per PRD 6.3:
    1. Validate required fields
    2. Validate CPF/CNPJ
    3. Validate data_vencimento >= CURRENT_DATE
    4. Upsert pagador (by documento) — create if new, update contact if changed
    5. If NF-e: insert into `autobo_nfes` with status 'IMPORTADA'
    6. Insert boleto with status 'RASCUNHO', generate `seu_numero`
    7. Insert items into `autobo_itens_boleto`
    8. Call Sicredi API
    9. If success: update boleto status to 'REGISTRADO', update NF-e if applicable
    10. Send notifications (email + WhatsApp)
  - Update `gerar_boleto` call in Sicredi to use new item structure
  - Implement `retentar_rascunho` command for re-processing failed boletos
  - Update `listar_boletos` to include `origem` field
  - Update `buscar_boleto` to return NF-e info if applicable

  **Must NOT do**:
  - Do NOT reference `movimentacoes_estoque` or `produtos` in boleto creation
  - Do NOT require `movimento_ids` parameter
  - Do NOT UPDATE any AutoOS table

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: NO — depends on Tasks 4, 5, 6, 7, 8
  - **Parallel Group**: Wave 3
  - **Blocks**: Tasks 15, 16
  - **Blocked By**: Tasks 4, 5, 6, 7, 8

  **References**:
  **API/Type References**:
  - `PRD_AutoBO_v2.md` section 6.3 — Complete generation sequence (steps 1-16)
  - `PRD_AutoBO_v2.md` section 5.3 — `autobo_boletos` schema
  - `PRD_AutoBO_v2.md` section 5.4 — `autobo_itens_boleto` schema
  - `src-tauri/src/services/sicredi.rs` — Existing Sicredi integration

  **Acceptance Criteria**:
  - [ ] `BoletoInput` has `origem`, `nfe_id`, `itens` fields (no `movimento_ids`)
  - [ ] `gerar_boleto` creates pagador by `documento` (not `cliente_id`)
  - [ ] `gerar_boleto` inserts NF-e record if `origem = 'NFE'`
  - [ ] `gerar_boleto` generates `seu_numero` as `NF-{numero_nf}` or `MAN-{YYYYMMDD}-{id}`
  - [ ] `gerar_boleto` inserts items from `itens` vec (no estoque reference)
  - [ ] `retentar_rascunho` command exists
  - [ ] `cargo build` succeeds

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Create manual boleto without NF-e
    Tool: Bash (cargo test + psql)
    Steps:
      1. Create pagador with documento '12345678000190'
      2. Call gerar_boleto with origem='MANUAL', pagador_id, items=[{descricao: 'Teste', quantidade: 1, valor_unitario: 100.00}]
      3. Query: SELECT * FROM autobo_boletos WHERE origem = 'MANUAL'
    Expected Result: Boleto created with status 'RASCUNHO' (Sicredi will fail in test), origem='MANUAL', seu_numero starts with 'MAN-'
    Failure Indicators: Missing fields, foreign key errors, estoque references
    Evidence: .sisyphus/evidence/task-11-manual-boleto.txt

  Scenario: No reference to movimentacoes_estoque
    Tool: Bash (grep)
    Steps:
      1. grep -r "movimentacoes_estoque" src-tauri/src/commands/boleto.rs
      2. grep -r "movimento_ids" src-tauri/src/commands/boleto.rs
    Expected Result: 0 matches
    Failure Indicators: Any match found
    Evidence: .sisyphus/evidence/task-11-no-estoque-ref.txt
  ```

  **Commit**: YES
  - Message: `feat(boleto): create gerar_boleto for NF-e and manual flows per PRD v2`
  - Files: `src-tauri/src/commands/boleto.rs`, `src-tauri/src/commands/types.rs`

- [x] 11. Create listar/cancelar boleto commands

  **What to do**:
  - Create `listar_boletos` command including `origem` and `nfe_id` fields in response
  - Create `buscar_boleto` command including NF-e info if `nfe_id` is present
  - Create `cancelar_boleto` command per PRD section 7.5 (call Sicredi DELETE API)
  - Add `nfe` field to `BoletoDetalhe` response with NF-e data if available
  - No JOIN with `clientes` or `movimentacoes_estoque`
  - Create dashboard metrics command using `origem` field

  **Must NOT do**:
  - Do NOT JOIN with `clientes` or `movimentacoes_estoque`

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 10, 12, 13)
  - **Parallel Group**: Wave 3
  - **Blocks**: Task 15
  - **Blocked By**: Tasks 5, 6

  **References**:
  **API/Type References**:
  - `PRD_AutoBO_v2.md` section 7.5 — Cancelamento de boletos
  - `PRD_AutoBO_v2.md` section 6.6 — Dashboard specification

  **Acceptance Criteria**:
  - [ ] `listar_boletos` returns `origem` field
  - [ ] `buscar_boleto` returns NF-e info when `nfe_id` is present
  - [ ] `cancelar_boleto` calls Sicredi DELETE API (not just local status change)
  - [ ] Dashboard metrics work with new schema

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: List boletos includes origem field
    Tool: Bash (cargo test)
    Steps:
      1. Create a manual boleto
      2. Call listar_boletos
    Expected Result: Response includes origem field ('MANUAL' or 'NFE')
    Failure Indicators: Missing origem field, SQL error
    Evidence: .sisyphus/evidence/task-12-listar-boletos.txt
  ```

  **Commit**: YES
  - Message: `feat(boleto): create listar/buscar/cancelar commands per PRD v2 schema`
  - Files: `src-tauri/src/commands/boleto.rs`

- [x] 12. Create dashboard + scheduler for new schema

  **What to do**:
  - Create `dashboard.rs`:
    - No `movimentacoes_estoque` query for `itens_pendentes_fila`
    - `itens_pendentes_fila` as count of boletos with status 'RASCUNHO' (or remove if not relevant)
    - Top devedores query uses `autobo_pagadores` without JOIN `clientes`
  - Create `scheduler.rs`:
    - No references to `movimentacoes_estoque`
    - Job 1 (status update): query Sicredi API for registered boletos
    - Job 2 (lembrete D-1): query boletos with vencimento = CURRENT_DATE + 1 day
    - Job 3 (aviso D+1): query boletos with vencimento = CURRENT_DATE - 1 day
    - Job 4 (marca vencidos): update status to VENCIDO where vencimento < CURRENT_DATE

  **Must NOT do**:
  - Do NOT query `movimentacoes_estoque` or `clientes`

  **Recommended Agent Profile**:
  - **Category**: `unspecified-high`
  - **Skills**: []

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 10, 11, 13)
  - **Parallel Group**: Wave 3
  - **Blocks**: Task 15
  - **Blocked By**: Tasks 5, 6, 8

  **References**:
  **API/Type References**:
  - `PRD_AutoBO_v2.md` section 11 — Jobs specification (4 jobs)
  - `PRD_AutoBO_v2.md` section 6.6 — Dashboard specification

  **Acceptance Criteria**:
  - [ ] Dashboard works with new schema (no estoque references)
  - [ ] 4 scheduler jobs run without `movimentacoes_estoque` references
  - [ ] `cargo build` succeeds

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Dashboard loads without estoque
    Tool: Bash (cargo run)
    Steps:
      1. Start app
      2. Call metricas_dashboard
    Expected Result: Returns metrics without error
    Failure Indicators: SQL error about missing column, panic
    Evidence: .sisyphus/evidence/task-13-dashboard.txt
  ```

  **Commit**: YES
  - Message: `feat(dashboard+scheduler): create dashboard and jobs per PRD v2`
  - Files: `src-tauri/src/commands/dashboard.rs`, `src-tauri/src/jobs/scheduler.rs`

- [x] 13. Create frontend types + db.ts for new API

  **What to do**:
  - Create `src/types/index.ts` with PRD v2 types:
    - `Pagador` type: `documento` UNIQUE, `nome`, `razao_social`, `nome_fantasia`, `email`, `telefone` — NO `cliente_id`
    - `Boleto` type: add `origem`, `nfe_id`
    - `NFeImportada` type (from parser result)
    - `ItemBoletoInput` type (descricao, quantidade, unidade, valor_unitario)
    - `ProdutoAutoOS` type
    - NO `MovimentacaoFaturavel`, `GrupoCliente`, `MovimentacaoFaturavelRow` types
  - Create `src/lib/db.ts` with Tauri invoke calls:
    - `importarNfe`, `importarNfeLote`, `listarProdutosAutoos`
    - `gerarBoleto` accepts `origem`, `nfeId`, `itens` — NOT `movimentoIds`
    - NO `listarFila` invoke call

  **Must NOT do**:
  - Do NOT reference `movimentacoes_estoque` in any frontend file

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
  - **Skills**: [`playwright`]

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Tasks 10, 11, 12)
  - **Parallel Group**: Wave 3
  - **Blocks**: Task 14
  - **Blocked By**: Tasks 5, 9

  **References**:
  **API/Type References**:
  - `PRD_AutoBO_v2.md` section 5.1-5.6 — Exact table schemas
  - `PRD_AutoBO_v2.md` section 9.1 — Tech stack

  **Acceptance Criteria**:
  - [ ] No `MovimentacaoFaturavel` type in `index.ts`
  - [ ] No `cliente_id` in `Pagador` type
  - [ ] `gerarBoleto` function accepts `origem`, `itens` (not `movimentoIds`)
  - [ ] `importarNfe`, `importarNfeLote`, `listarProdutosAutoos` functions exist in `db.ts`
  - [ ] TypeScript compiles without errors

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Frontend types compile without errors
    Tool: Bash (npm run build)
    Steps:
      1. Run: npm run build
    Expected Result: Build succeeds with no TypeScript errors
    Failure Indicators: Type errors about missing fields, removed types
    Evidence: .sisyphus/evidence/task-14-frontend-types.txt
  ```

  **Commit**: YES
  - Message: `feat(frontend): create types and API wrapper for PRD v2 schema`
  - Files: `src/types/index.ts`, `src/lib/db.ts`

- [x] 14. Create NovoBoleto page (NF-e upload + manual)

  **What to do**:
  - Create `src/pages/NovoBoleto.tsx` per PRD section 9.3:
    - Two cards: "Importar NF-e" and "Entrada Manual"
    - Drag & drop area for XML files
    - Manual form with all fields from PRD
  - Create `src/components/nfe/NFeUploader.tsx` — drag & drop component
  - Create `src/components/nfe/NFeListaImportacao.tsx` — batch import table
  - Create `src/components/boleto/BoletoRevisaoForm.tsx` — review screen per PRD 6.2
  - Create `src/hooks/useBoletos.ts` — add `useNFeImport` hook
  - Create sidebar in `src/components/Layout.tsx` with "Novo Boleto" menu item (no "Fila")
  - Create routes in `src/App.tsx`:
    - `/novo-boleto` route
    - No `/fila` route

  **Must NOT do**:
  - Do NOT display fila de faturamento from estoque

  **Recommended Agent Profile**:
  - **Category**: `visual-engineering`
  - **Skills**: [`playwright`]

  **Parallelization**:
  - **Can Run In Parallel**: NO — depends on Tasks 10, 13
  - **Parallel Group**: Wave 3
  - **Blocks**: Task 16
  - **Blocked By**: Tasks 10, 13

  **References**:
  **API/Type References**:
  - `PRD_AutoBO_v2.md` section 9.3-9.5 — UI wireframes
  - `PRD_AutoBO_v2.md` section 6.2 — Review screen specification

  **Acceptance Criteria**:
  - [ ] `/novo-boleto` page renders with two cards (NF-e and Manual)
  - [ ] NFeUploader drag & drop works
  - [ ] BoletoRevisaoForm displays all fields from PRD 6.2
  - [ ] No reference to "fila" or "movimentacoes_estoque" in frontend
  - [ ] Sidebar has "Novo Boleto" item (no "Fila")

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Novo Boleto page renders
    Tool: Playwright
    Steps:
      1. Navigate to /novo-boleto
      2. Verify two cards visible: "Importar NF-e" and "Entrada Manual"
      3. Verify drag & drop area visible
    Expected Result: Page renders without errors
    Failure Indicators: Blank page, console errors, missing components
    Evidence: .sisyphus/evidence/task-15-novo-boleto-page.png

  Scenario: Manual boleto form works
    Tool: Playwright
    Steps:
      1. Click "Entrada Manual"
      2. Fill CPF/CNPJ field
      3. Verify pagador auto-fill or "Novo pagador" badge appears
    Expected Result: Form displays correctly, validation works
    Failure Indicators: Form not rendering, validation not triggering
    Evidence: .sisyphus/evidence/task-15-manual-form.png
  ```

  **Commit**: YES
  - Message: `feat(frontend): create NovoBoleto page with NF-e upload and manual entry per PRD v2`
  - Files: `src/pages/NovoBoleto.tsx`, `src/components/nfe/NFeUploader.tsx`, `src/components/nfe/NFeListaImportacao.tsx`, `src/components/boleto/BoletoRevisaoForm.tsx`, `src/App.tsx`, `src/components/Layout.tsx`, `src/hooks/useBoletos.ts`

- [ ] 15. E2E test: manual boleto flow

  **What to do**:
  - Start app from clean database state
  - Test complete manual boleto creation flow:
    1. Open app, go to "Novo Boleto"
    2. Click "Entrada Manual"
    3. Fill in pagador data (CPF, nome, endereço)
    4. Fill in boleto data (valor, vencimento, tipo cobrança)
    5. Fill in items (descrição, quantidade, valor unitário)
    6. Click "Gerar Boleto"
    7. Verify boleto created in database with status RASCUNHO
    8. Verify pagador created in database
    9. Verify items created in database
  - Also test with existing pagador (by documento)
  - Test cancelar_boleto
  - Test listar_boletos with origem field

  **Must NOT do**:
  - Do NOT test with Sicredi API (credentials not available yet)
  - Do NOT test with `movimentacoes_estoque` data

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: [`playwright`]

  **Parallelization**:
  - **Can Run In Parallel**: NO — depends on all previous tasks
  - **Parallel Group**: Wave 4
  - **Blocks**: Final verification
  - **Blocked By**: Tasks 10, 11

  **References**:
  **API/Type References**:
  - `PRD_AutoBO_v2.md` section 2 (Fluxo B — Manual)

  **Acceptance Criteria**:
  - [ ] Manual boleto created successfully from end to end
  - [ ] Pagador auto-created by documento
  - [ ] Items stored correctly in `autobo_itens_boleto`
  - [ ] Boleto status is RASCUNHO (Sicredi not configured)
  - [ ] `origem` field is 'MANUAL'
  - [ ] No reference to `movimentacoes_estoque` in any query

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: Manual boleto creation end-to-end
    Tool: Bash (psql + cargo run + Playwright)
    Steps:
      1. Start app with clean database
      2. Navigate to /novo-boleto
      3. Click "Entrada Manual"
      4. Fill: documento = '12345678000190', nome = 'Empresa Teste LTDA', valor = 1500.00, vencimento = future date
      5. Add item: descricao='Serviço de consultoria', quantidade=1, valor_unitario=1500.00
      6. Click "Gerar Boleto"
      7. Verify success message
      8. Query: SELECT * FROM autobo_boletos WHERE origem = 'MANUAL'
      9. Query: SELECT * FROM autobo_pagadores WHERE documento = '12345678000190'
      10. Query: SELECT * FROM autobo_itens_boleto WHERE descricao = 'Serviço de consultoria'
    Expected Result: Boleto, pagador, and item created correctly. No errors.
    Failure Indicators: SQL error, missing data, wrong origem, panic
    Evidence: .sisyphus/evidence/task-17-manual-boleto-e2e.txt
  ```

  **Commit**: YES
  - Message: `test(e2e): manual boleto creation end-to-end`

- [ ] 16. E2E test: NF-e import flow

  **What to do**:
  - Create a sample NF-e XML file (version 4.0) with test data
  - Test NF-e import flow:
    1. Open app, go to "Novo Boleto"
    2. Click "Importar NF-e"
    3. Upload sample XML
    4. Verify parsed data displayed correctly
    5. Click "Gerar Boleto"
    6. Verify NF-e record in `autobo_nfes`
    7. Verify boleto with `origem = 'NFE'`
    8. Verify pagador auto-created from NF-e data
  - Test duplicate NF-e rejection (same chave_acesso)
  - Test invalid XML error handling
  - Also test with NF-e 3.1 sample XML

  **Must NOT do**:
  - Do NOT test with real NF-e data
  - Do NOT connect to SEFAZ

  **Recommended Agent Profile**:
  - **Category**: `deep`
  - **Skills**: [`playwright`]

  **Parallelization**:
  - **Can Run In Parallel**: YES (with Task 15)
  - **Parallel Group**: Wave 4
  - **Blocks**: Final verification
  - **Blocked By**: Tasks 10, 14

  **References**:
  **API/Type References**:
  - `PRD_AutoBO_v2.md` section 2 (Fluxo A — NF-e XML)
  - `PRD_AutoBO_v2.md` section 6.1 — Parser validation rules

  **Acceptance Criteria**:
  - [ ] Valid NF-e 4.0 XML parses correctly
  - [ ] Valid NF-e 3.1 XML parses correctly
  - [ ] Duplicate chave_acesso rejected with descriptive error
  - [ ] Invalid XML rejected with descriptive error
  - [ ] Boleto created with `origem = 'NFE'`
  - [ ] `autobo_nfes` record created with status 'IMPORTADA'

  **QA Scenarios (MANDATORY)**:

  ```
  Scenario: NF-e XML import and boleto generation
    Tool: Bash (cargo test + psql)
    Steps:
      1. Create sample NF-e 4.0 XML test file
      2. Call importar_nfe with base64-encoded XML
      3. Verify parsed data: numero_nf, chave_acesso, valor_total, destinatario
      4. Create boleto from parsed data
      5. Query: SELECT * FROM autobo_nfes WHERE chave_acesso = '{test_chave}'
      6. Query: SELECT * FROM autobo_boletos WHERE origem = 'NFE'
    Expected Result: NF-e parsed, boleto created with origem='NFE', nfe_id linked
    Failure Indicators: Parse error, missing fields, wrong origem
    Evidence: .sisyphus/evidence/task-18-nfe-import-e2e.txt

  Scenario: Duplicate NF-e rejection
    Tool: Bash (cargo test)
    Steps:
      1. Import same NF-e XML twice
    Expected Result: Second import returns error about duplicate chave_acesso
    Failure Indicators: Duplicate record created, no error
    Evidence: .sisyphus/evidence/task-18-duplicate-nfe.txt
  ```

  **Commit**: YES
  - Message: `test(e2e): NF-e import and duplicate validation`

---

## Final Verification Wave (MANDATORY — after ALL implementation tasks)

> 4 review agents run in PARALLEL. ALL must APPROVE.

- [x] F1. **Plan Compliance Audit** — `oracle`
  Read the plan end-to-end. For each "Must Have": verify implementation exists. For each "Must NOT Have": search codebase for forbidden patterns. Check evidence files. Compare deliverables against plan.
  Output: `Must Have [N/N] | Must NOT Have [N/N] | Tasks [N/N] | VERDICT: APPROVE/REJECT`

- [x] F2. **Code Quality Review** — `unspecified-high`
  Run `cargo clippy` + `cargo test`. Review all changed files for: `as any`/`@ts-ignore`, empty catches, `unwrap()` in production code (only OK in tests), `todo!()`/`unimplemented!()`, unused imports, AI slop patterns.
  Output: `Clippy [PASS/FAIL] | Tests [N pass/N fail] | Files [N clean/N issues] | VERDICT`

- [x] F3. **Real Manual QA** — `unspecified-high`
  Start app from clean state. Execute EVERY QA scenario from every task. Test cross-task integration. Save evidence to `.sisyphus/evidence/final-qa/`.
  Output: `Scenarios [N/N pass] | Integration [N/N] | Edge Cases [N tested] | VERDICT`

- [x] F4. **Scope Fidelity Check** — `deep`
  For each task: read "What to do", read actual diff. Verify 1:1 — everything in spec was built, nothing beyond spec was built. Check "Must NOT do" compliance. Detect cross-task contamination.
  Output: `Tasks [N/N compliant] | Contamination [CLEAN/N issues] | Verdict`

---

## Commit Strategy

- **Wave 0**: `feat(init): scaffold AutoBO Tauri project per PRD v2`
- **Wave 1**: `feat(migrations): create V200-V205 migrations per PRD v2` + `feat(crypto): create crypto module`
- **Wave 2**: Individual feature commits per module
- **Wave 3**: `feat(boleto): create gerar_boleto for NF-e and manual flows` + `feat(frontend): create pages for new flow`
- **Wave 4**: `test(e2e): manual boleto + NF-e import flows`
- Each task: pre-commit `cargo test`

---

## Success Criteria

### Verification Commands
```bash
cargo build                                          # Expected: SUCCESS
cargo test                                           # Expected: ALL PASS
psql $DB_URL -c "\dt autobo_*"                      # Expected: 6 tables (pagadores, nfes, boletos, itens_boleto, comunicacoes, configuracoes, migrations)
psql $DB_URL -c "SELECT * FROM autobo_configuracoes" # Expected: 16 seed rows
grep -r "movimentacao_id" src-tauri/src/             # Expected: 0 results
grep -r "cliente_id" src-tauri/src/commands/types.rs  # Expected: 0 results (in pagador context)
grep -r "movimento_ids" src-tauri/src/               # Expected: 0 results
```

### Final Checklist
- [ ] All "Must Have" present
- [ ] All "Must NOT Have" absent
- [ ] All tests pass