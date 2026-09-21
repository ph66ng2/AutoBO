// AutoBO — Tauri 2.x application library entry point.
// Contains the run() function invoked by main.rs.

pub mod commands;
pub mod db;
pub mod jobs;
pub mod services;
pub mod validators;

use db::AppState;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize tracing for structured logging.
/// Writes logs to both stdout and a rolling file in the OS-specific app data directory.
fn init_tracing() {
    let log_dir = directories::ProjectDirs::from("com", "bmitag", "AutoBO")
        .map(|dirs| dirs.data_local_dir().join("logs"))
        .unwrap_or_else(|| std::path::PathBuf::from("./logs"));

    let file_appender = tracing_appender::rolling::daily(&log_dir, "autobo.log");

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt::layer().with_writer(std::io::stdout))
        .with(fmt::layer().with_writer(file_appender))
        .init();

    tracing::info!("Tracing initialized. Log directory: {:?}", log_dir);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_tracing();

    tracing::info!("Starting AutoBO v{}", env!("CARGO_PKG_VERSION"));

    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    let pool = rt.block_on(async {
        let pool = db::init_db().await.expect("Failed to initialize database pool");
        jobs::scheduler::iniciar_scheduler(pool.clone());
        pool
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState { db: pool })
        .invoke_handler(tauri::generate_handler![
            // Pagador commands
            commands::pagador::listar_pagadores,
            commands::pagador::buscar_pagador,
            commands::pagador::buscar_pagador_por_documento,
            commands::pagador::salvar_pagador,
            commands::pagador::bloquear_pagador,
            // NFe commands
            commands::nfe::importar_nfe,
            commands::nfe::importar_nfe_lote,
            commands::nfe::importar_danfe_pdf,
            // Boleto commands
            commands::boleto::gerar_boleto,
            commands::boleto::listar_boletos,
            commands::boleto::buscar_boleto,
            commands::boleto::cancelar_boleto,
            commands::boleto::excluir_boleto,
            commands::boleto::retentar_rascunho,
            commands::boleto::retentar_pdf_oficial,
            commands::boleto::sincronizar_sicredi_agora,
            commands::boleto::abrir_pdf_boleto,
            commands::boleto::abrir_pdf_do_boleto,
            // Dashboard commands
            commands::dashboard::metricas_dashboard,
            commands::dashboard::top_devedores,
            // AutoOS integration
            commands::autoos::verificar_integracao_autoos,
            commands::autoos::listar_clientes_autoos,
            commands::autoos::listar_equipamentos_autoos,
            commands::autoos::buscar_verificacao_autoos,
            commands::autoos::gerar_documento_oficina,
            commands::autoos::listar_imagens_equipamento,
            commands::autoos::listar_produtos_autoos,
            commands::autoos::criar_produto_autoos,
            commands::autoos::atualizar_cadastro_produto_autoos,
            commands::autoos::registrar_movimentacao_estoque,
            commands::autoos::listar_movimentacoes_produto,
            commands::documentos::salvar_orcamento_pdf,
            commands::documentos::salvar_ordem_servico_pdf,
            // Configuracoes commands
            commands::configuracoes::get_config,
            commands::configuracoes::set_config,
            commands::configuracoes::listar_config_sicredi,
            commands::configuracoes::probe_keyring,
            commands::configuracoes::testar_sicredi,
            commands::configuracoes::testar_smtp,
            commands::configuracoes::testar_whatsapp,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
