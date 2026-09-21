
## Learnings from configuracoes.rs implementation

### Module structure
- `commands/mod.rs` uses `pub mod configuracoes;` + `pub use configuracoes::{...}` to re-export commands
- `lib.rs` registers commands via fully qualified path: `commands::configuracoes::get_config`
- The fully qualified path is necessary because `#[tauri::command]` generates helper macros in the defining module (`commands::configuracoes`), not in the re-exporting module

### Encryption pattern
- `crypto::encrypt()` and `crypto::decrypt()` from `crate::services::crypto` handle AES-256-GCM
- `get_config`: try decrypt, fall back to raw value (legacy plaintext compatibility)
- `set_config`: encrypt before storing if key is in `SENSITIVE_KEYS`
- `SENSITIVE_KEYS`: `smtp.senha`, `whatsapp.token`, `sicredi.api_secret`, `sicredi.client_secret`

### SQL pattern
- UPSERT via `INSERT ... ON CONFLICT (chave) DO UPDATE SET valor = $2, atualizado_em = NOW()`
- `sqlx::query_as::<_, ConfigRow>` for typed row mapping
- `ConfigRow` derives `FromRow` for sqlx compatibility

### Concurrency awareness
- Multiple agents may modify `mod.rs` concurrently; re-read file before each edit
- Use unique anchor strings in `edit()` to avoid matching wrong sections

## Task: Boleto Types + Commands (boleto.rs)

### Key Decisions
- **`BoletoInput` / `ItemInput`**: No `movimento_ids` or `movimentacao_id` fields per spec. Clean separation from estoque/OS domain.
- **`ItemBoletoRow.unidade`**: `String` (not `Option<String>`) — the DB column has `NOT NULL DEFAULT 'UN'`, so reads always return a value.
- **`cancelar_boleto`**: Implemented as real UPDATE (not stub) since the MUST DO spec provided complete SQL. Sets status to 'CANCELADO'.
- **`buscar_boleto`**: Returns `serde_json::Value` with both `boleto` and `itens` keys. NFe join deferred to Task 10.

### Tauri 2.x Patterns
- `generate_handler!` needs the EXACT module path where `#[tauri::command]` is defined (e.g., `commands::boleto::listar_boletos`). `pub use` re-exports work for public API but NOT for `generate_handler!` macro resolution.
- Module declaration order in `mod.rs` doesn't matter for compilation, but matters for readability.

### sqlx 0.7 Notes
- `DECIMAL` columns in PostgreSQL decode to Rust `f64` without needing `rust_decimal` or `bigdecimal` features.
- The `chrono` feature enables `NaiveDate`/`NaiveDateTime` decoding from DATE/TIMESTAMP columns.
- `FromRow` derive works with `SELECT *` — column names map to snake_case struct fields.

### Parallel Task Conflicts
- Multiple tasks modifying `mod.rs` and `lib.rs` simultaneously caused overwrites. Be prepared to redo edits.
- The `validators.rs` stub was needed because `lib.rs` declared `pub mod validators;` before the file existed.

## Task: Dashboard + Jobs (dashboard.rs, scheduler.rs)

### Dashboard Commands
- `metricas_dashboard` and `top_devedores` moved from inline stubs in `commands/mod.rs` to `commands/dashboard.rs`
- `DashboardMetricas` struct uses `Option<f64>` for `valor_a_receber`/`valor_recebido` (COALESCE in SQL ensures non-null, but FromRow needs Option for aggregate expressions)
- `TopDevedor` joins `autobo_boletos` with `autobo_pagadores`
- SQL uses `COUNT(*) FILTER (WHERE ...)` for conditional aggregation
- `generate_handler!` must use the defining module path (`commands::dashboard::metricas_dashboard`), not the re-export path

### Scheduled Jobs
- 4 cron jobs per PRD v2 section 11:
  1. `job_atualizar_status` — hourly, checks REGISTRADO boletos via Sicredi API (TODO stub)
  2. `job_lembrete_vencimento` — daily 08:00, D-1 payment reminders
  3. `job_aviso_vencido` — daily 09:00, D+1 overdue notices
  4. `job_marcar_vencidos` — daily 00:05, marks expired REGISTRADO → VENCIDO
- `tokio-cron-scheduler` 0.13: `JobScheduler::add()` returns `Future` — must `.await` before `.unwrap()`
- Scheduler initiated inside `rt.block_on` so `tokio::spawn` has a runtime handle
- `iniciar_scheduler` spawns a background task; the main thread doesn't block
