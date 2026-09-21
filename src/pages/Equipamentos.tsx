import { useEffect, useState } from "react";
import { FileText, Filter, Printer, RefreshCw, Search, X } from "lucide-react";
import { Card, CardContent } from "../components/ui/card";
import { Input } from "../components/ui/input";
import { Button } from "../components/ui/button";
import { StatusBadge } from "../components/equipamentos/StatusBadge";
import { DocumentosEquipamento } from "../components/equipamentos/DocumentosEquipamento";
import { listarEquipamentosAutoos } from "../lib/db";
import { formatCurrency, formatDatePtBr } from "../lib/format";
import { STATUS_OPTIONS } from "../lib/status-equipamento";
import type { Equipamento, EquipamentoAutoOS } from "../types";

const selectClass =
  "h-10 w-full rounded-md border border-input bg-background px-3 text-sm text-foreground sm:w-56";

function asEquipamento(eq: EquipamentoAutoOS): Equipamento {
  return { ...eq, status: eq.status ?? "" };
}

export default function Equipamentos() {
  const [busca, setBusca] = useState("");
  const [statusFiltro, setStatusFiltro] = useState("TODOS");
  const [itens, setItens] = useState<EquipamentoAutoOS[]>([]);
  const [carregando, setCarregando] = useState(true);
  const [erro, setErro] = useState<string | null>(null);
  const [selecionado, setSelecionado] = useState<Equipamento | null>(null);

  async function recarregar() {
    setCarregando(true);
    try {
      const lista = await listarEquipamentosAutoos({
        busca,
        status: statusFiltro,
      });
      setItens(lista);
      setErro(null);
    } catch (e) {
      setErro(String(e));
    } finally {
      setCarregando(false);
    }
  }

  useEffect(() => {
    const t = setTimeout(() => {
      void recarregar();
    }, 200);
    return () => clearTimeout(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [busca, statusFiltro]);

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold tracking-tight">OS / Orçamentos</h1>
        <p className="text-muted-foreground">
          Somente leitura — orçamentos, status e documentos da oficina no esquema do AutoOS.
        </p>
      </div>

      <Card>
        <CardContent className="pt-6">
          <div className="flex flex-col gap-3 sm:flex-row">
            <div className="relative flex-1">
              <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
              <Input
                className="pl-9"
                placeholder="Buscar por série, marca, modelo, cliente..."
                value={busca}
                onChange={(e) => setBusca(e.target.value)}
              />
            </div>
            <div className="relative">
              <Filter className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
              <select
                className={`${selectClass} pl-9`}
                value={statusFiltro}
                onChange={(e) => setStatusFiltro(e.target.value)}
              >
                {STATUS_OPTIONS.map((opt) => (
                  <option key={opt.value} value={opt.value}>
                    {opt.label}
                  </option>
                ))}
              </select>
            </div>
            <Button type="button" variant="outline" size="icon" onClick={() => void recarregar()}>
              <RefreshCw className="h-4 w-4" />
            </Button>
          </div>
        </CardContent>
      </Card>

      {erro && (
        <p className="rounded-md border border-red-200 bg-red-50 p-3 text-sm text-red-700">
          {erro}
        </p>
      )}

      <Card>
        <CardContent className="overflow-x-auto pt-6">
          {carregando ? (
            <div className="flex h-32 items-center justify-center">
              <div className="h-8 w-8 animate-spin rounded-full border-b-2 border-primary" />
            </div>
          ) : itens.length === 0 ? (
            <div className="py-12 text-center text-muted-foreground">
              <Printer className="mx-auto mb-4 h-16 w-16 opacity-20" />
              <p className="text-lg font-medium">Nenhum equipamento encontrado</p>
              <p className="text-sm">Tente ajustar a busca ou o status</p>
            </div>
          ) : (
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b text-left text-muted-foreground">
                  <th className="py-2">Nº Série</th>
                  <th>Equipamento</th>
                  <th>Cliente</th>
                  <th>Status</th>
                  <th>Entrada</th>
                  <th className="text-right">Ações</th>
                </tr>
              </thead>
              <tbody>
                {itens.map((eq) => (
                  <tr
                    key={eq.id}
                    className="cursor-pointer border-b transition-colors duration-150 ease-out hover:bg-muted"
                    onClick={() => setSelecionado(asEquipamento(eq))}
                  >
                    <td className="py-3 font-mono text-xs">{eq.serial_number}</td>
                    <td>
                      <span className="font-medium">
                        {eq.marca} {eq.modelo}
                      </span>
                      <span className="block text-xs text-muted-foreground">{eq.tipo}</span>
                    </td>
                    <td>
                      {eq.cliente_nome || "—"}
                      {eq.cliente_telefone ? (
                        <span className="block text-xs text-muted-foreground">{eq.cliente_telefone}</span>
                      ) : null}
                    </td>
                    <td>
                      <div className="flex flex-wrap items-center gap-2">
                        <StatusBadge status={eq.status} />
                        {(eq.status === "AGUARDANDO_APROVACAO" ||
                          eq.status === "ORCAMENTO_VENCIDO") &&
                          eq.valor_orcamento != null &&
                          eq.valor_orcamento > 0 && (
                            <span className="inline-flex items-center rounded-full border border-amber-200 bg-amber-50 px-2 py-0.5 text-xs font-medium text-amber-900">
                              Orçamento: {formatCurrency(eq.valor_orcamento)}
                            </span>
                          )}
                      </div>
                    </td>
                    <td className="text-muted-foreground">{formatDatePtBr(eq.data_entrada)}</td>
                    <td className="text-right">
                      <Button
                        type="button"
                        variant="outline"
                        size="sm"
                        className="h-8 gap-1"
                        onClick={(e) => {
                          e.stopPropagation();
                          setSelecionado(asEquipamento(eq));
                        }}
                      >
                        <FileText className="h-3.5 w-3.5" />
                        Documentos
                      </Button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </CardContent>
      </Card>

      {selecionado && (
        <div
          className="fixed inset-0 z-40 flex items-start justify-center overflow-y-auto bg-black/60 p-4 sm:p-8"
          onClick={() => setSelecionado(null)}
        >
          <div
            className="w-full max-w-3xl rounded-xl border bg-card p-5 shadow-lg"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="mb-4 flex items-start justify-between gap-3">
              <div>
                <p className="font-mono text-xs text-muted-foreground">{selecionado.serial_number}</p>
                <h2 className="text-lg font-semibold">
                  {selecionado.marca} {selecionado.modelo}
                </h2>
                <div className="mt-2 flex flex-wrap items-center gap-2">
                  <span className="text-sm text-muted-foreground">Status atual:</span>
                  <StatusBadge status={selecionado.status} />
                </div>
              </div>
              <Button type="button" variant="outline" size="icon" onClick={() => setSelecionado(null)}>
                <X className="h-4 w-4" />
              </Button>
            </div>

            <div className="mb-4 grid gap-3 sm:grid-cols-2">
              {selecionado.cliente_nome && (
                <div className="rounded-md border p-3 text-sm">
                  <p className="text-xs uppercase tracking-wide text-muted-foreground">Cliente</p>
                  <p className="mt-1 font-medium">{selecionado.cliente_nome}</p>
                  {selecionado.cliente_telefone && <p>Tel: {selecionado.cliente_telefone}</p>}
                  {selecionado.cliente_email && <p>Email: {selecionado.cliente_email}</p>}
                </div>
              )}
              {(selecionado.valor_orcamento != null || selecionado.valor_final != null) && (
                <div className="rounded-md border p-3 text-sm">
                  <p className="text-xs uppercase tracking-wide text-muted-foreground">Orçamento</p>
                  {selecionado.valor_orcamento != null && (
                    <p className="mt-1">
                      Valor: <strong>{formatCurrency(selecionado.valor_orcamento)}</strong>
                    </p>
                  )}
                  {selecionado.prazo_aprovacao && (
                    <p>Prazo: {formatDatePtBr(selecionado.prazo_aprovacao)}</p>
                  )}
                  {selecionado.valor_final != null && (
                    <p>
                      Valor final: <strong>{formatCurrency(selecionado.valor_final)}</strong>
                    </p>
                  )}
                </div>
              )}
            </div>

            <DocumentosEquipamento equipamento={selecionado} />
          </div>
        </div>
      )}
    </div>
  );
}
