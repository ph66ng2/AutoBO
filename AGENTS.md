# AutoBO

Fonte de verdade dos tickets: `.workflow/workflow.json`.

Branch principal: `main`, rastreando `origin/main`. Cada ticket sai dessa branch, em worktree e branch próprias (`ticket/<id>`). O pull request aponta para `main`. Uma pessoa faz o merge. Não há merge, release nem alteração de produção automáticos.

## Fora do índice

Não versionar `.env`, `src-tauri/.env`, `node_modules`, `dist`, `src-tauri/target`, `tmp` nem segredos. `src-tauri/.env.example` só contém placeholders.

## Checks mínimos

Frontend, na raiz:

```bash
npm run build
npm test
```

Rust, na raiz:

```bash
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

Banco: as migrations ficam em `src-tauri/migrations/` e o runner manual usa a tabela `autobo_migrations` (`src-tauri/src/db.rs`). O `cargo test` padrão não conecta no Postgres. No CI, o job `migrations` sobe um Postgres descartável e roda só `apply_migrations_to_disposable_database`. O teste ignorado `apply_migrations_to_configured_database` continua manual e exige o banco da oficina:

```bash
cargo test --manifest-path src-tauri/Cargo.toml -- --ignored apply_migrations_to_configured_database
```

## Planejador

```bash
.workflow/scripts/waves.sh plan .workflow/workflow.json
```

Um ticket só entra em onda quando todos os bloqueadores têm onda ou estão `merged`. Ticket com status `blocked` fica sem onda, e quem depende dele também.
