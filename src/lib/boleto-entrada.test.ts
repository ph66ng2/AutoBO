import { describe, expect, it } from "vitest";
import { validarEntradaBoleto, type EntradaBoleto } from "./boleto-entrada";

function entrada(parcial: Partial<EntradaBoleto> = {}): EntradaBoleto {
  return {
    nome: "Maria Sintetica",
    documento: "529.982.247-25",
    valor: "150,50",
    vencimento: "2026-10-21",
    logradouro: "Rua das Flores",
    numero: "10",
    cidade: "Salvador",
    uf: "BA",
    cep: "40000-000",
    ...parcial,
  };
}

describe("validarEntradaBoleto", () => {
  it("aceita pagador, valor e vencimento sintéticos", () => {
    expect(validarEntradaBoleto(entrada())).toBeNull();
  });

  it("pede o documento quando ele vem vazio", () => {
    expect(validarEntradaBoleto(entrada({ documento: "..." }))).toBe(
      "Informe o CPF/CNPJ do pagador.",
    );
  });

  it("pede um valor maior que zero", () => {
    expect(validarEntradaBoleto(entrada({ valor: "0" }))).toBe("Informe o valor do boleto.");
    expect(validarEntradaBoleto(entrada({ valor: "" }))).toBe("Informe o valor do boleto.");
  });

  it("pede o vencimento", () => {
    expect(validarEntradaBoleto(entrada({ vencimento: "" }))).toBe("Informe o vencimento.");
  });
});
