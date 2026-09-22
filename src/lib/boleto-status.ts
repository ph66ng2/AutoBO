export const STATUS_BOLETO_LABEL: Record<string, string> = {
  RASCUNHO: "Rascunho",
  REGISTRANDO: "Registrando",
  REGISTRADO: "Registrado",
  REGISTRO_INDETERMINADO: "Registro incerto",
  ERRO_PAYLOAD: "Erro no envio",
  AGUARDANDO_RETRY: "Nova tentativa",
  VENCIDO: "Vencido",
  BAIXA_ENVIANDO: "Baixa em envio",
  BAIXA_SOLICITADA: "Baixa solicitada",
  BAIXA_INDETERMINADA: "Baixa incerta",
  PAGO: "Pago",
  CANCELADO: "Cancelado",
};

export function labelStatusBoleto(status: string): string {
  return STATUS_BOLETO_LABEL[status] ?? status;
}

export const GRUPOS_STATUS_BOLETO = [
  { id: "TODOS", label: "Todos", statuses: [] },
  { id: "RASCUNHO", label: "Rascunho", statuses: ["RASCUNHO"] },
  { id: "BANCO", label: "No banco", statuses: ["REGISTRANDO", "REGISTRADO"] },
  {
    id: "ANDAMENTO",
    label: "Em andamento",
    statuses: ["AGUARDANDO_RETRY", "BAIXA_ENVIANDO", "BAIXA_SOLICITADA"],
  },
  {
    id: "ATENCAO",
    label: "Atenção",
    statuses: ["VENCIDO", "ERRO_PAYLOAD", "REGISTRO_INDETERMINADO", "BAIXA_INDETERMINADA"],
  },
  { id: "PAGO", label: "Pago", statuses: ["PAGO"] },
  { id: "CANCELADO", label: "Cancelado", statuses: ["CANCELADO"] },
] as const;

export type GrupoStatusBoleto = (typeof GRUPOS_STATUS_BOLETO)[number]["id"];

export function grupoStatusBoleto(id: string) {
  return GRUPOS_STATUS_BOLETO.find((grupo) => grupo.id === id) ?? GRUPOS_STATUS_BOLETO[0];
}

export function pertenceAoGrupo(status: string, grupoId: string): boolean {
  const grupo = grupoStatusBoleto(grupoId);
  if (grupo.id === "TODOS") return true;
  return (grupo.statuses as readonly string[]).includes(status);
}

export function contarStatus(statuses: string[]): Record<string, number> {
  return statuses.reduce<Record<string, number>>((acc, status) => {
    acc[status] = (acc[status] ?? 0) + 1;
    return acc;
  }, {});
}

export function contarGrupo(statuses: string[], grupoId: string): number {
  if (grupoId === "TODOS") return statuses.length;
  return statuses.filter((status) => pertenceAoGrupo(status, grupoId)).length;
}

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
