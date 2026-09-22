import { describe, expect, it } from "vitest";
import { criarMemoriaImportacaoNfe } from "../src/lib/nfe-importacao";
import type { NFeDados, NFeImportadaDTO } from "../src/types";

const chave = "35123456789012345678901234567890123456789012";

const dados: NFeDados = {
  numero_nf: "123",
  serie: "1",
  chave_acesso: chave,
  data_emissao: "2024-01-15",
  valor_total: 20,
  destinatario: {
    documento: "11222333000181",
    razao_social: "Empresa Teste LTDA",
  },
  itens: [
    {
      descricao: "Produto A",
      quantidade: 2,
      unidade: "UN",
      valor_unitario: 10,
      subtotal: 20,
    },
  ],
};

function importada(parcial: Partial<NFeImportadaDTO> = {}): NFeImportadaDTO {
  return { sucesso: true, dados, ...parcial };
}

describe("fluxo NF-e para boleto", () => {
  it("cria um rascunho consultável a partir da nota sintética", () => {
    const memoria = criarMemoriaImportacaoNfe();
    const resultado = memoria.importar(importada(), "2026-10-21");
    expect(resultado).toEqual({ ok: true, chave });
    expect(memoria.consultar(chave)?.boleto.origem).toBe("NFE");
    expect(memoria.consultar(chave)?.boleto.valor_nominal).toBe(20);
  });

  it("rejeita a mesma chave sem criar outro boleto", () => {
    const memoria = criarMemoriaImportacaoNfe();
    memoria.importar(importada(), "2026-10-21");
    const repetido = memoria.importar(importada(), "2026-10-21");
    expect(repetido).toEqual({
      ok: false,
      erro: `NF-e já importada: chave de acesso ${chave}`,
    });
    expect(memoria.tamanho()).toBe(1);
  });

  it("nao grava pagador, nota nem boleto quando a leitura falha", () => {
    const memoria = criarMemoriaImportacaoNfe();
    const falha = memoria.importar(
      { sucesso: false, erro: "Erro ao processar NF-e: XML inválido" },
      "2026-10-21",
    );
    expect(falha.ok).toBe(false);
    expect(memoria.tamanho()).toBe(0);
    expect(memoria.consultar(chave)).toBeNull();
  });
});