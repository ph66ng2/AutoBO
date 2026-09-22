import { describe, expect, it } from "vitest";
import { criarMemoriaBoletosManuais, type EntradaBoleto } from "../src/lib/boleto-entrada";

const pagador: EntradaBoleto = {
  nome: "Oficina Sintetica",
  documento: "11222333000181",
  valor: "89.90",
  vencimento: "2026-11-01",
  logradouro: "Av. Teste",
  numero: "20",
  cidade: "Salvador",
  uf: "BA",
  cep: "40100000",
};

describe("fluxo manual de boleto", () => {
  it("cria e consulta um rascunho sem provider", () => {
    const memoria = criarMemoriaBoletosManuais();
    const criado = memoria.criar(pagador, "MAN-SINTETICO-1");
    expect(criado.ok).toBe(true);
    if (!criado.ok) return;
    const achado = memoria.consultar(criado.id);
    expect(achado).toMatchObject({
      seuNumero: "MAN-SINTETICO-1",
      documento: "11222333000181",
      valor: 89.9,
      status: "RASCUNHO",
      origem: "MANUAL",
    });
  });

  it("repete o mesmo documento sem abrir outro rascunho", () => {
    const memoria = criarMemoriaBoletosManuais();
    const primeiro = memoria.criar(pagador, "MAN-SINTETICO-1");
    const segundo = memoria.criar(pagador, "MAN-SINTETICO-1");
    expect(primeiro).toEqual(segundo);
    expect(memoria.tamanho()).toBe(1);
  });

  it("nao grava rascunho quando a validacao falha", () => {
    const memoria = criarMemoriaBoletosManuais();
    const resultado = memoria.criar({ ...pagador, valor: "0" }, "MAN-INVALIDO");
    expect(resultado).toEqual({ ok: false, erro: "Informe o valor do boleto." });
    expect(memoria.tamanho()).toBe(0);
  });
});
