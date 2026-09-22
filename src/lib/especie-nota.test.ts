import { describe, expect, it } from "vitest";
import { ESPECIE_NOTA_COMERCIAL, ESPECIE_NOTA_SERVICO, especieInicial } from "./especie-nota";

describe("tipo da nota", () => {
  it("manda duplicata mercantil para nota comercial", () => {
    expect(especieInicial("1", "Venda")).toBe(ESPECIE_NOTA_COMERCIAL);
    expect(especieInicial()).toBe(ESPECIE_NOTA_COMERCIAL);
  });

  it("manda duplicata de serviço quando a origem é NFS-e", () => {
    expect(especieInicial("NFS-e")).toBe(ESPECIE_NOTA_SERVICO);
    expect(especieInicial("1", "NFS-e municipal")).toBe(ESPECIE_NOTA_SERVICO);
  });
});