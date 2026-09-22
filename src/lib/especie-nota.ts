export const ESPECIE_NOTA_COMERCIAL = "DUPLICATA_MERCANTIL_INDICACAO";
export const ESPECIE_NOTA_SERVICO = "DUPLICATA_SERVICO_INDICACAO";

export const OPCOES_TIPO_NOTA = [
  { value: ESPECIE_NOTA_COMERCIAL, label: "Nota comercial" },
  { value: ESPECIE_NOTA_SERVICO, label: "Nota de serviço" },
] as const;

export function especieInicial(serie?: string, natureza?: string): string {
  const texto = `${serie ?? ""} ${natureza ?? ""}`.toLowerCase();
  if (texto.includes("nfs")) return ESPECIE_NOTA_SERVICO;
  return ESPECIE_NOTA_COMERCIAL;
}
