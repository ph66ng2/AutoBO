import { describe, expect, it } from "vitest";
import {
  contarGrupo,
  labelStatusBoleto,
  pertenceAoGrupo,
} from "./boleto-status";

describe("status de boleto", () => {
  it("mostra um rótulo em português, não o código do banco", () => {
    expect(labelStatusBoleto("REGISTRO_INDETERMINADO")).toBe("Registro incerto");
    expect(labelStatusBoleto("BAIXA_SOLICITADA")).toBe("Baixa solicitada");
    expect(labelStatusBoleto("PAGO")).toBe("Pago");
  });

  it("agrupa registro e baixa sem misturar pago", () => {
    expect(pertenceAoGrupo("REGISTRADO", "BANCO")).toBe(true);
    expect(pertenceAoGrupo("PAGO", "ATENCAO")).toBe(false);
    expect(pertenceAoGrupo("VENCIDO", "ATENCAO")).toBe(true);
    expect(pertenceAoGrupo("CANCELADO", "TODOS")).toBe(true);
  });

  it("conta só os boletos do grupo", () => {
    const lista = ["RASCUNHO", "REGISTRADO", "REGISTRANDO", "PAGO"];
    expect(contarGrupo(lista, "TODOS")).toBe(4);
    expect(contarGrupo(lista, "BANCO")).toBe(2);
    expect(contarGrupo(lista, "ATENCAO")).toBe(0);
  });
});