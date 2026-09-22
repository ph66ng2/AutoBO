import { Badge } from "../ui/badge";
import { formatCurrency } from "../../lib/format";
import { labelStatusBoleto, variantStatusBoleto } from "../../lib/boleto-status";
import type { Boleto } from "../../types";

export function BoletoCard({ boleto }: { boleto: Boleto }) {
  return (
    <div className="rounded-lg border bg-card p-4">
      <div className="flex items-center justify-between">
        <p className="font-medium">{boleto.seu_numero}</p>
        <Badge variant={variantStatusBoleto(boleto.status)}>{labelStatusBoleto(boleto.status)}</Badge>
      </div>
      <p className="mt-2 text-sm text-muted-foreground">{formatCurrency(boleto.valor_nominal)}</p>
    </div>
  );
}
