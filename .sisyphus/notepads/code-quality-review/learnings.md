# Code Quality Review — Learnings

## Patterns & Conventions Observed
- Clean module structure: `commands/`, `services/`, `validators/`, `jobs/`, plus `db.rs`
- Tauri 2.x command registration uses `generate_handler![]` macro
- Error handling uses `Result<T, String>` pattern with `.map_err()` for DB errors
- Test modules live inside the same source files via `#[cfg(test)] mod tests {}`
- Frontend: TypeScript strict, no `as any` or `@ts-ignore` found
- Crypto service uses AES-256-GCM for encrypting sensitive config values
- Tracing (not println!) used for production logging

## Module Organization
- `lib.rs` → orchestrates modules and defines `run()` entry point
- `main.rs` → minimal, just calls `autobo_lib::run()`
- `commands/mod.rs` → re-exports all command handlers + types
- `services/mod.rs` → references planned but unimplemented services (sicredi, email, whatsapp)
- `validators/mod.rs` → CPF/CNPJ validation
- `jobs/mod.rs` → scheduler module
