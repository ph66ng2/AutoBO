//! Database initialization and manual migration runner.
//!
//! Uses `autobo_migrations` table (NOT `_sqlx_migrations`) to avoid
//! collision with AutoOS when sharing the same PostgreSQL database.

use sqlx::postgres::{PgPool, PgPoolOptions};
use std::env;
use std::fs;
use std::path::PathBuf;
use tracing::{info, warn};

/// Shared application state managed by Tauri.
pub struct AppState {
    pub db: PgPool,
}

/// Initialize the database connection pool and run pending migrations.
pub async fn init_db() -> Result<PgPool, Box<dyn std::error::Error>> {
    // Try .env file first, then environment variable
    dotenv();

    let database_url = normalize_database_url(
        env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env or environment"),
    );

    info!("Connecting to database...");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    info!("Database connection established. Running migrations...");

    run_migrations_manual(&pool).await?;

    let integracao = detectar_integracao_autoos(&pool).await;
    if integracao.produtos || integracao.clientes || integracao.equipamentos {
        info!(
            "AutoOS integration detected — clientes={} equipamentos={} produtos={}",
            integracao.clientes, integracao.equipamentos, integracao.produtos
        );
    } else {
        info!("Standalone mode — AutoOS tables not found");
    }

    Ok(pool)
}

/// Attempt to load DATABASE_URL from a .env file located next to the executable
/// or in the current working directory (development fallback).
fn dotenv() {
    let env_path = find_env_file();
    if let Some(path) = env_path {
        if let Ok(contents) = fs::read_to_string(&path) {
            for line in contents.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((key, value)) = line.split_once('=') {
                    let key = key.trim();
                    let value = value.trim();
                    if env::var(key).is_err() {
                        env::set_var(key, value);
                    }
                }
            }
        }
    }
}

/// Resolve the .env file path relative to the executable, falling back to CWD.
fn find_env_file() -> Option<PathBuf> {
    // Try next to executable first
    if let Ok(exe) = env::current_exe() {
        if let Some(dir) = exe.parent() {
            let path = dir.join(".env");
            if path.exists() {
                return Some(path);
            }
        }
    }
    if let Ok(manifest) = env::var("CARGO_MANIFEST_DIR") {
        let path = PathBuf::from(manifest).join(".env");
        if path.exists() {
            return Some(path);
        }
    }
    // Fallback: current working directory (development)
    let cwd_path = PathBuf::from(".env");
    if cwd_path.exists() {
        return Some(cwd_path);
    }
    let nested = PathBuf::from("src-tauri/.env");
    if nested.exists() {
        return Some(nested);
    }
    None
}

fn normalize_database_url(url: String) -> String {
    if url.contains("supabase.") && !url.contains("sslmode=") {
        if url.contains('?') {
            format!("{url}&sslmode=require")
        } else {
            format!("{url}?sslmode=require")
        }
    } else {
        url
    }
}

/// Run migrations manually using `autobo_migrations` tracking table.
/// Migration files live in `src-tauri/migrations/` relative to the Cargo manifest
/// directory. At runtime, we try the executable-relative path first, then CWD.
async fn run_migrations_manual(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    // Create tracking table if not exists
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS autobo_migrations (
            version     BIGINT PRIMARY KEY,
            description VARCHAR(255) NOT NULL,
            applied_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Old AutoBO V1xx tables share this database. CREATE TABLE IF NOT EXISTS
    // does not add missing columns, so reconcile before applying V2xx files.
    reconciliar_schema_legado(pool).await?;

    // Find the migrations directory
    let migrations_dir = find_migrations_dir();
    if !migrations_dir.exists() {
        warn!(
            "Migrations directory not found at {:?}. Skipping migrations.",
            migrations_dir
        );
        return Ok(());
    }

    // Read all .sql files sorted by name
    let mut entries: Vec<_> = fs::read_dir(&migrations_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "sql")
                .unwrap_or(false)
        })
        .collect();

    entries.sort_by_key(|e| e.file_name());

    // Note: Old AutoBO entries (versions 100-105) in autobo_migrations are
    // safely ignored since their corresponding .sql files no longer exist.
    // Zero-padded prefixes (e.g. 0200) are parsed correctly by
    // parse_version_from_filename and ensure alphabetical sort order.
    for entry in entries {
        let path = entry.path();
        let filename = entry.file_name().to_string_lossy().to_string();

        // Parse version from filename: "0200__autobo_pagadores.sql" → version 200
        let version = match parse_version_from_filename(&filename) {
            Some(v) => v,
            None => {
                warn!("Could not parse version from filename: {}. Skipping.", filename);
                continue;
            }
        };

        // Check if already applied
        let already_applied: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM autobo_migrations WHERE version = $1)",
        )
        .bind(version)
        .fetch_one(pool)
        .await?;

        if already_applied {
            info!("Migration V{} ({}) already applied. Skipping.", version, filename);
            continue;
        }

        info!("Applying migration V{}: {}", version, filename);

        let sql = fs::read_to_string(&path)?;

        // Split by `;`. Line comments can contain semicolons, so skip
        // fragments that have no SQL after stripping `--` comments.
        for statement in sql.split(';') {
            let statement = statement.trim();
            if !tem_sql(statement) {
                continue;
            }
            sqlx::query(statement).execute(pool).await?;
        }

        // Record migration as applied
        sqlx::query(
            "INSERT INTO autobo_migrations (version, description) VALUES ($1, $2)",
        )
        .bind(version)
        .bind(&filename)
        .execute(pool)
        .await?;

        info!("Migration V{} applied successfully.", version);
    }

    proteger_tabelas_autobo(pool).await?;

    Ok(())
}

/// Resolve the migrations directory.
/// Tries executable-relative path first, then Cargo manifest dir, then CWD.
fn find_migrations_dir() -> PathBuf {
    // Try next to executable
    if let Ok(exe) = env::current_exe() {
        if let Some(dir) = exe.parent() {
            let path = dir.join("migrations");
            if path.exists() {
                return path;
            }
        }
    }
    // Try relative to Cargo manifest dir (development)
    if let Ok(manifest) = env::var("CARGO_MANIFEST_DIR") {
        let path = PathBuf::from(&manifest).join("migrations");
        if path.exists() {
            return path;
        }
    }
    // Fallback: current working directory
    PathBuf::from("src-tauri/migrations")
}

fn tem_sql(statement: &str) -> bool {
    statement.lines().any(|line| {
        let line = line.trim();
        !line.is_empty() && !line.starts_with("--")
    })
}

/// Parse version from migration filename.
/// Expects format like "0200__autobo_pagadores.sql" → 200
fn parse_version_from_filename(filename: &str) -> Option<i64> {
    // Strip .sql extension
    let stem = filename.strip_suffix(".sql")?;
    // The version is the first part before any non-digit character
    let version_str = stem.chars().take_while(|c| c.is_ascii_digit()).collect::<String>();
    version_str.parse::<i64>().ok()
}

#[derive(Debug, Clone, Copy, Default)]
pub struct IntegracaoAutoOS {
    pub clientes: bool,
    pub equipamentos: bool,
    pub produtos: bool,
}

pub async fn tabela_existe(pool: &PgPool, nome: &str) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(
            SELECT 1 FROM information_schema.tables
            WHERE table_schema = 'public' AND table_name = $1
        )",
    )
    .bind(nome)
    .fetch_one(pool)
    .await
    .unwrap_or(false)
}

pub async fn detectar_integracao_autoos(pool: &PgPool) -> IntegracaoAutoOS {
    IntegracaoAutoOS {
        clientes: tabela_existe(pool, "clientes").await,
        equipamentos: tabela_existe(pool, "equipamentos").await,
        produtos: tabela_existe(pool, "produtos").await,
    }
}

/// Public API: check if AutoOS `produtos` table is available.
pub async fn produtos_disponivel(pool: &PgPool) -> bool {
    tabela_existe(pool, "produtos").await
}

pub async fn clientes_disponivel(pool: &PgPool) -> bool {
    tabela_existe(pool, "clientes").await
}

pub async fn equipamentos_disponivel(pool: &PgPool) -> bool {
    tabela_existe(pool, "equipamentos").await
}

pub async fn coluna_existe(pool: &PgPool, tabela: &str, coluna: &str) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(
            SELECT 1 FROM information_schema.columns
            WHERE table_schema = 'public' AND table_name = $1 AND column_name = $2
        )",
    )
    .bind(tabela)
    .bind(coluna)
    .fetch_one(pool)
    .await
    .unwrap_or(false)
}

fn ident_seguro(nome: &str) -> Result<(), Box<dyn std::error::Error>> {
    if nome.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        && nome.starts_with(|c: char| c.is_ascii_lowercase() || c == '_')
    {
        Ok(())
    } else {
        Err(format!("identificador inválido: {nome}").into())
    }
}

async fn add_column(
    pool: &PgPool,
    tabela: &str,
    coluna: &str,
    tipo_sql: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    ident_seguro(tabela)?;
    ident_seguro(coluna)?;
    if !tabela_existe(pool, tabela).await || coluna_existe(pool, tabela, coluna).await {
        return Ok(());
    }
    let sql = format!("ALTER TABLE {tabela} ADD COLUMN {coluna} {tipo_sql}");
    sqlx::query(&sql).execute(pool).await?;
    Ok(())
}

async fn drop_not_null(
    pool: &PgPool,
    tabela: &str,
    coluna: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    ident_seguro(tabela)?;
    ident_seguro(coluna)?;
    if !coluna_existe(pool, tabela, coluna).await {
        return Ok(());
    }
    let sql = format!("ALTER TABLE {tabela} ALTER COLUMN {coluna} DROP NOT NULL");
    sqlx::query(&sql).execute(pool).await?;
    Ok(())
}

/// Bring V1xx AutoBO tables up to the V2 columns the app expects.
/// No-op when tables are missing (fresh Supabase) or already match.
async fn reconciliar_schema_legado(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    if tabela_existe(pool, "autobo_pagadores").await {
        add_column(pool, "autobo_pagadores", "nome", "VARCHAR(200)").await?;
        add_column(pool, "autobo_pagadores", "razao_social", "VARCHAR(200)").await?;
        add_column(pool, "autobo_pagadores", "nome_fantasia", "VARCHAR(200)").await?;
        add_column(pool, "autobo_pagadores", "telefone", "VARCHAR(15)").await?;
        add_column(pool, "autobo_pagadores", "email", "VARCHAR(150)").await?;
        drop_not_null(pool, "autobo_pagadores", "cliente_id").await?;
        sqlx::query(
            "CREATE UNIQUE INDEX IF NOT EXISTS autobo_pagadores_documento_uidx ON autobo_pagadores (documento)",
        )
        .execute(pool)
        .await?;
        if coluna_existe(pool, "autobo_pagadores", "email_cobranca").await {
            sqlx::query(
                "UPDATE autobo_pagadores SET email = COALESCE(email, email_cobranca) WHERE email IS NULL",
            )
            .execute(pool)
            .await?;
        }
        if coluna_existe(pool, "autobo_pagadores", "telefone_cobranca").await {
            sqlx::query(
                "UPDATE autobo_pagadores SET telefone = COALESCE(telefone, telefone_cobranca) WHERE telefone IS NULL",
            )
            .execute(pool)
            .await?;
        }
    }

    if tabela_existe(pool, "autobo_boletos").await {
        let nfe_ref = if tabela_existe(pool, "autobo_nfes").await {
            "BIGINT REFERENCES autobo_nfes(id)"
        } else {
            "BIGINT"
        };
        add_column(pool, "autobo_boletos", "nfe_id", nfe_ref).await?;
        add_column(
            pool,
            "autobo_boletos",
            "origem",
            "VARCHAR(20) NOT NULL DEFAULT 'NFE'",
        )
        .await?;
        if coluna_existe(pool, "autobo_boletos", "gerado_em").await
            && !coluna_existe(pool, "autobo_boletos", "criado_em").await
        {
            sqlx::query("ALTER TABLE autobo_boletos RENAME COLUMN gerado_em TO criado_em")
                .execute(pool)
                .await?;
        }
        add_column(
            pool,
            "autobo_boletos",
            "criado_em",
            "TIMESTAMP NOT NULL DEFAULT NOW()",
        )
        .await?;
    }

    if tabela_existe(pool, "autobo_itens_boleto").await {
        add_column(pool, "autobo_itens_boleto", "descricao", "VARCHAR(500)").await?;
        add_column(
            pool,
            "autobo_itens_boleto",
            "valor_unitario",
            "DECIMAL(12,2)",
        )
        .await?;
        add_column(pool, "autobo_itens_boleto", "produto_autoos_id", "BIGINT").await?;
        add_column(pool, "autobo_itens_boleto", "subtotal", "DECIMAL(12,2)").await?;
        drop_not_null(pool, "autobo_itens_boleto", "movimentacao_id").await?;
        drop_not_null(pool, "autobo_itens_boleto", "produto_nome").await?;
        if coluna_existe(pool, "autobo_itens_boleto", "produto_nome").await {
            sqlx::query(
                "UPDATE autobo_itens_boleto SET descricao = COALESCE(NULLIF(descricao, ''), produto_nome) WHERE descricao IS NULL OR descricao = ''",
            )
            .execute(pool)
            .await?;
        }
        if coluna_existe(pool, "autobo_itens_boleto", "preco_unitario").await {
            sqlx::query(
                "UPDATE autobo_itens_boleto SET valor_unitario = COALESCE(valor_unitario, preco_unitario)",
            )
            .execute(pool)
            .await?;
        }
        sqlx::query("UPDATE autobo_itens_boleto SET unidade = 'UN' WHERE unidade IS NULL")
            .execute(pool)
            .await?;
    }

    if tabela_existe(pool, "autobo_comunicacoes").await {
        add_column(
            pool,
            "autobo_comunicacoes",
            "destinatario",
            "VARCHAR(150) NOT NULL DEFAULT ''",
        )
        .await?;
        add_column(pool, "autobo_comunicacoes", "assunto", "VARCHAR(200)").await?;
        add_column(pool, "autobo_comunicacoes", "conteudo", "TEXT").await?;
        add_column(
            pool,
            "autobo_comunicacoes",
            "status_envio",
            "VARCHAR(20) NOT NULL DEFAULT 'PENDENTE'",
        )
        .await?;
        add_column(
            pool,
            "autobo_comunicacoes",
            "tentativas",
            "INTEGER NOT NULL DEFAULT 0",
        )
        .await?;
        add_column(pool, "autobo_comunicacoes", "enviado_em", "TIMESTAMP").await?;
        if coluna_existe(pool, "autobo_comunicacoes", "contato").await {
            sqlx::query(
                "UPDATE autobo_comunicacoes SET destinatario = COALESCE(NULLIF(destinatario, ''), contato)",
            )
            .execute(pool)
            .await?;
        }
    }

    if tabela_existe(pool, "autobo_configuracoes").await {
        add_column(pool, "autobo_configuracoes", "id", "BIGSERIAL").await?;
        add_column(pool, "autobo_configuracoes", "descricao", "VARCHAR(300)").await?;
        drop_not_null(pool, "autobo_configuracoes", "tipo").await?;
    }

    Ok(())
}

const TABELAS_AUTOBO: &[&str] = &[
    "autobo_pagadores",
    "autobo_nfes",
    "autobo_boletos",
    "autobo_itens_boleto",
    "autobo_comunicacoes",
    "autobo_configuracoes",
    "autobo_migrations",
];

async fn proteger_tabelas_autobo(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let roles: Vec<String> = sqlx::query_scalar(
        "SELECT rolname FROM pg_roles WHERE rolname IN ('anon', 'authenticated', 'service_role')",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    for tabela in TABELAS_AUTOBO {
        if !tabela_existe(pool, tabela).await {
            continue;
        }
        ident_seguro(tabela)?;
        for role in &roles {
            ident_seguro(role)?;
            let sql = format!("REVOKE ALL ON TABLE {tabela} FROM {role}");
            sqlx::query(&sql).execute(pool).await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignora_ponto_e_virgula_dentro_de_comentario() {
        let sql = "-- keep posto; document that api_key lives in keyring\nUPDATE t SET x = 1";
        let stmts: Vec<_> = sql.split(';').filter(|s| tem_sql(s)).collect();
        assert_eq!(stmts.len(), 1);
        assert!(stmts[0].contains("UPDATE t SET x = 1"));
    }

    #[test]
    fn parse_version_strips_zero_padding() {
        assert_eq!(
            parse_version_from_filename("0200__autobo_pagadores.sql"),
            Some(200)
        );
        assert_eq!(parse_version_from_filename("0206__autobo_rls.sql"), Some(206));
        assert_eq!(parse_version_from_filename("readme.md"), None);
    }

    #[test]
    fn supabase_urls_require_tls() {
        let url = normalize_database_url(
            "postgresql://postgres.ref:x@aws-0-sa-east-1.pooler.supabase.com:5432/postgres"
                .into(),
        );
        assert!(url.contains("sslmode=require"));
        let already =
            normalize_database_url("postgresql://u:p@host/db?sslmode=require".into());
        assert_eq!(already.matches("sslmode").count(), 1);
    }

    #[tokio::test]
    #[ignore = "exige DATABASE_URL de um Postgres descartável, sem tabelas do AutoOS"]
    async fn apply_migrations_to_disposable_database() {
        let url = normalize_database_url(
            env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
        );
        let pool = PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await
            .expect("connect");
        run_migrations_manual(&pool).await.expect("migrations");
        run_migrations_manual(&pool).await.expect("migrations idempotentes");
        for tabela in [
            "autobo_migrations",
            "autobo_pagadores",
            "autobo_nfes",
            "autobo_boletos",
        ] {
            assert!(tabela_existe(&pool, tabela).await, "{tabela} ausente");
        }
        let integ = detectar_integracao_autoos(&pool).await;
        assert!(
            !integ.clientes && !integ.equipamentos && !integ.produtos,
            "o banco descartável não pode ser o operacional do AutoOS"
        );
    }

    #[tokio::test]
    #[ignore = "conecta no DATABASE_URL configurado"]
    async fn apply_migrations_to_configured_database() {
        dotenv();
        let url = normalize_database_url(
            env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
        );
        let pool = PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await
            .expect("connect");
        run_migrations_manual(&pool).await.expect("migrations");
        assert!(tabela_existe(&pool, "autobo_boletos").await);
        assert!(coluna_existe(&pool, "autobo_boletos", "nfe_id").await);
        let integ = detectar_integracao_autoos(&pool).await;
        assert!(integ.clientes && integ.equipamentos && integ.produtos);
    }
}
