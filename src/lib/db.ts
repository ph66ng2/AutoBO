import { invoke } from "@tauri-apps/api/core";
import type {
  Boleto,
  BoletoInput,
  ClienteAutoOS,
  DashboardMetricas,
  DocumentoOficinaGerado,
  EquipamentoAutoOS,
  EquipamentoHistoricoEvento,
  EquipamentoImagem,
  GerarBoletoResult,
  IntegracaoAutoOS,
  MovimentacaoEstoque,
  MovimentacaoEstoqueInput,
  NFeImportadaDTO,
  Pagador,
  PagadorInput,
  ProdutoAutoOS,
  ProdutoCadastroInput,
  TipoDocumentoOficina,
  TopDevedor,
  Verificacao,
  VerificacaoAutoOS,
} from "../types";

export async function listarPagadores(): Promise<Pagador[]> {
  return invoke("listar_pagadores");
}

export async function buscarPagador(id: number): Promise<Pagador> {
  return invoke("buscar_pagador", { id });
}

export async function salvarPagador(input: PagadorInput): Promise<Pagador> {
  return invoke("salvar_pagador", { input });
}

export async function bloquearPagador(
  id: number,
  bloquear: boolean,
  motivo?: string,
): Promise<void> {
  return invoke("bloquear_pagador", { id, bloquear, motivo });
}

export async function importarNfe(xmlBase64: string): Promise<NFeImportadaDTO> {
  return invoke("importar_nfe", { xmlBase64 });
}

export async function importarNfeLote(
  xmlsBase64: string[],
): Promise<NFeImportadaDTO[]> {
  return invoke("importar_nfe_lote", { xmlsBase64 });
}

export async function importarDanfePdf(pdfBase64: string): Promise<NFeImportadaDTO> {
  return invoke("importar_danfe_pdf", { pdfBase64 });
}

export async function gerarBoleto(input: BoletoInput): Promise<GerarBoletoResult> {
  return invoke("gerar_boleto", { input });
}

export async function listarBoletos(): Promise<Boleto[]> {
  return invoke("listar_boletos");
}

export async function buscarBoleto(id: number): Promise<unknown> {
  return invoke("buscar_boleto", { id });
}

export async function cancelarBoleto(id: number): Promise<void> {
  return invoke("cancelar_boleto", { id });
}

export async function excluirBoleto(id: number): Promise<void> {
  return invoke("excluir_boleto", { id });
}

export async function retentarRascunho(id: number): Promise<Boleto> {
  return invoke("retentar_rascunho", { id });
}

export async function abrirPdfBoleto(path: string): Promise<void> {
  return invoke("abrir_pdf_boleto", { path });
}

export async function abrirPdfDoBoleto(id: number): Promise<void> {
  return invoke("abrir_pdf_do_boleto", { id });
}

export async function metricasDashboard(): Promise<DashboardMetricas> {
  return invoke("metricas_dashboard");
}

export async function topDevedores(): Promise<TopDevedor[]> {
  return invoke("top_devedores");
}

export async function verificarIntegracaoAutoos(): Promise<IntegracaoAutoOS> {
  return invoke("verificar_integracao_autoos");
}

export async function listarClientesAutoos(busca?: string): Promise<ClienteAutoOS[]> {
  return invoke("listar_clientes_autoos", { busca: busca || null });
}

export async function listarEquipamentosAutoos(filtros?: {
  busca?: string;
  status?: string;
}): Promise<EquipamentoAutoOS[]> {
  return invoke("listar_equipamentos_autoos", {
    busca: filtros?.busca?.trim() ? filtros.busca : null,
    status:
      filtros?.status && filtros.status !== "TODOS" ? filtros.status : null,
  });
}

export async function buscarVerificacaoAutoos(
  equipamentoId: number,
): Promise<VerificacaoAutoOS | null> {
  return invoke("buscar_verificacao_autoos", { equipamentoId });
}

export async function listarImagensEquipamento(
  equipamentoId: number,
): Promise<EquipamentoImagem[]> {
  return invoke("listar_imagens_equipamento", { equipamentoId });
}

/** Contrato do AutoOS usado pelo pdf-service e DocumentosEquipamento. */
export const db = {
  async buscarVerificacao(equipamentoId: number): Promise<Verificacao | null> {
    return buscarVerificacaoAutoos(equipamentoId);
  },
  async listarImagensEquipamento(equipamentoId: number): Promise<EquipamentoImagem[]> {
    try {
      return await listarImagensEquipamento(equipamentoId);
    } catch {
      return [];
    }
  },
  async listarHistoricoEquipamento(
    equipamentoId: number,
  ): Promise<EquipamentoHistoricoEvento[]> {
    void equipamentoId;
    throw new Error("Histórico auditado indisponível no AutoBO");
  },
};

export async function gerarDocumentoOficina(
  equipamentoId: number,
  tipo: TipoDocumentoOficina,
): Promise<DocumentoOficinaGerado> {
  return invoke("gerar_documento_oficina", { equipamentoId, tipo });
}

export async function listarProdutosAutoos(filtros?: {
  busca?: string;
  categoria?: string;
  apenasEstoqueBaixo?: boolean;
}): Promise<ProdutoAutoOS[]> {
  return invoke("listar_produtos_autoos", {
    busca: filtros?.busca?.trim() ? filtros.busca : null,
    categoria:
      filtros?.categoria && filtros.categoria !== "TODOS" ? filtros.categoria : null,
    apenasEstoqueBaixo: filtros?.apenasEstoqueBaixo ?? false,
  });
}

export async function criarProdutoAutoos(
  input: ProdutoCadastroInput,
): Promise<ProdutoAutoOS> {
  return invoke("criar_produto_autoos", { input });
}

export async function atualizarCadastroProdutoAutoos(
  id: number,
  input: ProdutoCadastroInput,
): Promise<ProdutoAutoOS> {
  return invoke("atualizar_cadastro_produto_autoos", { id, input });
}

export async function registrarMovimentacaoEstoque(
  input: MovimentacaoEstoqueInput,
): Promise<ProdutoAutoOS> {
  return invoke("registrar_movimentacao_estoque", { input });
}

export async function listarMovimentacoesProduto(
  produtoId: number,
): Promise<MovimentacaoEstoque[]> {
  return invoke("listar_movimentacoes_produto", { produto_id: produtoId });
}

export async function getConfig(chave: string): Promise<unknown> {
  return invoke("get_config", { chave });
}

export async function setConfig(chave: string, valor: string): Promise<void> {
  return invoke("set_config", { chave, valor });
}

export async function listarConfigSicredi(): Promise<{
  valores: Record<string, unknown>;
  keyring_disponivel: boolean;
}> {
  return invoke("listar_config_sicredi");
}

export async function probeKeyring(): Promise<boolean> {
  return invoke("probe_keyring");
}

export async function testarSicredi(): Promise<boolean> {
  return invoke("testar_sicredi");
}

export async function sincronizarSicrediAgora(): Promise<void> {
  return invoke("sincronizar_sicredi_agora");
}

export async function retentarPdfOficial(id: number): Promise<void> {
  return invoke("retentar_pdf_oficial", { id });
}

export async function testarSmtp(): Promise<boolean> {
  return invoke("testar_smtp");
}

export async function testarWhatsapp(): Promise<boolean> {
  return invoke("testar_whatsapp");
}
