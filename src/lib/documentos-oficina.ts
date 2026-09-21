import type { PecaOrcamento, ServicoOrcamento } from "../types";

export function parseServicosOrcamento(json?: string | null): ServicoOrcamento[] {
  if (!json?.trim()) return [];
  try {
    const parsed = JSON.parse(json) as Array<Record<string, unknown>>;
    if (!Array.isArray(parsed)) return [];
    return parsed
      .map((item) => ({
        descricao: String(item.descricao ?? item.nome ?? "Serviço"),
        valor: Number(item.valor ?? 0),
      }))
      .filter((item) => item.descricao.trim().length > 0);
  } catch {
    return [];
  }
}

export function parsePecasOrcamento(json?: string | null): PecaOrcamento[] {
  if (!json?.trim()) return [];
  try {
    const parsed = JSON.parse(json) as Array<Record<string, unknown>>;
    if (!Array.isArray(parsed)) return [];
    return parsed.map((item) => {
      const quantidade = Number(item.quantidade ?? 1);
      const unitario = Number(item.valorUnitario ?? item.valor_unitario ?? 0);
      const total = Number(item.valorTotal ?? item.valor_total ?? quantidade * unitario);
      return {
        nome: String(item.nome ?? item.descricao ?? "Peça"),
        quantidade,
        valorUnitario: unitario,
        valorTotal: total,
      };
    });
  } catch {
    return [];
  }
}

export function totalOrcamento(input: {
  servicos: ServicoOrcamento[];
  pecas: PecaOrcamento[];
  custoTotal?: number | null;
}): number {
  const somaServicos = input.servicos.reduce((acc, item) => acc + (item.valor || 0), 0);
  const somaPecas = input.pecas.reduce(
    (acc, item) => acc + (item.valorTotal ?? item.valor_total ?? 0),
    0,
  );
  return input.custoTotal && input.custoTotal > 0
    ? input.custoTotal
    : somaServicos + somaPecas;
}
