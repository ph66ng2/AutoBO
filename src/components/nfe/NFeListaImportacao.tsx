import { useState } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "../ui/card";
import { Button } from "../ui/button";
import { Badge } from "../ui/badge";
import type { NFeImportadaDTO } from "../../types";

interface NFeListaImportacaoProps {
  resultados: NFeImportadaDTO[];
  onGerarSelecionados: (selecionados: NFeImportadaDTO[]) => void;
  onVoltar: () => void;
}

export function NFeListaImportacao({ resultados, onGerarSelecionados, onVoltar }: NFeListaImportacaoProps) {
  const [selecionados, setSelecionados] = useState<Set<number>>(new Set());

  const toggleSelecao = (index: number) => {
    setSelecionados((prev) => {
      const next = new Set(prev);
      if (next.has(index)) {
        next.delete(index);
      } else {
        next.add(index);
      }
      return next;
    });
  };

  const toggleTodos = () => {
    const bemSucedidos = resultados
      .map((r, i) => (r.sucesso ? i : -1))
      .filter((i) => i !== -1);
    if (selecionados.size === bemSucedidos.length) {
      setSelecionados(new Set());
    } else {
      setSelecionados(new Set(bemSucedidos));
    }
  };

  const bemSucedidos = resultados.filter((r) => r.sucesso);
  const falhas = resultados.filter((r) => !r.sucesso);
  const selecionadosList = Array.from(selecionados).map((i) => resultados[i]);

  const formatCurrency = (value: number): string => {
    return new Intl.NumberFormat("pt-BR", {
      style: "currency",
      currency: "BRL",
    }).format(value);
  };

  return (
    <div className="space-y-4">
      <Button variant="outline" size="sm" onClick={onVoltar}>
        ← Voltar
      </Button>

      <Card>
        <CardHeader className="flex flex-row items-center justify-between">
          <CardTitle>Resultado da Importação</CardTitle>
          <div className="flex gap-2">
            <Badge variant="success">{bemSucedidos.length} sucesso</Badge>
            {falhas.length > 0 && (
              <Badge variant="danger">{falhas.length} falha</Badge>
            )}
          </div>
        </CardHeader>
        <CardContent>
          <div className="overflow-hidden rounded-md border">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b bg-muted/60">
                  <th className="p-3 text-left">
                    <input
                      type="checkbox"
                      checked={
                        bemSucedidos.length > 0 &&
                        selecionados.size === bemSucedidos.length
                      }
                      onChange={toggleTodos}
                      className="rounded border-input bg-background"
                    />
                  </th>
                  <th className="p-3 text-left text-muted-foreground font-medium">Status</th>
                  <th className="p-3 text-left text-muted-foreground font-medium">NF-e</th>
                  <th className="p-3 text-left text-muted-foreground font-medium">Valor</th>
                  <th className="p-3 text-left text-muted-foreground font-medium">Destinatário</th>
                </tr>
              </thead>
              <tbody>
                {resultados.map((r, i) => (
                  <tr
                    key={i}
                    className={`border-b ${
                      r.sucesso ? "" : "bg-red-50"
                    }`}
                  >
                    <td className="p-3">
                      {r.sucesso && (
                        <input
                          type="checkbox"
                          checked={selecionados.has(i)}
                          onChange={() => toggleSelecao(i)}
                          className="rounded border-input bg-background"
                        />
                      )}
                    </td>
                    <td className="p-3">
                      {r.sucesso ? (
                        <span className="text-emerald-700">✓</span>
                      ) : (
                        <span className="text-destructive">✗</span>
                      )}
                    </td>
                    <td className="p-3 text-foreground">
                      {r.sucesso && r.dados ? r.dados.numero_nf : "—"}
                    </td>
                    <td className="p-3 text-foreground">
                      {r.sucesso && r.dados
                        ? formatCurrency(r.dados.valor_total)
                        : "—"}
                    </td>
                    <td className="p-3 text-foreground">
                      {r.sucesso && r.dados
                        ? r.dados.destinatario.nome ||
                          r.dados.destinatario.razao_social ||
                          r.dados.destinatario.documento
                        : r.erro || "Erro desconhecido"}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          <div className="mt-4 flex justify-end">
            <Button
              onClick={() => onGerarSelecionados(selecionadosList)}
              disabled={selecionados.size === 0}
            >
              Gerar Boletos Selecionados ({selecionados.size})
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
