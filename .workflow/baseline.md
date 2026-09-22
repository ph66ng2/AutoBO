# Baseline BO-GOV-001

Registrado em 2026-09-21, no commit `58c25d80de6ce426b9c9aa4dfa0b13b8197c76a2` (`origin/main`).

| Check | Resultado |
| --- | --- |
| `npm test` | 3 arquivos, 9 testes passaram |
| `npm run build` | `tsc` e Vite concluíram |
| `cargo check --manifest-path src-tauri/Cargo.toml` | concluiu |
| `cargo test --manifest-path src-tauri/Cargo.toml` | 66 passaram, 1 ignorado, 0 falharam |
| Varredura de segredos no índice | `.env` e `src-tauri/.env` ignorados; nenhum arquivo de chave privada versionado; `src-tauri/.env.example` só tem placeholders |

O teste ignorado é `apply_migrations_to_configured_database`. Ele conecta no `DATABASE_URL` e não faz parte deste baseline.

`cargo` avisou que `sqlx-postgres` v0.7.4 terá código rejeitado por uma versão futura do Rust. O check e os testes mesmo assim passaram.
