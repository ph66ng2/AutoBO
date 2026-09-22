import { describe, expect, it } from "vitest";
import { boletoDeNfe } from "./nfe-importacao";
import type { NFeDados } from "../types";

const dados: NFeDados = {
  numero_nf: "123",
  serie: "1",
  chave_acesso: "35123456789012345678901234567890123456789012",
  data_emissao: "2024-01-15",
  valor_total: 20,
  natureza_operacao: "Venda",
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

describe("boletoDeNfe", () => {
  it("monta a revisão financeira sem registrar no Sicredi", () => {
    const boleto = boletoDeNfe(dados, "2026-10-21");
    expect(boleto.origem).toBe("NFE");
    expect(boleto.registrar_sicredi).toBe(false);
    expect(boleto.valor_nominal).toBe(20);
    expect(boleto.nfe?.chave_acesso).toBe(dados.chave_acesso);
    expect(boleto.pagador?.documento).toBe("11222333000181");
  });
});
