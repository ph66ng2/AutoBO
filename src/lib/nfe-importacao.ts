import type { BoletoInput, NFeDados, NFeImportadaDTO } from "../types";

export interface RascunhoNfe {
  chaveAcesso: string;
  boleto: BoletoInput;
}

export function boletoDeNfe(dados: NFeDados, vencimento: string): BoletoInput {
  const documento = dados.destinatario.documento.replace(/\D/g, "");
  const nome = dados.destinatario.nome || dados.destinatario.razao_social || "";
  return {
    origem: "NFE",
    seu_numero: dados.numero_nf,
    valor_nominal: dados.valor_total,
    data_vencimento: vencimento,
    registrar_sicredi: false,
    pagador: {
      tipo_pessoa: documento.length > 11 ? "PJ" : "PF",
      documento,
      nome,
      razao_social: nome,
    },
    nfe: {
      numero_nf: dados.numero_nf,
      serie: dados.serie,
      chave_acesso: dados.chave_acesso,
      data_emissao: dados.data_emissao,
      valor_total: dados.valor_total,
      natureza_operacao: dados.natureza_operacao,
    },
    itens: dados.itens,
  };
}

export function criarMemoriaImportacaoNfe() {
  const porChave = new Map<string, RascunhoNfe>();

  return {
    importar(
      dto: NFeImportadaDTO,
      vencimento: string,
    ): { ok: true; chave: string } | { ok: false; erro: string } {
      if (!dto.sucesso || !dto.dados) {
        return { ok: false, erro: dto.erro || "Falha ao ler a NF-e." };
      }
      const chave = dto.dados.chave_acesso;
      if (porChave.has(chave)) {
        return { ok: false, erro: `NF-e já importada: chave de acesso ${chave}` };
      }
      porChave.set(chave, { chaveAcesso: chave, boleto: boletoDeNfe(dto.dados, vencimento) });
      return { ok: true, chave };
    },
    consultar(chave: string): RascunhoNfe | null {
      return porChave.get(chave) ?? null;
    },
    tamanho(): number {
      return porChave.size;
    },
  };
}
