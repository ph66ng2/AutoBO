# Evidência — AP-ID-001 (identidade compartilhada)

Espelho `BO-AP-ID-EXT-001`. Nenhum domínio de identidade é implementado neste repositório. A fonte autoritativa permanece no AutoPlatform.

## Entrega confirmada

| Campo | Valor |
| --- | --- |
| Ticket | `AP-ID-001` (`merged` no workflow do AutoPlatform) |
| Pull request | https://github.com/ph66ng2/AutoPlatform/pull/5 |
| Merge | `6a650677a4635b3d001f438fb9de13050b2c8781` em 2026-09-23 |
| CI em `main` | push `35813436228` verde após o merge |
| Confirmação humana | 2026-09-24, seguir o AutoBO a partir desta evidência |

Arquivos revisados no `origin/main` do AutoPlatform: `migrations/0002_identity.sql`, `crates/identity/src/{lib,model,jwt,directory}.rs`, `crates/identity/tests/tenant_isolation.rs`.

## Critérios de aceite

### `company_id` é autoritativo

A sessão não sai de `user_metadata`, `app_metadata` nem de `company_id` no JWT. O token autenticado só entrega `sub` (`account_id`). A company e o produto vêm da membership ativa no diretório, consultada com o `company_id` pedido pelo cliente.

Prova: `crates/identity/src/jwt.rs` (`Claims` só tem `sub` e `role`) e `resolve` em `crates/identity/src/lib.rs`. O teste `arbitrary_company_id_is_denied_like_any_other_miss` recusa company cruzada e company inexistente com o mesmo `AccessError::Denied`, sem revelar existência.

### ADMIN do AutoOS não concede FISCAL no AutoBO

Papel não atravessa produto. `Role::Fiscal` só cabe em `Product::AutoBo`. `Permission::Fiscal` só é concedida à role `fiscal` do AutoBO. ADMIN AutoOS e ADMIN AutoBO operam a empresa; nenhum dos dois recebe FISCAL por ser admin.

Prova: `Role::fits` / `Role::grants` em `crates/identity/src/model.rs` e o teste `autoos_admin_does_not_receive_autobo_fiscal`. A migration recusa `product = 'autoos' AND role = 'fiscal'`. O teste `rejects_autoos_fiscal_and_service_role_token` cobre essa inserção e recusa JWT `service_role`.

### Tenant A não acessa B

RLS com `FORCE` em `companies`, `memberships`, `audit_events` e `session_touches`. Policies têm `USING` e `WITH CHECK` ancorados em `identity.current_company_id()`. Membership suspensa ou revogada perde leitura e escrita.

Prova: `rls_blocks_cross_tenant_reads_and_writes` em `crates/identity/tests/tenant_isolation.rs` (job de Postgres do CI do AutoPlatform). Escrita cruzada e mudança de `company_id` retornam `42501`. Auditoria de suspensão/revogação não vaza para o outro tenant.

## Fora deste espelho

Não há migration de identidade no AutoBO, token versionado, nem login desktop. Isso é `BO-AUTH-001`, depois deste merge.
