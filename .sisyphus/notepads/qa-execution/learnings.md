# QA Learnings

## Test suite is comprehensive
- 27 unit tests covering crypto (5 tests), nfe_parser (7 tests), cpf (7 tests), cnpj (8 tests)
- All edge cases well covered (empty inputs, wrong formats, format variations, invalid characters)
- Tests run fast (<1s) and are isolated from database

## Build is clean
- Only warning is `sqlx-postgres v0.7.4` future incompatibility (not critical, upgrade to sqlx 0.8+ eventually)

## Frontend type-checking passes
- TypeScript strict mode is enforced and passes cleanly across all components, hooks, and pages

## Forbidden pattern check - false positive
- `cliente_id` appears in pagador.rs only in a doc comment stating it's NOT used: `//! Uses documento as the unique business key (NO cliente_id).`
- This is a documentation artifact, not an actual usage of `cliente_id` as a field.
