export {
  STATUS_EQUIPAMENTO,
  STATUS_LABELS,
  type StatusEquipamento,
} from "../lib/status-equipamento";

export interface Pagador {
  id: number;
  tipo_pessoa: string;
  documento: string;
  nome?: string;
  razao_social?: string;
  nome_fantasia?: string;
  telefone?: string;
  email?: string;
  cep?: string;
  logradouro?: string;
  numero?: string;
  complemento?: string;
  bairro?: string;
  cidade?: string;
  uf?: string;
  ativo: boolean;
  bloqueado: boolean;
}

export interface GerarBoletoResult {
  boleto: Boleto;
  pdf_path: string;
  avisos?: string[];
}

export interface Boleto {
  id: number;
  pagador_id: number;
  nfe_id?: number;
  seu_numero: string;
  nosso_numero?: string;
  valor_nominal: number;
  valor_pago?: number;
  data_emissao: string;
  data_vencimento: string;
  data_pagamento?: string;
  tipo_cobranca: string;
  status: string;
  linha_digitavel?: string;
  codigo_barras?: string;
  txid?: string;
  qr_code?: string;
  email_enviado?: boolean;
  whatsapp_enviado?: boolean;
  origem: string;
  pagador_nome?: string;
  pagador_documento?: string;
  mensagem?: string;
  pdf_pendente?: boolean;
  pdf_oficial_path?: string;
  sicredi_seu_numero?: string;
  ultimo_erro_mensagem?: string;
}

export interface DashboardMetricas {
  boletos_gerados: number;
  boletos_pagos: number;
  valor_a_receber: number | null;
  valor_recebido: number | null;
  boletos_vencidos: number;
}

export interface TopDevedor {
  pagador_id: number;
  documento: string;
  nome?: string;
  razao_social?: string;
  total_em_aberto: number | null;
  quantidade_boletos: number;
}

export interface NFeItem {
  descricao: string;
  quantidade: number;
  unidade?: string;
  valor_unitario: number;
  subtotal: number;
}

export interface NFeDestinatario {
  documento: string;
  nome?: string;
  razao_social?: string;
  email?: string;
  telefone?: string;
  cep?: string;
  logradouro?: string;
  numero?: string;
  complemento?: string;
  bairro?: string;
  cidade?: string;
  uf?: string;
}

export interface NFeDados {
  numero_nf: string;
  serie?: string;
  chave_acesso: string;
  data_emissao: string;
  valor_total: number;
  natureza_operacao?: string;
  destinatario: NFeDestinatario;
  itens: NFeItem[];
}

export interface NFeImportadaDTO {
  sucesso: boolean;
  dados?: NFeDados;
  erro?: string;
  avisos?: string[];
}

export interface PagadorInput {
  tipo_pessoa: string;
  documento: string;
  nome?: string;
  razao_social?: string;
  nome_fantasia?: string;
  telefone?: string;
  email?: string;
  cep?: string;
  logradouro?: string;
  numero?: string;
  complemento?: string;
  bairro?: string;
  cidade?: string;
  uf?: string;
}

export interface BoletoInput {
  pagador_id?: number;
  pagador?: PagadorInput;
  nfe_id?: number;
  nfe?: {
    numero_nf: string;
    serie?: string;
    chave_acesso: string;
    data_emissao: string;
    valor_total: number;
    natureza_operacao?: string;
  };
  origem: string;
  seu_numero?: string;
  valor_nominal: number;
  data_vencimento: string;
  tipo_cobranca?: string;
  especie_documento?: string;
  mensagem?: string;
  itens?: NFeItem[];
  registrar_sicredi?: boolean;
}

export interface IntegracaoAutoOS {
  clientes: boolean;
  equipamentos: boolean;
  produtos: boolean;
}

export interface ClienteAutoOS {
  id: number;
  nome?: string;
  tipo_pessoa?: string;
  documento?: string;
  razao_social?: string;
  nome_fantasia?: string;
  cpf_cnpj?: string;
  telefone?: string;
  email?: string;
  cep?: string;
  endereco?: string;
  numero?: string;
  complemento?: string;
  bairro?: string;
  cidade?: string;
  uf?: string;
  ativo?: boolean;
}

export interface EquipamentoAutoOS {
  id: number;
  serial_number: string;
  marca: string;
  modelo: string;
  tipo: string;
  status?: string;
  data_entrada: string;
  cliente_id?: number;
  cliente_nome?: string;
  cliente_documento?: string;
  cliente_telefone?: string;
  cliente_email?: string;
  responsavel_nome?: string;
  responsavel_email?: string;
  responsavel_telefone?: string;
  valor_orcamento?: number;
  valor_final?: number;
  data_pronto?: string;
  data_saida?: string;
  prazo_aprovacao?: string;
  data_aprovacao?: string;
  data_reprovacao?: string;
  data_verificacao?: string;
  defeito_relatado?: string;
  diagnostico?: string;
  acessorios?: string;
  acessorios_outros?: string;
  observacoes?: string;
  patrimonio?: string;
  proprietario?: string;
  criado_em?: string;
  tecnologia?: string;
  conectividade?: string;
  paginas_impressas?: number;
  tem_verificacao?: boolean;
}

/** Alias do AutoOS — o PDF de OS/orçamento usa este contrato. */
export type Equipamento = Omit<EquipamentoAutoOS, "status"> & { status: string };

export const CATEGORIAS_IMAGEM_EQUIPAMENTO = {
  ENTRADA: "ENTRADA",
  SAIDA: "SAIDA",
  VERIFICACAO: "VERIFICACAO",
} as const;

export type EquipamentoImagemCategoria =
  (typeof CATEGORIAS_IMAGEM_EQUIPAMENTO)[keyof typeof CATEGORIAS_IMAGEM_EQUIPAMENTO];

export interface EquipamentoImagem {
  id?: number;
  equipamento_id: number;
  categoria: EquipamentoImagemCategoria;
  filename: string;
  mime_type: string;
  tamanho_bytes: number;
  largura?: number;
  altura?: number;
  ordem: number;
  observacao?: string;
  storage_path: string;
  criado_em?: string;
  atualizado_em?: string;
}

export interface EquipamentoHistoricoEvento {
  tipo: "ETAPA" | "MUDANCA_STATUS" | "CORRECAO_STATUS";
  data: string;
  data_confiavel?: boolean;
  status_anterior?: string;
  status: string;
  motivo: string;
  autor?: string;
}

export interface ServicoOrcamento {
  id?: string;
  descricao: string;
  valor: number;
}

export interface PecaOrcamento {
  id?: string;
  nome: string;
  quantidade: number;
  valorUnitario?: number;
  valor_unitario?: number;
  valorTotal?: number;
  valor_total?: number;
}

export interface VerificacaoAutoOS {
  id: number;
  equipamento_id: number;
  tecnico_nome?: string;
  problema_relatado?: string;
  diagnostico?: string;
  servicos_necessarios?: string;
  pecas_necessarias?: string;
  custo_estimado_mao_obra?: number;
  custo_estimado_pecas?: number;
  custo_total?: number;
  observacoes?: string;
  forma_pagamento_codigo?: string;
  forma_pagamento_detalhe?: string;
  adjusted_at?: string;
}

export type Verificacao = VerificacaoAutoOS;

export interface ServicoNecessario {
  id: string;
  catalogo_id?: number;
  descricao: string;
  valor: number;
}

export interface PecaNecessaria {
  id: string;
  nome: string;
  quantidade: number;
  valorUnitario: number;
  valorTotal: number;
}

export type TipoDocumentoOficina = "ordem_servico" | "orcamento";

export interface DocumentoOficinaGerado {
  tipo: TipoDocumentoOficina;
  filename: string;
  path: string;
}

export interface ProdutoAutoOS {
  id: number;
  codigo: string;
  nome: string;
  descricao?: string;
  categoria: string;
  quantidade_estoque?: number;
  quantidade_minima?: number;
  quantidade_maxima?: number;
  unidade_medida?: string;
  localizacao?: string;
  preco_custo?: number;
  preco_venda?: number;
  margem_lucro?: number;
  ativo?: boolean;
  atualizado_em?: string;
}

export interface ProdutoCadastroInput {
  codigo: string;
  nome: string;
  descricao?: string;
  categoria: string;
  quantidade_inicial?: number;
  quantidade_minima?: number;
  quantidade_maxima?: number;
  unidade_medida?: string;
  localizacao?: string;
  preco_custo: number;
  preco_venda: number;
  marca_original?: string;
}

export interface MovimentacaoEstoqueInput {
  produto_id: number;
  tipo: "ENTRADA" | "SAIDA";
  quantidade: number;
  origem: "COMPRA" | "VENDA" | "MANUTENCAO" | "AJUSTE" | "PERDA" | "DEVOLUCAO" | string;
  referencia?: string;
}

export interface MovimentacaoEstoque {
  id: number;
  produto_id: number;
  tipo: string;
  quantidade: number;
  origem: string;
  referencia?: string;
  data_hora?: string;
}
