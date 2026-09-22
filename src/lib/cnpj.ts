import { invoke } from "@tauri-apps/api/core";
import type { ClienteAutoOS } from "../types";

export interface DadosEmpresa {
  nome: string;
  documento: string;
  email: string;
  telefone: string;
  cep: string;
  logradouro: string;
  numero: string;
  bairro: string;
  cidade: string;
  uf: string;
}

export function validarCNPJ(cnpj: string): boolean {
  const digitos = cnpj.replace(/\D/g, "");
  if (digitos.length !== 14) return false;
  if (/^(\d)\1{13}$/.test(digitos)) return false;

  let tamanho = digitos.length - 2;
  let numeros = digitos.substring(0, tamanho);
  const verificadores = digitos.substring(tamanho);
  let soma = 0;
  let pos = tamanho - 7;

  for (let i = tamanho; i >= 1; i--) {
    soma += parseInt(numeros.charAt(tamanho - i), 10) * pos--;
    if (pos < 2) pos = 9;
  }

  let resultado = soma % 11 < 2 ? 0 : 11 - (soma % 11);
  if (resultado !== parseInt(verificadores.charAt(0), 10)) return false;

  tamanho += 1;
  numeros = digitos.substring(0, tamanho);
  soma = 0;
  pos = tamanho - 7;

  for (let i = tamanho; i >= 1; i--) {
    soma += parseInt(numeros.charAt(tamanho - i), 10) * pos--;
    if (pos < 2) pos = 9;
  }

  resultado = soma % 11 < 2 ? 0 : 11 - (soma % 11);
  return resultado === parseInt(verificadores.charAt(1), 10);
}

function texto(value: unknown): string {
  if (typeof value === "string" || typeof value === "number") return String(value).trim();
  return "";
}

export function normalizarConsultaCnpj(payload: unknown, documento: string): DadosEmpresa {
  const data = (payload && typeof payload === "object" ? payload : {}) as Record<string, unknown>;
  const estabelecimento = (data.estabelecimento && typeof data.estabelecimento === "object"
    ? data.estabelecimento
    : {}) as Record<string, unknown>;
  const cidadeEstabelecimento = estabelecimento.cidade as { nome?: unknown } | undefined;
  const estadoEstabelecimento = estabelecimento.estado as { sigla?: unknown } | undefined;
  const telefoneWs = `${texto(estabelecimento.ddd1)}${texto(estabelecimento.telefone1)}`;

  return {
    nome: texto(data.razao_social) || texto(data.nome_fantasia) || texto(estabelecimento.nome_fantasia),
    documento: documento.replace(/\D/g, ""),
    email: texto(data.email) || texto(estabelecimento.email),
    telefone: texto(data.ddd_telefone_1 || data.telefone) || telefoneWs,
    cep: texto(data.cep) || texto(estabelecimento.cep),
    logradouro: texto(data.logradouro || data.endereco) || texto(estabelecimento.logradouro),
    numero: texto(data.numero) || texto(estabelecimento.numero),
    bairro: texto(data.bairro) || texto(estabelecimento.bairro),
    cidade: texto(data.municipio || data.cidade) || texto(cidadeEstabelecimento?.nome),
    uf: (texto(data.uf) || texto(estadoEstabelecimento?.sigla)).toUpperCase(),
  };
}

export function preencherInformados(atual: DadosEmpresa, origem: DadosEmpresa): DadosEmpresa {
  const proximo = { ...atual };
  (Object.keys(origem) as (keyof DadosEmpresa)[]).forEach((campo) => {
    if (origem[campo].trim()) proximo[campo] = origem[campo];
  });
  return proximo;
}

export function preencherVazios(atual: DadosEmpresa, consulta: DadosEmpresa): DadosEmpresa {
  const proximo = { ...atual };
  (Object.keys(consulta) as (keyof DadosEmpresa)[]).forEach((campo) => {
    if (!proximo[campo].trim() && consulta[campo].trim()) {
      proximo[campo] = consulta[campo];
    }
  });
  return proximo;
}

export function dadosDoCliente(cliente: ClienteAutoOS): DadosEmpresa {
  const documento = (cliente.cpf_cnpj || cliente.documento || "").replace(/\D/g, "");
  return {
    nome: cliente.razao_social || cliente.nome || cliente.nome_fantasia || "",
    documento,
    email: cliente.email || "",
    telefone: cliente.telefone || "",
    cep: cliente.cep || "",
    logradouro: cliente.endereco || "",
    numero: cliente.numero || "",
    bairro: cliente.bairro || "",
    cidade: cliente.cidade || "",
    uf: (cliente.uf || "").toUpperCase(),
  };
}

export function mensagemConsultaCnpj(error: unknown): string {
  const bruto = error instanceof Error ? error.message : String(error);
  const match = /^CNPJ_LOOKUP\|(?:invalid|not_found|rate_limited|unavailable|timeout|offline)\|(.+)$/s.exec(bruto);
  return match?.[1] || "Não foi possível consultar o CNPJ agora. Você pode preencher os dados manualmente.";
}

export async function consultarCnpj(cnpj: string): Promise<DadosEmpresa> {
  const payload = await invoke<unknown>("consultar_cnpj", { cnpj });
  return normalizarConsultaCnpj(payload, cnpj);
}
