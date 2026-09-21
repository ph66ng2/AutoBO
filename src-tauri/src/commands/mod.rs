//! Tauri IPC command stubs and real implementations.
//!
//! Full implementations are added module by module as they are completed.

pub mod autoos;
pub use autoos::{
    atualizar_cadastro_produto_autoos, buscar_verificacao_autoos, criar_produto_autoos,
    gerar_documento_oficina, listar_clientes_autoos, listar_equipamentos_autoos,
    listar_imagens_equipamento, listar_movimentacoes_produto, listar_produtos_autoos,
    registrar_movimentacao_estoque, verificar_integracao_autoos, ProdutoAutoOS,
};

pub mod documentos;
pub use documentos::{salvar_orcamento_pdf, salvar_ordem_servico_pdf};

pub mod boleto;
pub use boleto::{
    abrir_pdf_boleto, abrir_pdf_do_boleto, buscar_boleto, cancelar_boleto, excluir_boleto,
    gerar_boleto, listar_boletos, retentar_pdf_oficial, retentar_rascunho,
    sincronizar_sicredi_agora, BoletoInput, BoletoRow, ItemBoletoRow, ItemInput,
};

pub mod configuracoes;
pub use configuracoes::{
    get_config, listar_config_sicredi, probe_keyring, set_config, testar_sicredi, testar_smtp,
    testar_whatsapp,
};

pub mod nfe;
pub use nfe::{importar_nfe, importar_nfe_lote, NFeImportadaDTO};

pub mod pagador;
pub use pagador::{
    bloquear_pagador, buscar_pagador, buscar_pagador_por_documento,
    listar_pagadores, salvar_pagador, PagadorInput, PagadorRow,
};

pub mod dashboard;
pub use dashboard::{metricas_dashboard, top_devedores, DashboardMetricas, TopDevedor};
