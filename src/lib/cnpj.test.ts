import { describe, expect, it } from "vitest";
import { dadosDoCliente, normalizarConsultaCnpj, preencherVazios, validarCNPJ } from "./cnpj";

describe("consulta de CNPJ", () => {
  it("aceita um CNPJ válido e recusa sequência repetida", () => {
    expect(validarCNPJ("11.222.333/0001-81")).toBe(true);
    expect(validarCNPJ("11.111.111/1111-11")).toBe(false);
  });

  it("normaliza a resposta da BrasilAPI", () => {
    const dados = normalizarConsultaCnpj(
      {
        razao_social: "Oficina Sintetica LTDA",
        ddd_telefone_1: "7133334444",
        email: "fiscal@oficina.test",
        cep: "40100000",
        logradouro: "Av. Teste",
        numero: "20",
        bairro: "Centro",
        municipio: "Salvador",
        uf: "ba",
      },
      "11222333000181",
    );
    expect(dados.nome).toBe("Oficina Sintetica LTDA");
    expect(dados.uf).toBe("BA");
    expect(dados.logradouro).toBe("Av. Teste");
  });

  it("não sobrescreve campo que o usuário já preencheu", () => {
    const atual = normalizarConsultaCnpj({}, "");
    atual.nome = "Nome digitado";
    atual.logradouro = "";
    const consulta = { ...atual, nome: "Receita Federal", logradouro: "Rua Nova" };
    expect(preencherVazios(atual, consulta)).toMatchObject({
      nome: "Nome digitado",
      logradouro: "Rua Nova",
    });
  });

  it("monta o pagador a partir do cliente do AutoOS", () => {
    expect(
      dadosDoCliente({
        id: 7,
        razao_social: "Cliente Oficina",
        cpf_cnpj: "11.222.333/0001-81",
        endereco: "Rua da Oficina",
        numero: "15",
        cidade: "Salvador",
        uf: "ba",
      }),
    ).toMatchObject({
      nome: "Cliente Oficina",
      documento: "11222333000181",
      logradouro: "Rua da Oficina",
      uf: "BA",
    });
  });
});