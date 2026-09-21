/** Mesmos códigos persistidos pelo AutoOS. A tela nunca mostra o valor cru do banco. */
export const STATUS_EQUIPAMENTO = {
  RECEBIDO: "RECEBIDO",
  EM_VERIFICACAO: "EM_VERIFICACAO",
  VERIFICADO: "VERIFICADO",
  AGUARDANDO_APROVACAO: "AGUARDANDO_APROVACAO",
  APROVADO: "APROVADO",
  REPROVADO: "REPROVADO",
  EM_MANUTENCAO: "EM_MANUTENCAO",
  AGUARDANDO_PECA: "AGUARDANDO_PECA",
  PRONTO: "PRONTO",
  ENTREGUE: "ENTREGUE",
  ORCAMENTO_VENCIDO: "ORCAMENTO_VENCIDO",
  ABANDONADO: "ABANDONADO",
} as const;

export type StatusEquipamento =
  (typeof STATUS_EQUIPAMENTO)[keyof typeof STATUS_EQUIPAMENTO];

export const STATUS_LABELS: Record<StatusEquipamento, string> = {
  RECEBIDO: "Recebido",
  EM_VERIFICACAO: "Em Verificação",
  VERIFICADO: "Verificado",
  AGUARDANDO_APROVACAO: "Aguardando Aprovação",
  APROVADO: "Aprovado",
  REPROVADO: "Reprovado",
  EM_MANUTENCAO: "Em Manutenção",
  AGUARDANDO_PECA: "Aguardando Peça",
  PRONTO: "Pronto",
  ENTREGUE: "Entregue",
  ORCAMENTO_VENCIDO: "Orçamento Vencido",
  ABANDONADO: "Abandonado",
};

/** Pills do AutoOS (`EquipamentosStatusBadge`). */
export const STATUS_COLORS: Record<StatusEquipamento, string> = {
  RECEBIDO: "bg-blue-100 text-blue-800",
  EM_VERIFICACAO: "bg-yellow-100 text-yellow-800",
  VERIFICADO: "bg-purple-100 text-purple-800",
  AGUARDANDO_APROVACAO: "bg-orange-100 text-orange-800",
  APROVADO: "bg-green-100 text-green-800",
  REPROVADO: "bg-red-100 text-red-800",
  EM_MANUTENCAO: "bg-indigo-100 text-indigo-800",
  AGUARDANDO_PECA: "bg-amber-100 text-amber-800",
  PRONTO: "bg-emerald-100 text-emerald-800",
  ENTREGUE: "bg-gray-100 text-gray-800",
  ORCAMENTO_VENCIDO: "bg-rose-100 text-rose-800",
  ABANDONADO: "bg-stone-100 text-stone-800",
};

export const STATUS_BADGE_LABELS: Partial<Record<StatusEquipamento, string>> = {
  RECEBIDO: "Recebido",
  EM_VERIFICACAO: "Em Verificação",
  VERIFICADO: "Verificado",
  AGUARDANDO_APROVACAO: "Aguard. Aprovação",
  APROVADO: "Aprovado",
  REPROVADO: "Reprovado",
  EM_MANUTENCAO: "Em Manutenção",
  AGUARDANDO_PECA: "Aguard. Peça",
  PRONTO: "Pronto",
  ENTREGUE: "Entregue",
  ORCAMENTO_VENCIDO: "Orçam. Vencido",
  ABANDONADO: "Abandonado",
};

export const STATUS_OPTIONS = [
  { value: "TODOS", label: "Todos os Status" },
  ...Object.entries(STATUS_LABELS).map(([value, label]) => ({ value, label })),
];

export const STATUS_COM_ORCAMENTO = [
  "AGUARDANDO_APROVACAO",
  "APROVADO",
  "EM_MANUTENCAO",
  "PRONTO",
  "ENTREGUE",
  "ORCAMENTO_VENCIDO",
] as const;

export function isStatusEquipamento(value: string): value is StatusEquipamento {
  return value in STATUS_LABELS;
}

export function labelStatusEquipamento(status?: string | null): string {
  if (!status) return "—";
  if (isStatusEquipamento(status)) {
    return STATUS_BADGE_LABELS[status] || STATUS_LABELS[status];
  }
  return status;
}

export function corStatusEquipamento(status?: string | null): string {
  if (status && isStatusEquipamento(status)) {
    return STATUS_COLORS[status];
  }
  return "bg-gray-100 text-gray-800";
}

export function deveMostrarOrcamento(input: {
  status?: string | null;
  valor_orcamento?: number | null;
  tem_verificacao?: boolean | null;
}): boolean {
  const valor = input.valor_orcamento ?? 0;
  if (valor > 0) return true;
  if (input.status && STATUS_COM_ORCAMENTO.includes(input.status as (typeof STATUS_COM_ORCAMENTO)[number])) {
    return true;
  }
  return Boolean(input.tem_verificacao);
}

export function numeroOS(equipamentoId: number): string {
  return `OS-${String(equipamentoId).padStart(5, "0")}`;
}
