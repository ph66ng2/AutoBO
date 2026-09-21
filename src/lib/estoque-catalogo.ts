export const CATEGORIA_OPTIONS = [
  { value: "TODOS", label: "Todas as categorias" },
  { value: "TONER", label: "Toner" },
  { value: "CARTUCHO", label: "Cartucho" },
  { value: "CILINDRO", label: "Cilindro" },
  { value: "FUSOR", label: "Fusor" },
  { value: "ROLO", label: "Rolo" },
  { value: "PEÇA", label: "Peça" },
  { value: "OUTRO", label: "Outro" },
] as const;

export const ORIGEM_OPTIONS = [
  { value: "COMPRA", label: "Compra" },
  { value: "VENDA", label: "Venda" },
  { value: "MANUTENCAO", label: "Manutenção" },
  { value: "AJUSTE", label: "Ajuste" },
  { value: "PERDA", label: "Perda" },
  { value: "DEVOLUCAO", label: "Devolução" },
] as const;
