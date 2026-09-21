import { useEffect, useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { FileText, Trash2 } from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "../components/ui/card";
import { Button } from "../components/ui/button";
import { Badge } from "../components/ui/badge";
import { Input } from "../components/ui/input";
import { abrirPdfDoBoleto, excluirBoleto, listarBoletos } from "../lib/db";
import { formatCurrency, formatDatePtBr } from "../lib/format";
import {
  FILTROS_STATUS_BOLETO,
  variantStatusBoleto,
  type FiltroStatusBoleto,
} from "../lib/boleto-status";
import type { Boleto } from "../types";

export default function Boletos() {
  const [boletos, setBoletos] = useState<Boleto[]>([]);
  const [busca, setBusca] = useState("");
  const [status, setStatus] = useState<FiltroStatusBoleto>("TODOS");
  const [erro, setErro] = useState<string | null>(null);
  const [excluindo, setExcluindo] = useState<number | null>(null);
  const [abrindo, setAbrindo] = useState<number | null>(null);

  async function recarregar() {
    const lista = await listarBoletos();
    setBoletos(lista);
  }

  useEffect(() => {
    recarregar().catch((e) => setErro(String(e)));
  }, []);

  const filtrados = useMemo(() => {
    const q = busca.trim().toLowerCase();
    return boletos.filter((b) => {
      if (status !== "TODOS" && b.status !== status) return false;
      if (!q) return true;
      const blob = [
        b.seu_numero,
        b.pagador_nome,
        b.pagador_documento,
        b.origem,
        b.mensagem,
      ]
        .filter(Boolean)
        .join(" ")
        .toLowerCase();
      return blob.includes(q);
    });
  }, [boletos, busca, status]);

  async function excluir(id: number, numero: string) {
    if (!window.confirm(`Excluir o boleto ${numero}? Dá para importar a nota de novo depois.`)) {
      return;
    }
    setExcluindo(id);
    setErro(null);
    try {
      await excluirBoleto(id);
      await recarregar();
    } catch (e) {
      setErro(String(e));
    } finally {
      setExcluindo(null);
    }
  }

  async function abrirPdf(id: number) {
    setAbrindo(id);
    setErro(null);
    try {
      await abrirPdfDoBoleto(id);
    } catch (e) {
      setErro(String(e));
    } finally {
      setAbrindo(null);
    }
  }

  return (
    <div className="space-y-4">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">Boletos</h1>
          <p className="text-sm text-muted-foreground">
            Boletos gravados no banco da oficina. Abre o PDF, filtra e exclui o que for teste.
          </p>
        </div>
        <Link to="/novo-boleto">
          <Button>Novo boleto</Button>
        </Link>
      </div>

      <div className="flex flex-col gap-3 sm:flex-row sm:items-center">
        <Input
          placeholder="Buscar por número, pagador ou documento"
          value={busca}
          onChange={(e) => setBusca(e.target.value)}
        />
        <div className="flex flex-wrap gap-1">
          {FILTROS_STATUS_BOLETO.map((filtro) => (
            <button
              key={filtro}
              type="button"
              onClick={() => setStatus(filtro)}
              className={`h-8 rounded-md px-2.5 text-xs font-medium ${
                status === filtro
                  ? "bg-primary text-primary-foreground"
                  : "bg-secondary text-secondary-foreground hover:bg-accent hover:text-accent-foreground"
              }`}
              aria-pressed={status === filtro}
            >
              {filtro === "TODOS" ? "Todos" : filtro}
            </button>
          ))}
        </div>
      </div>

      {erro && (
        <p className="rounded-md border border-red-200 bg-red-50 p-3 text-sm text-red-700">{erro}</p>
      )}

      <Card>
        <CardHeader>
          <CardTitle>
            {filtrados.length} {filtrados.length === 1 ? "boleto" : "boletos"}
          </CardTitle>
        </CardHeader>
        <CardContent>
          {boletos.length === 0 ? (
            <p className="text-sm text-muted-foreground">
              Nenhum boleto ainda.{" "}
              <Link to="/novo-boleto" className="text-primary hover:text-primary/80">
                Gerar o primeiro
              </Link>
              .
            </p>
          ) : filtrados.length === 0 ? (
            <p className="text-sm text-muted-foreground">Nenhum boleto com esse filtro.</p>
          ) : (
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b text-left text-muted-foreground">
                  <th className="py-2">Número</th>
                  <th>Pagador</th>
                  <th>Origem</th>
                  <th>Valor</th>
                  <th>Vencimento</th>
                  <th>Status</th>
                  <th className="text-right">Ações</th>
                </tr>
              </thead>
              <tbody>
                {filtrados.map((b) => (
                  <tr key={b.id} className="border-b">
                    <td className="py-2 font-medium tabular-nums">{b.seu_numero}</td>
                    <td>
                      <p>{b.pagador_nome || "—"}</p>
                      {b.pagador_documento ? (
                        <p className="text-xs text-muted-foreground tabular-nums">{b.pagador_documento}</p>
                      ) : null}
                    </td>
                    <td>{b.origem}</td>
                    <td className="tabular-nums">{formatCurrency(b.valor_nominal)}</td>
                    <td className="tabular-nums">{formatDatePtBr(b.data_vencimento)}</td>
                    <td>
                      <Badge variant={variantStatusBoleto(b.status)}>{b.status}</Badge>
                    </td>
                    <td className="text-right">
                      <div className="flex justify-end gap-1">
                        <Button
                          type="button"
                          variant="outline"
                          size="sm"
                          className="gap-1"
                          disabled={abrindo === b.id}
                          aria-label={`Abrir PDF do boleto ${b.seu_numero}`}
                          onClick={() => void abrirPdf(b.id)}
                        >
                          <FileText className="h-3.5 w-3.5" />
                          {abrindo === b.id ? "Abrindo..." : "PDF"}
                        </Button>
                        <Button
                          type="button"
                          variant="destructive"
                          size="sm"
                          className="gap-1"
                          disabled={excluindo === b.id}
                          aria-label={`Excluir boleto ${b.seu_numero}`}
                          onClick={() => void excluir(b.id, b.seu_numero)}
                        >
                          <Trash2 className="h-3.5 w-3.5" />
                          {excluindo === b.id ? "Excluindo..." : "Excluir"}
                        </Button>
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
