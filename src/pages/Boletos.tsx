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
  GRUPOS_STATUS_BOLETO,
  contarGrupo,
  contarStatus,
  grupoStatusBoleto,
  labelStatusBoleto,
  pertenceAoGrupo,
  variantStatusBoleto,
  type GrupoStatusBoleto,
} from "../lib/boleto-status";
import type { Boleto } from "../types";

export default function Boletos() {
  const [boletos, setBoletos] = useState<Boleto[]>([]);
  const [busca, setBusca] = useState("");
  const [grupo, setGrupo] = useState<GrupoStatusBoleto>("TODOS");
  const [status, setStatus] = useState<string | null>(null);
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
      if (!pertenceAoGrupo(b.status, grupo)) return false;
      if (status && b.status !== status) return false;
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
  }, [boletos, busca, grupo, status]);

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

      <div className="space-y-3">
        <Input
          className="max-w-md"
          placeholder="Buscar por número, pagador ou documento"
          value={busca}
          onChange={(e) => setBusca(e.target.value)}
        />
        <FiltroSituacao
          statuses={boletos.map((b) => b.status)}
          grupo={grupo}
          status={status}
          onGrupo={(id) => {
            setGrupo(id);
            setStatus(null);
          }}
          onStatus={setStatus}
        />
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
                      <Badge variant={variantStatusBoleto(b.status)}>{labelStatusBoleto(b.status)}</Badge>
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

function FiltroSituacao({
  statuses,
  grupo,
  status,
  onGrupo,
  onStatus,
}: {
  statuses: string[];
  grupo: GrupoStatusBoleto;
  status: string | null;
  onGrupo: (id: GrupoStatusBoleto) => void;
  onStatus: (status: string | null) => void;
}) {
  const frequencia = contarStatus(statuses);
  const visiveis = GRUPOS_STATUS_BOLETO.filter(
    (item) => item.id === "TODOS" || item.id === grupo || contarGrupo(statuses, item.id) > 0,
  );
  const detalhe = grupoStatusBoleto(grupo).statuses.filter((item) => (frequencia[item] ?? 0) > 0);

  return (
    <div className="space-y-2">
      <div className="flex flex-wrap gap-1.5" role="group" aria-label="Filtrar por situação">
        {visiveis.map((item) => {
          const ativo = item.id === grupo;
          const quantidade = contarGrupo(statuses, item.id);
          return (
            <button
              key={item.id}
              type="button"
              aria-pressed={ativo}
              onClick={() => onGrupo(item.id)}
              className={classeFiltro(ativo)}
            >
              {item.label}
              <span className={`tabular-nums ${ativo ? "text-primary-foreground/75" : "text-muted-foreground"}`}>
                {quantidade}
              </span>
            </button>
          );
        })}
      </div>
      {detalhe.length > 1 && (
        <div className="flex flex-wrap gap-1.5" role="group" aria-label="Detalhar situação">
          {detalhe.map((item) => (
            <button
              key={item}
              type="button"
              aria-pressed={status === item}
              onClick={() => onStatus(status === item ? null : item)}
              className={classeFiltro(status === item, true)}
            >
              {labelStatusBoleto(item)}
              <span className={`tabular-nums ${status === item ? "text-primary-foreground/75" : "text-muted-foreground"}`}>
                {frequencia[item]}
              </span>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

function classeFiltro(ativo: boolean, menor = false) {
  return [
    "inline-flex items-center gap-1.5 rounded-md font-medium transition-[color,background-color,transform] duration-150 ease-out active:scale-[0.96]",
    menor ? "h-7 px-2 text-[11px]" : "h-8 px-2.5 text-xs",
    ativo
      ? "bg-primary text-primary-foreground"
      : "bg-secondary text-secondary-foreground hover:bg-accent hover:text-accent-foreground",
  ].join(" ");
}
