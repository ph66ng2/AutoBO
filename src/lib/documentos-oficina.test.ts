import { describe, expect, it } from "vitest";
import {
  parsePecasOrcamento,
  parseServicosOrcamento,
  totalOrcamento,
} from "./documentos-oficina";

describe("documentos de orçamento", () => {
  it("lê serviços no JSON do AutoOS", () => {
    const servicos = parseServicosOrcamento(
      JSON.stringify([
        { id: "1", descricao: "Troca da cabeça de Impressão", valor: 3850 },
      ]),
    );
    expect(servicos).toEqual([
      { descricao: "Troca da cabeça de Impressão", valor: 3850 },
    ]);
  });

  it("lê peças no camelCase do AutoOS", () => {
    const pecas = parsePecasOrcamento(
      JSON.stringify([
        { nome: "Rolo", quantidade: 2, valorUnitario: 10, valorTotal: 20 },
      ]),
    );
    expect(pecas[0]?.valorTotal).toBe(20);
  });

  it("usa custo_total da verificação quando existe", () => {
    expect(
      totalOrcamento({
        servicos: [{ descricao: "A", valor: 1 }],
        pecas: [],
        custoTotal: 3851,
      }),
    ).toBe(3851);
  });
});
