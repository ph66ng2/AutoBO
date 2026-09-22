import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { Card, CardContent, CardHeader, CardTitle } from "../components/ui/card";
import { Button } from "../components/ui/button";
import { Badge } from "../components/ui/badge";
import { listarBoletos, metricasDashboard } from "../lib/db";
import { labelStatusBoleto, variantStatusBoleto } from "../lib/boleto-status";
import { formatCurrency, formatDatePtBr } from "../lib/format";
import type { Boleto, DashboardMetricas } from "../types";

const METRICAS_ZERO: DashboardMetricas = {
  boletos_gerados: 0,
  boletos_pagos: 0,
  valor_a_receber: 0,
  valor_recebido: 0,
  boletos_vencidos: 0,
};

export default function Dashboard() {
  const [metricas, setMetricas] = useState<DashboardMetricas>(METRICAS_ZERO);
  const [boletos, setBoletos] = useState<Boleto[]>([]);
  const [erro, setErro] = useState<string | null>(null);

  useEffect(() => {
    Promise.all([metricasDashboard(), listarBoletos()])
      .then(([m, lista]) => {
        setMetricas(m);
        setBoletos(lista);
      })
      .catch((e) => setErro(String(e)));
  }, []);

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">Dashboard</h1>
          <p className="text-sm text-muted-foreground">Resumo. A lista completa está em Boletos.</p>
        </div>
        <div className="flex gap-2">
          <Link to="/boletos">
            <Button variant="outline">Ver boletos</Button>
          </Link>
          <Link to="/novo-boleto">
            <Button>Novo boleto</Button>
          </Link>
        </div>
      </div>

      {erro && (
        <p className="rounded-md border border-red-200 bg-red-50 p-3 text-sm text-red-700">{erro}</p>
      )}

      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Em aberto</CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-2xl font-bold tabular-nums">{metricas.boletos_gerados}</p>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Pagos</CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-2xl font-bold tabular-nums">{metricas.boletos_pagos}</p>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">A receber</CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-2xl font-bold tabular-nums">
              {formatCurrency(metricas.valor_a_receber ?? 0)}
            </p>
          </CardContent>
        </Card>
        <Card className={metricas.boletos_vencidos > 0 ? "border-red-200 bg-red-50/50" : ""}>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Vencidos</CardTitle>
          </CardHeader>
          <CardContent>
            <p className={`text-2xl font-bold tabular-nums ${metricas.boletos_vencidos > 0 ? "text-red-600" : ""}`}>
              {metricas.boletos_vencidos}
            </p>
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader>
          <div className="flex items-center justify-between gap-3">
            <CardTitle>Últimos boletos</CardTitle>
            <Link to="/boletos" className="text-sm text-primary hover:text-primary/80">
              Ver todos
            </Link>
          </div>
        </CardHeader>
        <CardContent>
          {boletos.length === 0 ? (
            <p className="text-sm text-muted-foreground">Nenhum boleto ainda. Importe uma NF-e para começar.</p>
          ) : (
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b text-left text-muted-foreground">
                  <th className="py-2">Número</th>
                  <th>Pagador</th>
                  <th>Valor</th>
                  <th>Vencimento</th>
                  <th>Status</th>
                </tr>
              </thead>
              <tbody>
                {boletos.slice(0, 8).map((b) => (
                  <tr key={b.id} className="border-b">
                    <td className="py-2 tabular-nums">{b.seu_numero}</td>
                    <td>{b.pagador_nome || "—"}</td>
                    <td className="tabular-nums">{formatCurrency(b.valor_nominal)}</td>
                    <td className="tabular-nums">{formatDatePtBr(b.data_vencimento)}</td>
                    <td>
                      <Badge variant={variantStatusBoleto(b.status)}>{labelStatusBoleto(b.status)}</Badge>
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
