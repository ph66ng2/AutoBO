export const FILTROS_STATUS_BOLETO = [
  "TODOS",
  "RASCUNHO",
  "REGISTRANDO",
  "REGISTRADO",
  "REGISTRO_INDETERMINADO",
  "ERRO_PAYLOAD",
  "AGUARDANDO_RETRY",
  "VENCIDO",
  "BAIXA_ENVIANDO",
  "BAIXA_SOLICITADA",
  "BAIXA_INDETERMINADA",
  "PAGO",
  "CANCELADO",
] as const;

export type FiltroStatusBoleto = (typeof FILTROS_STATUS_BOLETO)[number];

export function variantStatusBoleto(status: string) {
  if (status === "PAGO") return "success" as const;
  if (
    status === "VENCIDO" ||
    status === "CANCELADO" ||
    status === "ERRO_PAYLOAD" ||
    status === "REGISTRO_INDETERMINADO" ||
    status === "BAIXA_INDETERMINADA"
  ) {
    return "danger" as const;
  }
  if (
    status === "RASCUNHO" ||
    status === "AGUARDANDO_RETRY" ||
    status === "BAIXA_SOLICITADA" ||
    status === "REGISTRANDO" ||
    status === "BAIXA_ENVIANDO"
  ) {
    return "warning" as const;
  }
  return "default" as const;
}
